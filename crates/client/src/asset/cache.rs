use std::{
    io::{self, Cursor},
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use ahash::{HashMap, RandomState};
use dashmap::DashMap;
use ddsfile::Dds;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{asset::AssetManager, error::AssetLoadError};
use mod_render::{
    Attributes, Indices, MaterialDescriptor, MaterialPool, MaterialResource, Mesh, MeshPool,
    MeshResource, SamplerPool, SkinningDataLayout, TexturePool, TextureViewPool, Vertices,
    MAX_BONES,
};
use wgpu::util::DeviceExt;

use crate::component::{BoneCollection, Child, Parent, Sibling, ToParentTrans, WorldTransform};

use super::blob::{MaterialBlob, MeshBlob, ModelBlob, NodeBlob, SkinningBlob, TextureBlob};

/// 로드된 모델 데이터를 관리하는 풀 객체입니다.
static POOL: OnceLock<DashMap<String, Root, RandomState>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> &'static DashMap<String, Root, RandomState> {
    POOL.get_or_init(|| DashMap::default())
}

/// ## Model Pool
/// 로드된 모델 데이터를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `ModelPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct ModelPool;

impl ModelPool {
    pub fn spawn(
        name: &str,
        workspace: &str,
        asset_manager: &AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<
        (
            HashMap<String, Entity>,
            Vec<(Entity, EntityBuilder)>,
            Entity,
        ),
        ModelSpawnError,
    > {
        let name: String = name.into();
        let root = get_pool().entry(name.clone()).or_insert(load_model_root(
            &name,
            workspace,
            asset_manager,
            device,
            queue,
        )?);

        spawn_model(world, device, queue, &root)
    }
}

/// ## Model Load Error List
#[derive(Debug, thiserror::Error)]
pub enum ModelSpawnError {
    /// 모델을 구성하는 노드를 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("model node not found (NODE:{0})")]
    NodeNotFound(String),

    /// 파일을 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("file not found (PATH:{0})")]
    FileNotFound(PathBuf),

