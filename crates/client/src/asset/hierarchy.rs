use std::{
    io::Cursor,
    path::Path,
    sync::{Arc, OnceLock},
};

use ahash::HashMap;
use mod_app::asset::AssetManager;
use mod_network::components::{
    HierarchyNode, MaterialData, MeshData, ModelHierarchyData, SkinningData,
};
use mod_render::{
    Attributes, Indices, MaterialDescriptor, MaterialPool, MaterialResource, Mesh, MeshPool,
    Vertices, MAX_BONES,
};
use parking_lot::{FairMutex, FairMutexGuard};

use super::{AssetError, SamplerPool, TextureDataPool, TexturePool, TextureViewPool};

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
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) -> Result<Arc<Root>, AssetError> {
        let mut pool = get_pool();
        match pool.get(name).cloned() {
            Some(root) => Ok(root),
            None => {
                let root = load_model_root(
                    name,
                    workspace,
                    asset_manager,
                    device,
                    queue,
                    encoder,
                    staging_buffers,
                )?;
                pool.insert(name.to_string(), root.clone());
                Ok(root)
            }
        }
    }

    /// 풀 객체에 해당 모델 계층 데이터를 제거합니다.  
    /// 풀 객체에 해당 모델 계층 데이터가 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    #[allow(dead_code)]
    pub fn remove(name: &str) -> Option<Arc<Root>> {
        get_pool().remove(name)
    }

    /// 풀 객체에 있는 모든 모델 계층 데이터를 제거합니다.
    #[allow(dead_code)]
    pub fn clear() {
        get_pool().clear()
    }
}

/// ## Model Root Node
#[derive(Debug, Clone)]
pub struct Root {
    pub node: Node,
    pub num_nodes: usize,
}

/// ## Model Node
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub transform: glam::Mat4,
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
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> Result<Arc<Root>, AssetError> {
    let path = format!("{}/{}.hierarchy", workspace, name);
    log::debug!("load model asset (PATH:{})", &path);
    let cached_asset = asset_manager.get_or_init(&path).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &path);
        AssetError::from(e)
    })?;

    log::debug!("parse model asset (PATH:{})", &path);
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: ModelHierarchyData = serde_json::de::from_reader(reader).map_err(|e| {
        log::error!("{} (PATH:{})", &e, &path);
        AssetError::from(e)
    })?;

    let node = load_model_node_recursive(
        workspace,
        asset_manager,
        device,
        queue,
        encoder,
        staging_buffers,
        blob.root,
    )?;
    let root = Arc::new(Root {
        node,
        num_nodes: blob.num_nodes as usize,
    });

    log::debug!("cleanup model asset cache (PATH:{})", &path);
    asset_manager.remove(path);

    Ok(root)
}

