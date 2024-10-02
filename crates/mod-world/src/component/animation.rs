use crate::render::animation::AnimationClip;

/// 애니메이션 집합입니다.
pub struct AnimationSet {
    /// 애니메이션 클립 집합입니다.
    pub clips: Vec<AnimationClip>, 

    /// 현재 애니메이션 인덱스입니다.
    pub index: usize,

    /// 애니메이션 타이머입니다.
    pub timer: f32, 
}
