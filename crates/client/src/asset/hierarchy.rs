use std::{
    io::{self, Cursor},
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use ahash::{HashMap, RandomState};
use dashmap::DashMap;
use ddsfile::Dds;
use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_render::{
    Attributes, Indices, MaterialDescriptor, MaterialPool, MaterialResource, Mesh, MeshPool,
    MeshResource, SamplerPool, SkinningDataLayout, TexturePool, TextureViewPool, Vertices,
    MAX_BONES,
};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt as _;

use crate::component::{BoneCollection, Child, Parent, Sibling, ToParentTrans, WorldTransform};

use super::{AddressMode, FilterMode, Float2, Float3, Float4, Matrix, Uint4, ViewDimension};

/// 로드된 모델의 노드 데이터를 관리하는 풀 객체입니다.
static POOL: OnceLock<DashMap<String, Node, RandomState>> = OnceLock::new();

/// 노드 데이터를 관리하는 풀 객체를 가져옵니다.
fn get_pool() -> &'static DashMap<String, Node, RandomState> {
    POOL.get_or_init(|| DashMap::default())
}

/// ## Model Hierarchy Pool
/// 로드된 모델의 계층 구조 데이터를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `ModelHierarchyPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct ModelHierarchyPool;

impl ModelHierarchyPool {
    /// 모델 계층 구조 데이터를 읽어 모델을 구성하는 `Entity`를 생성합니다.  
    /// 풀 객체에 모델 계층 구조 데이터가 존재하지 않는 경우 파일에서 로드합니다.  
    ///
    /// # Errors
    /// 모델 계층 구조 데이터를 로드하는 도중 오류가 발생하거나, `Entity`를 생성하는 도중 오류가 발생한 경우 `Error`를 반환합니다.
    ///
    pub fn spawn(
        name: &str,
        workspace: &str,
        asset_manager: &AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<
        (
            Entity,
            HashMap<String, Entity>,
            Vec<(Entity, EntityBuilder)>,
        ),
        Error,
    > {
        let root = get_pool()
            .entry(name.to_string())
            .or_insert(load_model_root(
                &name,
                workspace,
                asset_manager,
                device,
                queue,
            )?);

        spawn_model(world, device, queue, &root)
    }

    /// 풀 객체에 해당 모델 계층 데이터를 제거합니다.  
    /// 풀 객체에 해당 모델 계층 데이터가 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    pub fn remove(name: &str) -> Option<Node> {
        get_pool().remove(name).map(|(_, root)| root)
    }

    /// 풀 객체에 있는 모든 모델 계층 데이터를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}

/// ## Model Load Error List
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 모델을 구성하는 노드를 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("model node not found (NODE:{0})")]
    NodeNotFound(String),

    /// dds 포맷의 텍스처를 읽는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to read texture for the following reason:{0}")]
    TextureError(#[from] ddsfile::Error),

    /// 에셋 파일을 구문 분석하는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to parse asset for the following reason:{0}")]
    ParsingFailed(#[from] serde_json::Error),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read asset for the following reason:{0}")]
    IOError(#[from] io::Error),
}

/// ## Node Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeBlob {
    pub name: String,
    pub transform: Matrix,
    pub mesh: Option<MeshBlob>,
    pub materials: Vec<MaterialBlob>,
    pub children: Vec<NodeBlob>,
}

/// ## Mesh Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshBlob {
    pub name: String,
    pub minimum: Float3,
    pub maximum: Float3,
    pub vertices: Vec<Float3>,
    pub colors: Vec<Float4>,
    pub normals: Vec<Float3>,
    pub tangents: Vec<Float3>,
    pub texcoords0: Vec<Float2>,
    pub texcoords1: Vec<Float2>,
    pub texcoords2: Vec<Float2>,
    pub texcoords3: Vec<Float2>,
    pub bone_indices: Vec<Uint4>,
    pub bone_weights: Vec<Float4>,
    pub submeshes: Vec<Vec<u32>>,
    pub skinning: Option<SkinningBlob>,
}

/// ## Material Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialBlob {
    pub name: String,
    pub glossiness: Option<f32>,
    pub smoothness: Option<f32>,
    pub metallic: Option<f32>,
    pub bump_scale: Option<f32>,
    pub parallax: Option<f32>,
    pub strength: Option<f32>,
    pub albedo: Option<Float4>,
    pub specular: Option<Float4>,
    pub emissive: Option<Float4>,
    pub albedo_map: Option<TextureBlob>,
    pub specular_map: Option<TextureBlob>,
    pub emissive_map: Option<TextureBlob>,
    pub normal_map: Option<TextureBlob>,
    pub parallax_map: Option<TextureBlob>,
    pub occlusion_map: Option<TextureBlob>,
}

/// ## Texture View Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextureBlob {
    pub name: String, // Texture는 다른 파일에 저장됨.
    pub dimension: ViewDimension,
    pub address_u: AddressMode,
    pub address_v: AddressMode,
    pub address_w: AddressMode,
    pub filter_mode: FilterMode,
}

/// ## Skinning Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinningBlob {
    pub quality: u32,
    pub root_bone: String,
    pub bones: Vec<String>,
    pub bindposes: Vec<Matrix>,
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

/// 모델을 구성하는 노드들을 생성합니다.
fn spawn_model(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Node,
) -> Result<
    (
        Entity,
        HashMap<String, Entity>,
        Vec<(Entity, EntityBuilder)>,
    ),
    Error,
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
        root,
        &[],
    )?;

    Ok((root, entities, batch_commands))
}

/// 모델을 구성하는 노드의 계층 구조를 생성합니다.
fn spawn_model_node_recursive<'a>(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    entities: &mut HashMap<String, Entity>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &'a Node,
    siblings: &'a [Node],
) -> Result<Entity, Error> {
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
                .ok_or(Error::NodeNotFound(skinning.root_bone.clone()))?;
            let mut bones = Vec::with_capacity(skinning.bones.len());
            for name in skinning.bones.iter() {
                bones.push(
                    entities
                        .get(name)
                        .cloned()
                        .ok_or(Error::NodeNotFound(name.clone()))?,
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

/// 모델의 노드 데이터를 로드합니다.
fn load_model_root(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Node, Error> {
    let path = format!("{}/{}.hierarchy", workspace, name);
    let cached_asset = asset_manager
        .get_or_init(&path)
        .map_err(|e| Error::from(e))?;
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: NodeBlob = serde_json::de::from_reader(reader).map_err(|e| Error::from(e))?;

    let node = load_model_node_recursive(workspace, asset_manager, device, queue, blob)?;

    asset_manager.remove(path);
    Ok(node)
}

/// 모델을 구성하는 노드의 계층구조를 구성합니다.
fn load_model_node_recursive(
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut blob: NodeBlob,
) -> Result<Node, Error> {
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
) -> Result<Arc<MaterialResource>, Error> {
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
) -> Result<(Arc<wgpu::TextureView>, Arc<wgpu::Sampler>), Error> {
    let texture = TexturePool::get_or_init(
        blob.name.clone(),
        move || -> Result<Arc<wgpu::Texture>, Error> {
            let path = format!("{}/{}.dds", workspace, &blob.name);
            let cached_asset = asset_manager
                .get_or_init(&path)
                .map_err(|e| Error::from(e))?;

            let dds =
                Dds::read(Cursor::new(cached_asset.as_bytes())).map_err(|e| Error::from(e))?;

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
