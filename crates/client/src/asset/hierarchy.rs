#![allow(dead_code)]
//! 모델 계층 구조 데이터 에셋과 관련된 코드를 관리합니다.
//!

use std::{fs::OpenOptions, io::Read, ops::Deref, path::Path, sync::Arc};

use ahash::{HashMap, RandomState};
use mod_network::components::Matrix;
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};

use crate::component::{MaterialData, MaterialDataPool, Mesh};

use super::{
    AssetError, MeshPool, SamplerPool, Skinning, TextureDataPool, TexturePool, TextureViewPool,
};

/// 모델의 계층 구조 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelData {
    pub root: ModelNodeData,
    pub num_nodes: u32,
}

/// 모델의 계층 구조를 구성하는 노드 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelNodeData {
    pub name: String,
    pub transform: Matrix,
    pub mesh: Option<String>,
    pub materials: Vec<String>,
    pub children: Vec<ModelNodeData>,
}

/// 루트 모델 노드입니다.
#[derive(Debug, Clone)]
pub struct ModelRoot {
    pub node: ModelNode,
    pub num_nodes: usize,
}

/// 모델 노드입니다.
#[derive(Debug, Clone)]
pub struct ModelNode {
    pub name: String,
    pub transform: glam::Mat4,
    pub mesh: Option<Arc<Mesh>>,
    pub skinning: Option<Arc<Skinning>>,
    pub materials: Vec<Arc<MaterialData>>,
    pub children: Vec<ModelNode>,
}

/// 로드된 모델 데이터를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct ModelPool(Arc<FairMutex<ModelPoolType>>);

/// 모델 데이터 풀 객체의 타입입니다.
pub type ModelPoolType = HashMap<String, Arc<ModelRoot>>;

/// 모델 데이터 풀 객체의 용량입니다.
pub const MODEL_POOL_CAPACITY: usize = 64;

