use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

use ahash::HashMap;
use mod_app::asset::AssetManager;
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};

use super::{Matrix, ModelAssetError};

type PoolType = HashMap<String, Arc<HashMap<String, Motion>>>;

/// 로드된 애니메이션 데이터를 관리하는 풀 객체입니다.
static POOL: OnceLock<FairMutex<PoolType>> = OnceLock::new();

/// 애니메이션 데이터를 관리하는 풀 객체를 가져옵니다.
fn get_pool() -> FairMutexGuard<'static, PoolType> {
    POOL.get_or_init(|| FairMutex::new(HashMap::default()))
        .lock()
}

/// ## Motion Pool
/// 로드된 모델의 애니메이션 데이터를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `MotionPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct MotionPool;

impl MotionPool {
    /// 모델의 애니메이션 데이터를 가져옵니다.  
    /// 모델의 애니메이션 데이터가 풀 객체에 존재하지 않는 경우 파일에서 로드합니다.  
    ///
    /// # Errors
    /// 애니메이션 데이터를 로드하는 도중 오류가 발생한 경우 `Error`를 반환합니다.
    ///
    pub fn get_or_init(
        name: &str,
        workspace: &str,
        asset_manager: &AssetManager,
    ) -> Result<Arc<HashMap<String, Motion>>, ModelAssetError> {
        let mut pool = get_pool();
        match pool.get(name).cloned() {
            Some(motion) => Ok(motion),
            None => {
                let motion = load_model_animation(name, workspace, asset_manager)?;
                pool.insert(name.to_string(), motion.clone());
                Ok(motion)
            }
        }
    }

    /// 풀 객체에 존재하는 해당 애니메이션 데이터를 제거합니다.  
    /// 풀 객체에 해당 애니메이션 데이터가 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    pub fn remove(name: &str) -> Option<Arc<HashMap<String, Motion>>> {
        get_pool().remove(name)
    }

    /// 풀 객체에 존재하는 모든 애니메이션 데이터를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}

/// 모델의 애니메이션 데이터를 로드합니다.
fn load_model_animation(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
) -> Result<Arc<HashMap<String, Motion>>, ModelAssetError> {
    let path = format!("{}/{}.motion", workspace, name);
    let cached_asset = asset_manager
        .get_or_init(&path)
        .map_err(|e| ModelAssetError::from(e))?;
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: Vec<Motion> =
        serde_json::de::from_reader(reader).map_err(|e| ModelAssetError::from(e))?;
    let blob = blob
        .into_iter()
        .map(|blob| (blob.name.clone(), blob))
        .collect();

    asset_manager.remove(path);
    Ok(Arc::new(blob))
}

/// ## Animation Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Motion {
    pub name: String,
    /// NOTE: 스키닝된 메쉬의 최상위 뼈 노드가 아닌 모델의 최상위 뼈 노드
    /// 차후 제거 예정
    pub root: String,
    pub length: f32,
    pub frame_rate: f32,
    pub keyframes: Vec<KeyFrame>,
}

/// ## Animation Key Frame Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrame {
    pub time_point: f32,
    /// NOTE: 스키닝된 메쉬의 최상위 뼈 노드가 아닌 모델의 최상위 뼈 노드 변환 행렬
    pub root_matrix: Matrix,
    pub meshes: Vec<KeyFrameMesh>,
}

/// ## Animation Key Frame Skinned Mesh Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMesh {
    pub name: String,
    pub bone_trans: Vec<Matrix>,
}
