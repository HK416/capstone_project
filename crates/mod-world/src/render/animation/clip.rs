use crate::objects::ObjectId;

use super::{KeyFrame, SkinningData};



/// 애니메이션 데이터입니다.
pub struct AnimationClip {
    /// 애니메이션 이름입니다.
    name: String, 

    /// 최상위 뼈 노드의 식별자입니다.
    root_bone: ObjectId, 

    /// 애니메이션의 총 재생길이입니다.
    length: f32, 

    /// 애니메이션 샘플링 프레임 레이트입니다.
    frame_rate: f32, 

    /// 애니메이션을 구성하는 키 프레임 데이터입니다.
    /// 
    /// 키 프레임의 시각이 낮은 순으로 정렬되어 있습니다.
    /// 
    keyframes: Vec<KeyFrame>, 
}

impl AnimationClip {
    /// 새로운 애니메이션 데이터를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 애니메이션의 길이 또는 애니메이션 샘플링 프레임 레이트가 0보다 작거나 같을 경우
    /// 또는 주어진 키 프레임 데이터가 비어있는 경우 `panic!`을 호출합니다.
    /// 
    pub fn new<N, I>(name: N, root_bone: ObjectId, length: f32, frame_rate: f32, keyframes: I) -> Self 
    where 
        N: Into<String>, 
        I: IntoIterator<Item = KeyFrame>, 
        I::IntoIter: ExactSizeIterator,
    {
        let name = name.into();
        assert!(length > 0.0, "The total length of the given animation must be greater than zero!");
        assert!(frame_rate > 0.0, "The frame rate of the given animation must be greater than zero!");

        let mut keyframes: Vec<_> = keyframes.into_iter().collect();
        assert!(!keyframes.is_empty(), "The given key frame data is empty!");
        keyframes.sort_by(|lhs, rhs| lhs.time_point().total_cmp(&rhs.time_point()));

        unsafe { Self::new_unchecked(name, root_bone, length, frame_rate, keyframes) }
    }

    /// 새로운 애니메이션 데이터를 생성합니다.
    /// 
    /// # Unsafe
    /// 주어진 애니메이션의 길이 또는 애니메이션 샘플링 프레임 레이트가 0보다 작거나 같을 경우
    /// 또는 주어진 키 프레임 데이터가 존재하지 않거나 키 프레임 시각으로 정렬되지 않았을 경우
    /// 정의되지 않은 동작입니다.
    /// 
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked<N, I>(name: N, root_bone: ObjectId, length: f32, frame_rate: f32, keyframes: I) -> Self 
    where 
        N: Into<String>, 
        I: IntoIterator<Item = KeyFrame>, 
        I::IntoIter: ExactSizeIterator,
    {
        Self { name: name.into(), root_bone, length, frame_rate, keyframes: keyframes.into_iter().collect() }
    }

    /// 애니메이션 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 최상위 뼈 노드의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn root_bone_id(&self) -> &ObjectId {
        &self.root_bone
    }

    /// 애니메이션의 총 재생 길이를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn length(&self) -> f32 {
        self.length
    }

    /// 애니메이션의 샘플링 프레임 레이트를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn frame_rate(&self) -> f32 {
        self.frame_rate
    }

    /// 주어진 시각의 애니메이션을 샘플링합니다.
    /// 
    /// `time_point`는 애니메이션 총 재생 길이 범위로 변환됩니다.
    /// 
    #[must_use]
    pub fn sample_animation(&self, time_point: f32) -> KeyFrame {
        let time_point = time_point.min(self.length());
        let delta_time = 1.0 / self.frame_rate(); // 애니메이션 키 프레임 간격
        let max_keyframe_index = self.keyframes.len() - 1; // Safe: 키 프레임 데이터는 적어도 1개 이상

        let prev = ((time_point / delta_time).floor() as usize).min(max_keyframe_index);
        let next = (prev + 1).min(max_keyframe_index);
        
        let t = (time_point % delta_time) / delta_time; // 두 키 프레임의 선형보간을 위한 오프셋
        let prev = unsafe { self.keyframes.get(prev).unwrap_unchecked() }; // Safe: prev는 키 프레임 배열의 범위를 벗어나지 않음
        let next = unsafe { self.keyframes.get(next).unwrap_unchecked() }; // Safe: next는 키 프레임 배열의 범위를 벗어나지 않음

        let meshes = prev.meshes().iter().zip(next.meshes().iter())
            .map(|(prev, next)| {
                SkinningData {
                    mesh: prev.mesh.clone(), 
                    transforms: prev.transforms.iter().zip(next.transforms.iter())
                        .map(|(&a, &b)| {
                            (1.0 - t) * a + t * b
                        })
                        .collect()
                }
            });

        let root_bone = (1.0 - t) * prev.root_bone() + t * next.root_bone();
        unsafe { KeyFrame::new_unchecked(time_point, root_bone, meshes) } // Safe: 키 프레임 스키닝 데이터는 비어있지 않습니다.
    }
}
