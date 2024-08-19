use std::cmp;

use crate::animation::SkinnedMesh;



/// 애니메이션 키 프레임입니다.
#[derive(Debug)]
pub struct KeyFrame {
    /// 키 프레임의 시간입니다.
    time_point: f32, 

    /// 애니메이션 메쉬입니다.
    meshes: Vec<SkinnedMesh>, 
}

impl KeyFrame {
    /// 애니메이션 키 프레임을 생성합니다.
    #[must_use]
    pub fn new<I>(time_point: f32, meshes: I) -> Self 
    where 
        I: IntoIterator<Item = SkinnedMesh>, 
        I::IntoIter: ExactSizeIterator,
    {
        Self { time_point, meshes: meshes.into_iter().collect() }
    }

    /// 키 프레임의 시간을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn time_point(&self) -> f32 {
        self.time_point
    }

    /// 키 프레임의 스키닝된 메쉬를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn meshes(&self) -> &[SkinnedMesh] {
        &self.meshes
    }
}

impl Eq for KeyFrame { }

impl cmp::PartialEq<Self> for KeyFrame {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.time_point.eq(&other.time_point)
    }
}

impl Ord for KeyFrame {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.time_point.total_cmp(&other.time_point)
    }
}

impl cmp::PartialOrd<Self> for KeyFrame {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.time_point.partial_cmp(&other.time_point)
    }
}