/// 모델을 구성하는 노드의 계층구조를 구성합니다.
fn load_model_node_recursive(
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    blob: HierarchyNode,
) -> Result<Node, AssetError> {
    let name = blob.name.clone();
    let transform = blob.transform.into_mat4();
    let (skinning, mesh) = match blob.mesh.as_ref() {
        Some(filename) => {
            let path = format!("{}/{}.mesh", workspace, &filename);
            let cached = asset_manager.get_or_init(&path).map_err(|e| {
                log::error!("{} (PATH:{})", &e, &path);
                AssetError::from(e)
            })?;
            let mut blob: MeshData =
                serde_json::de::from_slice(cached.as_bytes()).map_err(|e| {
                    log::error!("{} (PATH:{})", &e, &path);
                    AssetError::from(e)
                })?;

            match blob.skinning.take() {
                Some(SkinningData {
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
            }
        }
        None => (None, None),
    };

    let mut materials = Vec::with_capacity(blob.materials.len());
    for filename in blob.materials.iter() {
        let path = format!("{}/{}.material", &workspace, &filename);
        let cached = asset_manager.get_or_init(&path).map_err(|e| {
            log::error!("{} (PATH:{})", &e, &path);
            AssetError::from(e)
        })?;
        let blob: MaterialData = serde_json::de::from_slice(cached.as_bytes()).map_err(|e| {
            log::error!("{} (PATH:{})", &e, &path);
            AssetError::from(e)
        })?;

        materials.push(create_material(
            workspace,
            asset_manager,
            device,
            queue,
            encoder,
            staging_buffers,
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
            encoder,
            staging_buffers,
            blob,
        )?);
    }

    // 캐싱된 에셋을 정리합니다.
    if let Some(filename) = blob.mesh.as_ref() {
        let path = format!("{}/{}.mesh", workspace, &filename);
        asset_manager.remove(path);
    }
    for filename in blob.materials.iter() {
        let path = format!("{}/{}.material", &workspace, &filename);
        asset_manager.remove(path);
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
fn create_mesh(device: &wgpu::Device, queue: &wgpu::Queue, mut blob: MeshData) -> Arc<Mesh> {
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
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    blob: MaterialData,
) -> Result<Arc<MaterialResource>, AssetError> {
    MaterialPool::get_or_init(&blob.name.clone(), move || {
        let mut desc = MaterialDescriptor::new(&blob.name);
        desc.layout.glossiness = blob.glossiness.unwrap_or(0.5);
        desc.layout.smoothness = blob.smoothness.unwrap_or(0.5);
        desc.layout.metallic = blob.metallic.unwrap_or(0.2);
        desc.layout.bump_scale = blob.bump_scale.unwrap_or(0.0);
        desc.layout.parallax = blob.parallax.unwrap_or(0.0);
        desc.layout.strength = blob.strength.unwrap_or(0.0);

        let mut path = asset_manager.get_root_dir().to_path_buf();
        path.push(workspace);
        println!("{}", workspace);
        if let Some(texture_blob) = blob.albedo_map {
            let (view, sampler) =
                get_or_init_texture(path, texture_blob.name, device, encoder, staging_buffers)?;
            desc.with_albedo_texture(view, sampler);
        } else if let Some(color) = blob.albedo {
            desc.with_albedo_color(color.into());
        }

        if let Some(texture_blob) = blob.specular_map {
            let (view, sampler) = get_or_init_texture(
                &workspace,
                texture_blob.name,
                device,
                encoder,
                staging_buffers,
            )?;
            desc.with_specular_texture(view, sampler);
        } else if let Some(color) = blob.specular {
            desc.with_specular_color(color.into());
        }

        if let Some(texture_blob) = blob.emissive_map {
            let (view, sampler) = get_or_init_texture(
                &workspace,
                texture_blob.name,
                device,
                encoder,
                staging_buffers,
            )?;
            desc.with_emissive_texture(view, sampler);
        } else if let Some(color) = blob.emissive {
            desc.with_emissive_color(color.into());
        }

        if let Some(texture_blob) = blob.normal_map {
            let (view, sampler) = get_or_init_texture(
                &workspace,
                texture_blob.name,
                device,
                encoder,
                staging_buffers,
            )?;
            desc.with_normal_texture(view, sampler);
        }

        if let Some(texture_blob) = blob.height_map {
            let (view, sampler) = get_or_init_texture(
                &workspace,
                texture_blob.name,
                device,
                encoder,
                staging_buffers,
            )?;
            desc.with_height_texture(view, sampler);
        }

        if let Some(texture_blob) = blob.occlusion_map {
            let (view, sampler) = get_or_init_texture(
                &workspace,
                texture_blob.name,
                device,
                encoder,
                staging_buffers,
            )?;
            desc.with_occlusion_texture(view, sampler);
        }

        let resource = MaterialResource::new(device, queue, &desc);
        Ok(Arc::new(resource))
    })
}

/// 이미 생성된 텍스처를 가져오거나, 텍스처를 파일로부터 생성합니다.
fn get_or_init_texture<Dir, Uri>(
    workspace: Dir,
    uri: Uri,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> Result<(Arc<wgpu::TextureView>, Arc<wgpu::Sampler>), AssetError>
where
    Dir: AsRef<Path>,
    Uri: AsRef<str>,
{
    // 텍스처 데이터를 가져옵니다.
    let data = TextureDataPool::get_or_init(workspace.as_ref(), uri.as_ref())?;
    // 텍스처를 가져옵니다.
    let texture = TexturePool::get_or_init(workspace, device, encoder, staging_buffers, &data)?;
    // 텍스처 뷰를 가져옵니다.
    let view = TextureViewPool::get_or_init(
        &texture,
        &wgpu::TextureViewDescriptor {
            dimension: Some(data.dimension.into()),
            ..Default::default()
        },
    );
    // 텍스처 샘플러를 가져옵니다.
    let sampler = SamplerPool::get_or_init(
        device,
        &wgpu::SamplerDescriptor {
            address_mode_u: data.address_u.into(),
            address_mode_v: data.address_v.into(),
            address_mode_w: data.address_w.into(),
            mag_filter: data.filter_mode.into(),
            min_filter: data.filter_mode.into(),
            mipmap_filter: data.filter_mode.into(),
            ..Default::default()
        },
    );

    Ok((view, sampler))
}
