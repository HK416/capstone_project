use std::{
    io::{self, Cursor},
    path::PathBuf,
    sync::OnceLock,
};

use ahash::{HashMap, RandomState};
use dashmap::DashMap;
use mod_app::{asset::AssetManager, error::AssetLoadError};

use super::blob::Action;

/// 로드된 애니메이션 데이터를 관리하는 풀 객체입니다.
static POOL: OnceLock<DashMap<String, HashMap<String, Action>, RandomState>> = OnceLock::new();

/// 애니메이션 데이터를 관리하는 풀 객체를 가져옵니다.
fn get_pool() -> &'static DashMap<String, HashMap<String, Action>, RandomState> {
    POOL.get_or_init(|| DashMap::default())
}

/// ## Action Pool
/// 로드된 모델의 애니메이션 데이터를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `ActionPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct ActionPool;

impl ActionPool {
    /// 모델의 애니메이션 데이터를 가져옵니다.  
    /// 모델의 애니메이션 데이터가 풀 객체에 존재하지 않는 경우 파일에서 로드합니다.  
    ///
    /// # Errors
    /// 애니메이션 데이터를 로드하는 도중 오류가 발생한 경우 `Error`를 반환합니다.
    ///
    pub fn get_or_init<F>(
        name: &str,
        workspace: &str,
        asset_manager: &AssetManager,
        func: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(&HashMap<String, Action>),
    {
        let actions = get_pool()
            .entry(name.to_string())
            .or_insert(load_model_animation(name, workspace, asset_manager)?);
        func(&actions);
        Ok(())
    }

    /// 풀 객체에 존재하는 해당 애니메이션 데이터를 제거합니다.  
    /// 풀 객체에 해당 애니메이션 데이터가 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    pub fn remove(name: &str) -> Option<HashMap<String, Action>> {
        get_pool().remove(name).map(|(_, actions)| actions)
    }

    /// 풀 객체에 존재하는 모든 애니메이션 데이터를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}

/// ## Action Load Error List
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 에셋 파일을 구문 분석하는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to parse asset for the following reason:{0}")]
    ParsingFailed(#[from] serde_json::Error),

    /// 파일을 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("file not found (PATH:{0})")]
    FileNotFound(PathBuf),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read file for the following reason:{0}")]
    IOError(#[from] io::Error),
}

/// 모델의 애니메이션 데이터를 로드합니다.
fn load_model_animation(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
) -> Result<HashMap<String, Action>, Error> {
    let path = format!("{}/{}.action", workspace, name);
    let cached_asset = asset_manager.get_or_init(&path).map_err(|e| match e {
        AssetLoadError::IOError(e) => Error::IOError(e),
        AssetLoadError::PathNotFound(path) => Error::FileNotFound(path),
    })?;
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: Vec<Action> = serde_json::de::from_reader(reader).map_err(|e| Error::from(e))?;
    let blob = blob
        .into_iter()
        .map(|blob| (blob.name.clone(), blob))
        .collect();

    asset_manager.remove(path);
    Ok(blob)
}
