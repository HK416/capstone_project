use super::Bone;
use super::KeyFrame;



/// 애니메이션 데이터입니다.
#[derive(Debug)]
pub struct Animation {
    /// 애니메이션 이름입니다.
    name: String, 

    /// 애니메이션 재생 길이입니다.
    length: f32, 

    /// 애니메이션의 초당 프레임 갯수입니다.
    frame_rate: f32, 

    /// 애니메이션 키 프레임 데이터입니다.
    keyframes: Vec<KeyFrame>, 
}

impl Animation {
    /// 새로운 애니메이션 데이터를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 키 프레임 데이터가 비어있거나 주어진 애니메이션 길이가 0보다 작거나 같을 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn new<S, I>(
        name: S, 
        length: f32, 
        frame_rate: f32, 
        keyframes: I
    ) -> Self 
    where 
        S: Into<String>, 
        I: IntoIterator<Item = KeyFrame>, 
        I::IntoIter: ExactSizeIterator, 
    {
        // 애니메이션 재생 길이를 확인합니다.
        assert!(0.0 <= length, "The length of the given animation must be greater than zero!");

        // 키 프레임 데이터를 확인합니다.
        let mut keyframes: Vec<_> = keyframes.into_iter().collect();
        assert!(!keyframes.is_empty(), "The given keyframe data is empty!");

        // 키 프레임을 시간 순서로 정렬합니다.
        keyframes.sort_by(|lhs, rhs| lhs.time_point().total_cmp(&rhs.time_point()));

        Self { name: name.into(), length, frame_rate, keyframes }
    }

    /// 애니에미션 이름을 반환합니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 애니메이션 재생 길이를 반환합니다.
    #[inline]
    #[must_use]
    pub fn length(&self) -> f32 {
        self.length
    }

    /// 주어진 시각의 키 프레임 데이터를 가져옵니다.
    /// 주어진 시각은 애니메이션 재생 길이 범위로 변환됩니다.
    /// 
    #[must_use]
    pub fn sample_animation(&self, time_point: f32) -> KeyFrame {
        let time_point = time_point.clamp(0.0, self.length);
        let delta_time = 1.0 / self.frame_rate;
        let max_keyframe_index = self.keyframes.len() - 1;
        let prev_index = ((time_point / delta_time).floor() as usize).min(max_keyframe_index);
        let next_index = ((time_point / delta_time).floor() as usize + 1).min(max_keyframe_index);

        let t = (time_point % delta_time) / delta_time;
        let prev_bones = self.keyframes[prev_index].bones();
        let next_bones = self.keyframes[next_index].bones();

        let bones: Vec<_> = prev_bones.zip(next_bones)
            .map(|(prev, next)| {
                let target = prev.target().clone();
                let transforms: Vec<_> = prev.transforms().zip(next.transforms())
                    .map(|(prev, next)| prev.lerp(next, t))
                    .collect();
                Bone::new(target, transforms)
            })
            .collect();
        
        KeyFrame::new(time_point, bones)
    }
}
