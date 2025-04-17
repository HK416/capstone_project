#![allow(dead_code)]
//! 애니메이션 데이터 에셋과 관련된 코드를 관리합니다.
//!

use std::{fs::OpenOptions, io::Read, path::Path, sync::Arc};

use ahash::{HashMap, RandomState};
use mod_network::components::Matrix;
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};

use super::AssetError;

/// 로드된 애니메이션 데이터를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct MotionPool(Arc<FairMutex<AnimationPoolType>>);

/// 애니메이션 풀 객체의 타입입니다.
pub type AnimationPoolType = HashMap<String, Arc<HashMap<String, Motion>>>;

/// 애니메이션 풀 객체의 용량입니다.
pub const ANIMATION_POOL_CAPACITY: usize = 128;

impl MotionPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            ANIMATION_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, AnimationPoolType> {
        self.0.lock()
    }

    /// 파일로부터 [MotionData]를 생성합니다.
    fn load_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<Vec<MotionData>, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.motion", uri.as_ref()));

        log::debug!("open animation data asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open animation data asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read animation data asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read animation data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close animation data asset (PATH:{})", path.display());
        drop(file);

        log::debug!("decode animation data asset (PATH:{})", path.display());
        serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to decode animation data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })
    }

    /// 애니메이션 데이터 풀 객체에서 등록된 애니메이션을 가져옵니다.  
    /// 해당 Uri에 등록된 애니메이션 데이터가 없는 경우 파일에서 읽어 생성합니다.
    pub fn get_or_init<Dir, Uri>(
        &self,
        workspace: Dir,
        uri: Uri,
    ) -> Result<Arc<HashMap<String, Motion>>, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(motion) = pool.get(uri.as_ref()).cloned() {
            return Ok(motion);
        }

        // 애니메이션 데이터를 생성합니다.
        let data = Self::load_from_file(workspace.as_ref(), uri.as_ref())?;
        let motion: HashMap<String, Motion> = data
            .into_iter()
            .map(|data| {
                (
                    data.name,
                    Motion {
                        length: data.length,
                        frame_rate: data.frame_rate,
                        keyframes: data
                            .keyframes
                            .into_iter()
                            .map(|keyframe| KeyFrame {
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
                    },
                )
            })
            .collect();
        let motion = Arc::new(motion);

        // 생성된 애니메이션 데이터를 풀 객체에 등록합니다.
        pool.insert(uri.as_ref().into(), motion.clone());
        Ok(motion)
    }

    /// 주어진 Uri에에 해당하는 애니메이션 데이터를 가져옵니다.
    /// 해당 애니메이션 데이터가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<Arc<HashMap<String, Motion>>>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// Uri에 해당하는 애니메이션 데이터를 풀 객체에서 제거합니다.  
    /// 애니메이션 데이터가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<Arc<HashMap<String, Motion>>>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 애니메이션 데이터를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}

/// 애니메이션 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MotionData {
    pub name: String,
    pub length: f32,
    pub frame_rate: f32,
    pub keyframes: Vec<KeyFrameData>,
}

/// 애니메이션의 키 프레임 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameData {
    pub time_point: f32,
    /// NOTE: 스키닝된 메쉬의 최상위 뼈 노드가 아닌 모델의 최상위 뼈 노드 변환 행렬
    pub root_matrix: Matrix,
    pub meshes: Vec<KeyFrameMeshBlob>,
}

/// 애니메이션의 키 프레임 스키닝 메쉬 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMeshBlob {
    pub name: String,
    pub bone_trans: Vec<Matrix>,
}

/// 애니메이션 데이터입니다.
#[derive(Debug, Clone)]
pub struct Motion {
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
            root_matrix,
            meshes,
        }
    }
}

/// ## Animation Key Frame Data
#[derive(Debug, Clone)]
pub struct KeyFrame {
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
