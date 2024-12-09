use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

use ahash::HashMap;
use ddsfile::Dds;
use mod_app::asset::AssetManager;
use mod_render::{
    Attributes, Indices, MaterialDescriptor, MaterialPool, MaterialResource, Mesh, MeshPool,
    SamplerPool, TexturePool, TextureViewPool, Vertices, MAX_BONES,
};
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt as _;

use super::{
    AddressMode, FilterMode, Float2, Float3, Float4, Matrix, ModelAssetError, Uint4, ViewDimension,
};

type PoolType = HashMap<String, Arc<Root>>;

/// 로드된 모델의 노드 데이터를 관리하는 풀 객체입니다.
static POOL: OnceLock<FairMutex<PoolType>> = OnceLock::new();

/// 노드 데이터를 관리하는 풀 객체를 가져옵니다.
fn get_pool() -> FairMutexGuard<'static, PoolType> {
    POOL.get_or_init(|| FairMutex::new(HashMap::default()))
        .lock()
}

/// ## Model Hierarchy Pool
/// 로드된 모델의 계층 구조 데이터를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `ModelHierarchyPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct ModelHierarchyPool;

impl ModelHierarchyPool {
    /// 모델 계층 구조 데이터를 로드합니다.  
    /// 이 함수는 항상 파일에서 모델 계층 구조 데이터를 읽어 저장합니다.
    ///
    /// # Errors
    /// 모델 계층 구조 데이터를 로드하는 도중 오류가 발생한 경우 `Error`를 반환합니다.
    ///
    pub fn get_or_init(
        name: &str,
        workspace: &str,
        asset_manager: &AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Arc<Root>, ModelAssetError> {
        let mut pool = get_pool();
        match pool.get(name).cloned() {
            Some(root) => Ok(root),
            None => {
                let root = load_model_root(name, workspace, asset_manager, device, queue)?;
                pool.insert(name.to_string(), root.clone());
                Ok(root)
            }
        }
    }

    /// 풀 객체에 해당 모델 계층 데이터를 제거합니다.  
    /// 풀 객체에 해당 모델 계층 데이터가 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    pub fn remove(name: &str) -> Option<Arc<Root>> {
        get_pool().remove(name)
    }

    /// 풀 객체에 있는 모든 모델 계층 데이터를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}

/// ## Model Hierarchy Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HierarchyBlob {
    pub root: NodeBlob,
    pub num_nodes: u32,
    pub minimum: Float3,
    pub maximum: Float3,
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

/// ## Model Root Node
#[derive(Debug, Clone)]
pub struct Root {
    pub node: Node,
    pub num_nodes: usize,
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
}

/// ## Model Node
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub transform: Matrix,
    pub mesh: Option<Arc<Mesh>>,
    pub skinning: Option<Skinning>,
    pub materials: Vec<Arc<MaterialResource>>,
    pub children: Vec<Node>,
}

/// ## Skinned Mesh Data
#[derive(Debug, Clone)]
pub struct Skinning {
    pub quality: u32,
    pub num_bones: u32,
    pub root_bone: String,
    pub bones: Vec<String>,
    pub bindposes: Vec<[f32; 16]>,
}

/// 모델의 노드 데이터를 로드합니다.
fn load_model_root(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Arc<Root>, ModelAssetError> {
    let path = format!("{}/{}.hierarchy", workspace, name);
    let cached_asset = asset_manager
        .get_or_init(&path)
        .map_err(|e| ModelAssetError::from(e))?;
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: HierarchyBlob =
        serde_json::de::from_reader(reader).map_err(|e| ModelAssetError::from(e))?;

    let node = load_model_node_recursive(workspace, asset_manager, device, queue, blob.root)?;
    let root = Arc::new(Root {
        node,
        num_nodes: blob.num_nodes as usize,
        minimum: blob.minimum.into(),
        maximum: blob.maximum.into(),
    });

    asset_manager.remove(path);
    Ok(root)
}

/// 모델을 구성하는 노드의 계층구조를 구성합니다.
fn load_model_node_recursive(
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut blob: NodeBlob,
) -> Result<Node, ModelAssetError> {
    let name = blob.name.clone();
    let transform = blob.transform.clone();
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
    MeshPool::get_or_init(&blob.name.clone(), move || {
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
) -> Result<Arc<MaterialResource>, ModelAssetError> {
    MaterialPool::get_or_init(&blob.name.clone(), move || {
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
) -> Result<(Arc<wgpu::TextureView>, Arc<wgpu::Sampler>), ModelAssetError> {
    let texture = TexturePool::get_or_init(
        &blob.name.clone(),
        move || -> Result<Arc<wgpu::Texture>, ModelAssetError> {
            let path = format!("{}/{}.dds", workspace, &blob.name);
            let cached_asset = asset_manager
                .get_or_init(&path)
                .map_err(|e| ModelAssetError::from(e))?;

            let dds = Dds::read(Cursor::new(cached_asset.as_bytes()))
                .map_err(|e| ModelAssetError::from(e))?;

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
