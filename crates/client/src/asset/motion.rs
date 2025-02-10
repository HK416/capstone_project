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
    #[allow(dead_code)]
    pub fn remove(name: &str) -> Option<Arc<HashMap<String, Motion>>> {
        get_pool().remove(name)
    }

    /// 풀 객체에 존재하는 모든 애니메이션 데이터를 제거합니다.
    #[allow(dead_code)]
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
        .map_err(|e| ModelAssetError::IOError(path.clone(), e))?;
    let reader = Cursor::new(cached_asset.as_bytes());
    let blob: Vec<MotionBlob> =
        serde_json::de::from_reader(reader).map_err(|e| ModelAssetError::ParsingFailed(path.clone(), e))?;
    let blob = blob
        .into_iter()
        .map(|blob| (blob.name.clone(), blob.into()))
        .collect();

    asset_manager.remove(path);
    Ok(Arc::new(blob))
}

/// ## Animation Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MotionBlob {
    pub name: String,
    /// NOTE: 스키닝된 메쉬의 최상위 뼈 노드가 아닌 모델의 최상위 뼈 노드
    /// 차후 제거 예정
    pub root: String,
    pub length: f32,
    pub frame_rate: f32,
    pub keyframes: Vec<KeyFrameBlob>,
}

impl Into<Motion> for MotionBlob {
    fn into(self) -> Motion {
        Motion {
            name: self.name,
            length: self.length,
            frame_rate: self.frame_rate,
            keyframes: self
                .keyframes
                .into_iter()
                .map(|keyframe| KeyFrame {
                    time_point: keyframe.time_point,
                    root_matrix: keyframe.root_matrix.into(),
                    meshes: keyframe
                        .meshes
                        .into_iter()
                        .map(|mesh| KeyFrameMesh {
                            name: mesh.name,
                            bone_trans: mesh
                                .bone_trans
                                .into_iter()
                                .map(|trans| trans.into())
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// ## Animation Key Frame Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameBlob {
    pub time_point: f32,
    /// NOTE: 스키닝된 메쉬의 최상위 뼈 노드가 아닌 모델의 최상위 뼈 노드 변환 행렬
    pub root_matrix: Matrix,
    pub meshes: Vec<KeyFrameMeshBlob>,
}

/// ## Animation Key Frame Skinned Mesh Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMeshBlob {
    pub name: String,
    pub bone_trans: Vec<Matrix>,
}

#[derive(Debug, Clone)]
pub struct Motion {
    #[allow(dead_code)]
    pub name: String,
    pub length: f32,
    pub frame_rate: f32,
    pub keyframes: Vec<KeyFrame>,
}

impl Motion {
    pub fn linear_sampling(&self, time_point: f32) -> KeyFrame {
        debug_assert!(!self.keyframes.is_empty(), "invalid animation data");

        let time_point = time_point.min(self.length);
        let delta_time = 1.0 / self.frame_rate; // 애니메이션 키 프레임 간격
        let max_keyframe_index = self.keyframes.len() - 1;

        let prev = ((time_point / delta_time).floor() as usize).min(max_keyframe_index);
        let next = (prev + 1).min(max_keyframe_index);

        let t = (time_point % delta_time) / delta_time; // 두 키 프레임의 선형 보간을 위한 오프셋
        let prev = &self.keyframes[prev];
        let next = &self.keyframes[next];

        let root_matrix = (1.0 - t) * prev.root_matrix + t * next.root_matrix;
        let meshes = prev
            .meshes
            .iter()
            .zip(next.meshes.iter())
            .map(|(prev, next)| KeyFrameMesh {
                name: prev.name.clone(),
                bone_trans: prev
                    .bone_trans
                    .iter()
                    .zip(next.bone_trans.iter())
                    .map(|(&lhs, &rhs)| (1.0 - t) * lhs + t * rhs)
                    .collect(),
            })
            .collect();

        KeyFrame {
            time_point,
            root_matrix,
            meshes,
        }
    }
}

/// ## Animation Key Frame Data
#[derive(Debug, Clone)]
pub struct KeyFrame {
    #[allow(dead_code)]
    pub time_point: f32,
    /// NOTE: 스키닝된 메쉬의 최상위 뼈 노드가 아닌 모델의 최상위 뼈 노드 변환 행렬
    pub root_matrix: glam::Mat4,
    pub meshes: Vec<KeyFrameMesh>,
}

/// ## Animation Key Frame Skinned Mesh Data
#[derive(Debug, Clone)]
pub struct KeyFrameMesh {
    pub name: String,
    pub bone_trans: Vec<glam::Mat4>,
}