    /// dds 포맷의 텍스처를 읽는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to read texture for the following reason:{0}")]
    TextureError(#[from] ddsfile::Error),

    #[error("failed to parse asset for the following reason:{0}")]
    ParsingFailed(#[from] serde_json::Error),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read file for the following reason:{0}")]
    IOError(#[from] io::Error),
}

/// ## Root Model Node
#[derive(Debug, Clone)]
struct Root {
    root: Node,
}

/// ## Model Node
#[derive(Debug, Clone)]
struct Node {
    name: String,
    transform: ToParentTrans,
    mesh: Option<Arc<Mesh>>,
    skinning: Option<Skinning>,
    materials: Vec<Arc<MaterialResource>>,
    children: Vec<Node>,
}

/// ## Skinned Mesh Data
#[derive(Debug, Clone)]
struct Skinning {
    quality: u32,
    num_bones: u32,
    root_bone: String,
    bones: Vec<String>,
    bindposes: Vec<[f32; 16]>,
}

fn spawn_model(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Root,
) -> Result<
    (
        HashMap<String, Entity>,
        Vec<(Entity, EntityBuilder)>,
        Entity,
    ),
    ModelSpawnError,
> {
    let mut entities = HashMap::default();
    let mut batch_commands = Vec::with_capacity(256);
    let root = spawn_model_node_recursive(
        world,
        device,
        queue,
        &mut entities,
        &mut batch_commands,
        Entity::DANGLING,
        &root.root,
        &[],
    )?;

    Ok((entities, batch_commands, root))
}

fn spawn_model_node_recursive<'a>(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    entities: &mut HashMap<String, Entity>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &'a Node,
    siblings: &'a [Node],
) -> Result<Entity, ModelSpawnError> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();
    builder.add(Parent(parent));
    builder.add(current.transform.clone());
    builder.add(WorldTransform::default());

    if let Some(child) = current.children.first() {
        let child = spawn_model_node_recursive(
            world,
            device,
            queue,
            entities,
            batch_commands,
            entity,
            child,
            &current.children[1..],
        )?;
        builder.add(Child(child));
    }

    if let Some(sibling) = siblings.first() {
        let sibling = spawn_model_node_recursive(
            world,
            device,
            queue,
            entities,
            batch_commands,
            parent,
            sibling,
            &siblings[1..],
        )?;
        builder.add(Sibling(sibling));
    }

    if let Some(mesh) = &current.mesh {
        let resource = MeshResource::uninit(Some(&mesh.name()), device);

        if let Some(skinning) = &current.skinning {
            resource.skinning_uniform.update(
                device,
                queue,
                SkinningDataLayout {
                    quality: skinning.quality,
                    num_bones: skinning.num_bones,
                    ..Default::default()
                },
            );
            resource.bindpose_uniform.update(
                device,
                queue,
                skinning.bindposes.iter().cloned().collect(),
            );

            let root = entities
                .get(&skinning.root_bone)
                .cloned()
                .ok_or(ModelSpawnError::NodeNotFound(skinning.root_bone.clone()))?;
            let mut bones = Vec::with_capacity(skinning.bones.len());
            for name in skinning.bones.iter() {
                bones.push(
                    entities
                        .get(name)
                        .cloned()
                        .ok_or(ModelSpawnError::NodeNotFound(name.clone()))?,
                );
            }
            builder.add(BoneCollection { root, bones });
        }

        builder.add_bundle((mesh.clone(), resource));
    }

    if !current.materials.is_empty() {
        let mut materials = Vec::with_capacity(current.materials.len());
        for resource in current.materials.iter() {
            materials.push(resource.clone());
        }
        builder.add(materials);
    }

    entities.insert(current.name.clone(), entity);
    batch_commands.push((entity, builder));

    Ok(entity)
}

/// 모델을 구성합니다.
fn load_model_root(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Root, ModelSpawnError> {
    let path = format!("{}/{}.json", workspace, name);
    let cached_asset = asset_manager.get_or_init(&path).map_err(|e| match e {
        AssetLoadError::IOError(e) => ModelSpawnError::IOError(e),
        AssetLoadError::PathNotFound(path) => ModelSpawnError::FileNotFound(path),
    })?;
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: ModelBlob =
        serde_json::de::from_reader(reader).map_err(|e| ModelSpawnError::from(e))?;

    let root = load_model_node_recursive(workspace, asset_manager, device, queue, blob.root)?;
    asset_manager.remove(path);
    Ok(Root { root })
}

/// 모델을 구성하는 노드의 계층구조를 구성합니다.
fn load_model_node_recursive(
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut blob: NodeBlob,
) -> Result<Node, ModelSpawnError> {
    let name = blob.name.clone();
    let transform = ToParentTrans(blob.transform.into());
    let (skinning, mesh) = match blob.mesh.take() {
        Some(mut blob) => match blob.skinning.take() {
            Some(SkinningBlob {
                quality,
                root_bone,
                bones,
                bindposes,
            }) => (
                Some(Skinning {
                    quality: quality.min(4),
                    num_bones: bones.len().min(MAX_BONES) as u32,
                    root_bone,
                    bones,
                    bindposes: bindposes.into_iter().map(|m| m.into()).collect(),
                }),
                Some(create_mesh(device, queue, blob)),
            ),
            None => (None, Some(create_mesh(device, queue, blob))),
        },
        None => (None, None),
    };

    let mut materials = Vec::with_capacity(blob.materials.len());
    for blob in blob.materials {
        materials.push(create_material(
            workspace,
            asset_manager,
            device,
            queue,
            blob,
        )?);
    }

    let mut children = Vec::with_capacity(blob.children.len());
    for blob in blob.children {
        children.push(load_model_node_recursive(
            workspace,
            asset_manager,
            device,
            queue,
            blob,
        )?);
    }

    Ok(Node {
        name,
        transform,
        mesh,
        skinning,
        materials,
        children,
    })
}

/// 메쉬를 생성합니다. 풀 객체에 메쉬가 없는 경우 풀 객체에 추가합니다.
fn create_mesh(device: &wgpu::Device, queue: &wgpu::Queue, mut blob: MeshBlob) -> Arc<Mesh> {
    MeshPool::get_or_init(blob.name.clone(), move || {
        let vertices: Vec<[f32; 3]> = blob.vertices.iter().cloned().map(|v| v.into()).collect();
        let vertices = Vertices(vertices);
        let mut mesh = Mesh::new(&blob.name, device, queue, vertices);
        blob.vertices.clear();

        if !blob.colors.is_empty() {
            let attributes: Vec<[f32; 4]> = blob.colors.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Color(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.colors.clear();
        }

        if !blob.normals.is_empty() {
            // 정점의 노멀 속성 추가
            let attributes: Vec<[f32; 3]> =
                blob.normals.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Normal(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.normals.clear();
        }

        if !blob.tangents.is_empty() {
            // 정점의 탄젠트 공간 노멀 속성 추가
            let attributes: Vec<[f32; 3]> =
                blob.tangents.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Tangent(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.tangents.clear();
        }

        if !blob.texcoords0.is_empty() {
            // 정점의 0번 텍스처 좌표 속성 추가
            let attributes: Vec<[f32; 2]> =
                blob.texcoords0.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Texcoord0(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.texcoords0.clear();
        }

        if !blob.texcoords1.is_empty() {
            // 정점의 1번 텍스처 좌표 속성 추가
            let attributes: Vec<[f32; 2]> =
                blob.texcoords1.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Texcoord1(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.texcoords1.clear();
        }

        if !blob.texcoords2.is_empty() {
            // 정점의 2번 텍스처 좌표 속성 추가
            let attributes: Vec<[f32; 2]> =
                blob.texcoords2.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Texcoord2(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.texcoords2.clear();
        }

        if !blob.texcoords3.is_empty() {
            // 정점의 3번 텍스처 좌표 속성 추가
            let attributes: Vec<[f32; 2]> =
                blob.texcoords3.iter().cloned().map(|v| v.into()).collect();
            let attributes = Attributes::Texcoord3(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.texcoords3.clear();
        }

        if !blob.bone_indices.is_empty() {
            // 정점의 뼈 번호 속성 추가
            let attributes: Vec<[u32; 4]> = blob
                .bone_indices
                .iter()
                .cloned()
                .map(|v| v.into())
                .collect();
            let attributes = Attributes::BoneIndex(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.bone_indices.clear();
        }

        if !blob.bone_weights.is_empty() {
            // 정점의 뼈 가중치 속성 추가
            let attributes: Vec<[f32; 4]> = blob
                .bone_weights
                .iter()
                .cloned()
                .map(|v| v.into())
                .collect();
            let attributes = Attributes::BoneWeight(attributes);
            mesh.add_attribute(device, queue, attributes);
            blob.bone_weights.clear();
        }

        for submesh in blob.submeshes.iter() {
            // 하위 메쉬 집합을 추가합니다.
            let indices = Indices::U32(submesh.clone());
            mesh.add_submesh(device, queue, indices);
        }
        blob.submeshes.clear();

        Arc::new(mesh)
    })
}

/// 재질을 생성합니다. 풀 객체에 재질이 없는 경우 풀 객체에 추가합니다.
fn create_material(
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    blob: MaterialBlob,
) -> Result<Arc<MaterialResource>, ModelSpawnError> {
    MaterialPool::get_or_init(blob.name.clone(), move || {
        let mut desc = MaterialDescriptor::new(&blob.name, device, queue);

        desc.layout.glossiness = blob.glossiness.unwrap_or(0.5);
        desc.layout.smoothness = blob.smoothness.unwrap_or(0.5);
        desc.layout.metallic = blob.metallic.unwrap_or(0.2);
        desc.layout.bump_scale = blob.bump_scale.unwrap_or(0.0);
        desc.layout.parallax = blob.parallax.unwrap_or(0.0);
        desc.layout.strength = blob.strength.unwrap_or(0.0);
        desc.layout.albedo = blob.albedo.map(|v| v.into()).unwrap_or([0.0; 4]);
        desc.layout.specular = blob.specular.map(|v| v.into()).unwrap_or([0.0; 4]);
        desc.layout.emissive = blob.emissive.map(|v| v.into()).unwrap_or([0.0; 4]);

        if let Some(texture_blob) = blob.albedo_map {
            let (texture_view, sampler) =
                load_dds_texture(&workspace, asset_manager, device, queue, texture_blob)?;
            desc.albedo_map = texture_view;
            desc.albedo_sampler = sampler;
        }

        if let Some(texture_blob) = blob.specular_map {
            let (texture_view, sampler) =
                load_dds_texture(&workspace, asset_manager, device, queue, texture_blob)?;
            desc.specular_map = texture_view;
            desc.specular_sampler = sampler;
        }

        if let Some(texture_blob) = blob.emissive_map {
            let (texture_view, sampler) =
                load_dds_texture(&workspace, asset_manager, device, queue, texture_blob)?;
            desc.emissive_map = texture_view;
            desc.emissive_sampler = sampler;
        }

        if let Some(texture_blob) = blob.normal_map {
            let (texture_view, sampler) =
                load_dds_texture(&workspace, asset_manager, device, queue, texture_blob)?;
            desc.normal_map = texture_view;
            desc.normal_sampler = sampler;
        }

        if let Some(texture_blob) = blob.parallax_map {
            let (texture_view, sampler) =
                load_dds_texture(&workspace, asset_manager, device, queue, texture_blob)?;
            desc.parallax_map = texture_view;
            desc.parallax_sampler = sampler;
        }

        if let Some(texture_blob) = blob.occlusion_map {
            let (texture_view, sampler) =
                load_dds_texture(&workspace, asset_manager, device, queue, texture_blob)?;
            desc.occlusion_map = texture_view;
            desc.occlusion_sampler = sampler;
        }

        let resource = MaterialResource::new(device, queue, &desc);
        Ok(Arc::new(resource))
    })
}

/// 텍스처를 생성합니다. 풀 객체에 텍스처가 없는 경우 풀 객체에 추가합니다.
fn load_dds_texture(
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    blob: TextureBlob,
) -> Result<(Arc<wgpu::TextureView>, Arc<wgpu::Sampler>), ModelSpawnError> {
    let texture = TexturePool::get_or_init(
        blob.name.clone(),
        move || -> Result<Arc<wgpu::Texture>, ModelSpawnError> {
            let path = format!("{}/{}.dds", workspace, &blob.name);
            let cached_asset = asset_manager.get_or_init(&path).map_err(|e| match e {
                AssetLoadError::IOError(e) => ModelSpawnError::IOError(e),
                AssetLoadError::PathNotFound(path) => ModelSpawnError::FileNotFound(path),
            })?;

            let dds = Dds::read(Cursor::new(cached_asset.as_bytes()))
                .map_err(|e| ModelSpawnError::from(e))?;

            let texture = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", &blob.name)),
                    size: wgpu::Extent3d {
                        width: dds.get_width(),
                        height: dds.get_height(),
                        depth_or_array_layers: dds.get_num_array_layers(),
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bc7RgbaUnorm,
                    mip_level_count: dds.get_num_mipmap_levels(),
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &dds.data,
            );

            asset_manager.remove(path);
            Ok(Arc::new(texture))
        },
    )?;

    let texture_view = TextureViewPool::get_or_init(
        &texture,
        &wgpu::TextureViewDescriptor {
            dimension: Some(blob.dimension.into()),
            ..Default::default()
        },
    );

    let sampler = SamplerPool::get_or_init(
        device,
        &wgpu::SamplerDescriptor {
            address_mode_u: blob.address_u.into(),
            address_mode_v: blob.address_v.into(),
            address_mode_w: blob.address_w.into(),
            mag_filter: blob.filter_mode.into(),
            min_filter: blob.filter_mode.into(),
            mipmap_filter: blob.filter_mode.into(),
            ..Default::default()
        },
    );

    Ok((texture_view, sampler))
}
