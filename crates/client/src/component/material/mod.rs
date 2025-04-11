//! 재질과 관련된 코드를 관리합니다.
//!

mod bullet;
mod character;
mod stage;

use std::{fs::OpenOptions, io::Read, ops::RangeBounds, path::Path, sync::Arc};

use ahash::{HashMap, RandomState};
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};

use crate::asset::AssetError;

pub use self::{bullet::*, character::*, stage::*};

/// 재질 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MaterialData {
    Bullet(BulletMaterialData),
    EnergyBullet(EnergyBulletMaterialData),
    Character(CharacterMaterialData),
    CharacterEyeMouth(EyeMouthMaterialData),
    CharacterHalo(HaloMaterialData),
    Stage(StageMaterialData),
}

/// 생성된 재질 데이터를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct MaterialDataPool(Arc<FairMutex<MaterialDataPoolType>>);

/// 재질 데이터 풀 객체의 타입입니다.
pub type MaterialDataPoolType = HashMap<String, Arc<MaterialData>>;

/// 재질 데이터 풀 객체의 용량입니다.
pub const METARIAL_DATA_POOL_CAPACITY: usize = 64;

impl MaterialDataPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            METARIAL_DATA_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, MaterialDataPoolType> {
        self.0.lock()
    }

    /// 파일로부터 [MaterialData]를 생성합니다.
    fn load_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<MaterialData, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.material", uri.as_ref()));

        log::debug!("open material data asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open material data asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read material data asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read material data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close material data asset (PATH:{})", path.display());
        drop(file);

        log::debug!("decode material data asset (PATH:{})", path.display());
        serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to decode material data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })
    }

    /// 재질 데이터 풀 객체에 등록된 재질 데이터를 가져옵니다.  
    /// 해당 Uri에 등록된 재질 데이터가 없는 경우 재질 데이터를 새로 생성합니다.
    pub fn get_or_init<Dir, Uri>(
        &self,
        workspace: Dir,
        uri: Uri,
    ) -> Result<Arc<MaterialData>, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(texture) = pool.get(uri.as_ref()).cloned() {
            return Ok(texture);
        }

        // 재질 데이터를 생성합니다.
        let data = Arc::new(Self::load_from_file(workspace.as_ref(), uri.as_ref())?);

        // 생성된 재질 데이터를 풀 객체에 등록합니다.
        pool.insert(uri.as_ref().into(), data.clone());
        Ok(data)
    }

    /// 주어진 Uri에 해당하는 재질 데이터를 가져옵니다.
    /// 해당 재질 데이터가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<Arc<MaterialData>>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// 주어진 Uri에 해당하는 재질 데이터를 풀 객체에서 제거합니다.  
    /// 해당 재질 데이터가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<Arc<MaterialData>>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 재질 데이터를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}

/// 재질 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialUniform {
    Bullet(BulletMaterialUniform),
    EnergyBullet(EnergyBulletMaterialUniform),
    Character(CharacterMaterialUniform),
    CharacterEyeMouth(EyeMouthMaterialUniform),
    CharacterHalo(HaloMaterialUniform),
    Stage(StageMaterialUniform),
}

impl MaterialUniform {
    /// 범위에 해당하는 슬라이스된 유니폼 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        match self {
            MaterialUniform::Bullet(uniform) => uniform.slice(bounds),
            MaterialUniform::EnergyBullet(uniform) => uniform.slice(bounds),
            MaterialUniform::Character(uniform) => uniform.slice(bounds),
            MaterialUniform::CharacterEyeMouth(uniform) => uniform.slice(bounds),
            MaterialUniform::CharacterHalo(uniform) => uniform.slice(bounds),
            MaterialUniform::Stage(uniform) => uniform.slice(bounds),
        }
    }

    /// 유니폼 버퍼의 [`wgpu::BindingResource`]를 반환합니다.
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            MaterialUniform::Bullet(uniform) => uniform.as_entire_binding(),
            MaterialUniform::EnergyBullet(uniform) => uniform.as_entire_binding(),
            MaterialUniform::Character(uniform) => uniform.as_entire_binding(),
            MaterialUniform::CharacterEyeMouth(uniform) => uniform.as_entire_binding(),
            MaterialUniform::CharacterHalo(uniform) => uniform.as_entire_binding(),
            MaterialUniform::Stage(uniform) => uniform.as_entire_binding(),
        }
    }
}

/// 재질 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialResource(pub(crate) Arc<wgpu::BindGroup>);

impl MaterialResource {
    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}