impl ModelPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            MODEL_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, ModelPoolType> {
        self.0.lock()
    }

    /// 파일로부터 [ModelData]를 생성합니다.
    fn load_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<ModelData, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.hierarchy", uri.as_ref()));

        log::debug!("open model data asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open model data asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read model data asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read model data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close model data asset (PATH:{})", path.display());
        drop(file);

        log::debug!("decode model data asset (PATH:{})", path.display());
        serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to decode model data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })
    }

    /// [ModelData]로 부터 [ModelRoot]를 생성합니다.
    fn create_model_root<Dir>(
        mesh_pool: &MeshPool,
        material_data_pool: &MaterialDataPool,
        texture_data_pool: &TextureDataPool,
        texture_pool: &TexturePool,
        texture_view_pool: &TextureViewPool,
        sampler_pool: &SamplerPool,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        workspace: Dir,
        data: ModelData,
    ) -> Result<Arc<ModelRoot>, AssetError>
    where
        Dir: AsRef<Path>,
    {
        let node = Self::create_model_node(
            mesh_pool,
            material_data_pool,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
            device,
            encoder,
            staging_buffers,
            workspace,
            &data.root,
        )?;

        Ok(Arc::new(ModelRoot {
            node,
            num_nodes: data.num_nodes as usize,
        }))
    }

    /// [ModelNodeData]로 부터 [ModelNode]를 생성합니다.
    fn create_model_node<Dir>(
        mesh_pool: &MeshPool,
        material_data_pool: &MaterialDataPool,
        texture_data_pool: &TextureDataPool,
        texture_pool: &TexturePool,
        texture_view_pool: &TextureViewPool,
        sampler_pool: &SamplerPool,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        workspace: Dir,
        data: &ModelNodeData,
    ) -> Result<ModelNode, AssetError>
    where
        Dir: AsRef<Path>,
    {
        // 노드의 기본 정보를 수집합니다.
        let name = data.name.clone();
        let transform = data.transform.into_mat4();

        // 노드에 연결된 메쉬 데이터를 가져옵니다.
        let (mesh, skinning) = match &data.mesh {
            Some(mesh_uri) => {
                let (mesh, skinning) = mesh_pool.get_or_init(
                    workspace.as_ref(),
                    mesh_uri,
                    device,
                    encoder,
                    staging_buffers,
                )?;
                (Some(mesh), skinning)
            }
            None => (None, None),
        };

        // 노드에 연결된 재질 데이터를 가져옵니다.
        let mut materials = Vec::with_capacity(data.materials.len());
        for material_uri in data.materials.iter() {
            let data = material_data_pool.get_or_init(workspace.as_ref(), material_uri)?;
            match data.deref() {
                MaterialData::Character(data) => {
                    // 메인 컬러 텍스처를 로드합니다.
                    texture_data_pool.get_or_init(
                        workspace.as_ref(),
                        &data.main_color,
                        device,
                        encoder,
                        staging_buffers,
                        texture_pool,
                        texture_view_pool,
                        sampler_pool,
                    )?;
                }
                MaterialData::CharacterEyeMouth(data) => {
                    // 메인 컬러 텍스처를 로드합니다.
                    texture_data_pool.get_or_init(
                        workspace.as_ref(),
                        &data.main_color,
                        device,
                        encoder,
                        staging_buffers,
                        texture_pool,
                        texture_view_pool,
                        sampler_pool,
                    )?;
                    // 입 텍스처를 로드합니다.
                    texture_data_pool.get_or_init(
                        workspace.as_ref(),
                        &data.eye_mouth,
                        device,
                        encoder,
                        staging_buffers,
                        texture_pool,
                        texture_view_pool,
                        sampler_pool,
                    )?;
                }
                MaterialData::CharacterHalo(data) => {
                    // 메인 컬러 텍스처를 로드합니다.
                    texture_data_pool.get_or_init(
                        workspace.as_ref(),
                        &data.main_color,
                        device,
                        encoder,
                        staging_buffers,
                        texture_pool,
                        texture_view_pool,
                        sampler_pool,
                    )?;
                }
                MaterialData::Stage(data) => {
                    // 메인 컬러 텍스처를 로드합니다.
                    texture_data_pool.get_or_init(
                        workspace.as_ref(),
                        &data.main_color,
                        device,
                        encoder,
                        staging_buffers,
                        texture_pool,
                        texture_view_pool,
                        sampler_pool,
                    )?;
                }
                _ => {}
            }
            materials.push(data);
        }

        // 노드에 연결된 자식 데이터를 가져옵니다.
        let mut children = Vec::with_capacity(data.children.len());
        for data in data.children.iter() {
            let node = Self::create_model_node(
                mesh_pool,
                material_data_pool,
                texture_data_pool,
                texture_pool,
                texture_view_pool,
                sampler_pool,
                device,
                encoder,
                staging_buffers,
                workspace.as_ref(),
                data,
            )?;

            children.push(node);
        }

        Ok(ModelNode {
            name,
            transform,
            mesh,
            skinning,
            materials,
            children,
        })
    }

    /// 루트 모델 노드 풀 객체에 등록된 루트 모델 노드를 가져옵니다.  
    /// 해당 Uri에 등록된 루트 모델 노드가 없는 경우 루트 모델 노드를 새로 생성합니다.
    pub fn get_or_init<Dir, Uri>(
        &self,
        mesh_pool: &MeshPool,
        material_data_pool: &MaterialDataPool,
        texture_data_pool: &TextureDataPool,
        texture_pool: &TexturePool,
        texture_view_pool: &TextureViewPool,
        sampler_pool: &SamplerPool,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        workspace: Dir,
        uri: Uri,
    ) -> Result<Arc<ModelRoot>, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(texture) = pool.get(uri.as_ref()).cloned() {
            return Ok(texture);
        }

        // 텍스처 데이터를 생성합니다.
        let data = Self::load_from_file(workspace.as_ref(), uri.as_ref())?;
        let data = Self::create_model_root(
            mesh_pool,
            material_data_pool,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
            device,
            encoder,
            staging_buffers,
            workspace,
            data,
        )?;

        // 생성된 텍스처를 풀 객체에 등록합니다.
        pool.insert(uri.as_ref().into(), data.clone());
        Ok(data)
    }

    /// Uri에 해당하는 루트 모델 노드를 풀 객체에서 가져옵니다.
    /// 루트 모델 노드가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<Arc<ModelRoot>>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// Uri에 해당하는 루트 모델 노드를 풀 객체에서 제거합니다.  
    /// 루트 모델 노드가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<Arc<ModelRoot>>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 텍스처 뷰 객체를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}
