#![allow(dead_code)]
//! 메쉬 에셋과 관련된 코드를 관리합니다.
//!

use std::{fs::OpenOptions, io::Read, path::Path, sync::Arc};

use ahash::{HashMap, RandomState};
use mod_network::components::{Float2, Float3, Float4, Matrix, Uint4};
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};

use crate::component::{Attributes, BindposeUniform, Indices, Mesh, Vertices};

use super::AssetError;

/// 모델 메쉬 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshData {
    pub name: String,
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
    pub skinning: Option<SkinningData>,
}

/// 모델 메쉬의 스키닝 애니메이션 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinningData {
    pub quality: u32,
    pub root_bone: String,
    pub bones: Vec<String>,
    pub bindposes: Vec<Matrix>,
}

/// 모델 메쉬의 스키닝 애니메이션 데이터입니다.
#[derive(Debug)]
pub struct Skinning {
    /// 바인드 포즈 뼈 변환 행렬 유니폼 버퍼입니다.
    pub bindpose_uniform: BindposeUniform,
    /// 최상위 뼈 노드의 이름입니다.
    pub root_bone: String,
    /// 스키닝 메쉬에 포함된 뼈 노드의 이름입니다.
    pub bones: Vec<String>,
}

/// 생성된 메쉬 객체를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct MeshPool(Arc<FairMutex<MeshPoolType>>);

/// 메쉬 풀 객체의 타입입니다.
pub type MeshPoolType = HashMap<String, (Arc<Mesh>, Option<Arc<Skinning>>)>;

/// 메쉬 풀 객체의 용량입니다.
pub const MESH_POOL_CAPACITY: usize = 64;

impl MeshPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            MESH_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, MeshPoolType> {
        self.0.lock()
    }

    /// 파일로부터 [MeshData]를 생성합니다.
    fn load_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<MeshData, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.mesh", uri.as_ref()));

        log::debug!("open mesh data asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open mesh data asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read mesh data asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read mesh data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close mesh data asset (PATH:{})", path.display());
        drop(file);

        log::debug!("decode mesh data asset (PATH:{})", path.display());
        serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to decode mesh data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })
    }

    /// 주어진 데이터로 메쉬를 생성합니다.
    fn create_mesh(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: MeshData,
    ) -> Result<(Arc<Mesh>, Option<Arc<Skinning>>), AssetError> {
        // 정점 데이터가 비어있는지 확인합니다.
        if data.vertices.is_empty() {
            log::error!("invalid mesh data (URI:{})", &data.name);
            return Err(AssetError::InvalidData);
        }

        // 메쉬를 생성합니다.
        let mut mesh = Mesh::new(
            &data.name,
            device,
            encoder,
            staging_buffers,
            Vertices(data.vertices.into_iter().map(|it| it.into()).collect()),
        );

        // 정점 속성을 추가합니다.
        if !data.colors.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Color(data.colors.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.normals.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Normal(data.normals.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.tangents.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Tangent(data.tangents.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.texcoords0.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Texcoord0(data.texcoords0.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.texcoords1.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Texcoord1(data.texcoords1.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.texcoords2.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Texcoord2(data.texcoords2.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.texcoords3.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::Texcoord3(data.texcoords3.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.bone_indices.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::BoneIndex(data.bone_indices.into_iter().map(|it| it.into()).collect()),
            );
        }

        if !data.bone_weights.is_empty() {
            mesh.with_attribute(
                device,
                encoder,
                staging_buffers,
                Attributes::BoneWeight(data.bone_weights.into_iter().map(|it| it.into()).collect()),
            );
        }

        // 하위 메쉬 집합을 추가합니다.
        for submesh in data.submeshes {
            mesh.with_submesh(device, encoder, staging_buffers, Indices(submesh));
        }

        let skinning = data.skinning.map(|skinning_data| {
            // 바인드 포즈 뼈 변환 행렬 유니폼 버퍼를 생성합니다.
            let bindpose_uniform = BindposeUniform::new(
                Some(&format!("Bindpose({})", &data.name)),
                device,
                encoder,
                staging_buffers,
                skinning_data
                    .bindposes
                    .into_iter()
                    .map(|it| it.into())
                    .collect(),
            );

            Arc::new(Skinning {
                bindpose_uniform,
                root_bone: skinning_data.root_bone,
                bones: skinning_data.bones,
            })
        });

        Ok((Arc::new(mesh), skinning))
    }

    /// 메쉬 풀 객체에 등록된 메쉬를 가져옵니다.
    /// 해당 Uri에 등록된 메쉬가 없는 경우 메쉬를 새로 생성합니다.
    pub fn get_or_init<Dir, Uri>(
        &self,
        workspace: Dir,
        uri: Uri,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) -> Result<(Arc<Mesh>, Option<Arc<Skinning>>), AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(pair) = pool.get(uri.as_ref()).cloned() {
            return Ok(pair);
        }

        // 메쉬를 생성합니다.
        let data = Self::load_from_file(workspace.as_ref(), uri.as_ref())?;
        let mesh = Self::create_mesh(device, encoder, staging_buffers, data)?;

        // 생성된 메쉬를 풀 객체에 등록합니다.
        pool.insert(uri.as_ref().into(), mesh.clone());
        Ok(mesh)
    }
    /// 메쉬 풀 객체에 메쉬를 등록합니다.  
    /// 이미 Uri에 해당하는 메쉬가 존재할 경우 기존의 메쉬를 반환합니다.
    pub fn insert<Uri>(
        &self,
        uri: Uri,
        mesh: Arc<Mesh>,
        skinning: Option<Arc<Skinning>>,
    ) -> Option<(Arc<Mesh>, Option<Arc<Skinning>>)>
    where
        Uri: AsRef<str>,
    {
        self.lock().insert(uri.as_ref().into(), (mesh, skinning))
    }

    /// 메쉬 객체에 해당하는 메쉬 객체를 가져옵니다.
    /// 해당 메쉬 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<(Arc<Mesh>, Option<Arc<Skinning>>)>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// Uri에 해당하는 메쉬 객체를 풀 객체에서 제거합니다.  
    /// 메쉬 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<(Arc<Mesh>, Option<Arc<Skinning>>)>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 메쉬 객체를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}
