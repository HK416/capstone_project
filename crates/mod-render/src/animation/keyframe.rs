use super::Skinning;



/// 애니메이션을 구성하는 키 프레임 데이터입니다.
#[derive(Debug)]
pub struct KeyFrame {
    /// 키 프레임의 시각입니다.
    time_point: f32, 

    /// 키 프레임에 영향을 받는 스키닝 데이터입니다.
    meshes: Vec<Skinning>
}


impl KeyFrame {
    /// 새로운 키 프레임 데이터를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 키 프레임 스키닝 데이터가 비어있는 경우 `panic!`을 호출합니다.
    /// 
    #[must_use]
    pub fn new<I>(time_point: f32, meshes: I) -> Self 
    where I: IntoIterator<Item = Skinning>, I::IntoIter: ExactSizeIterator {
        let meshes: Vec<_> = meshes.into_iter().collect();
        assert!(!meshes.is_empty(), "The given skinning data is empty!");
        unsafe { Self::new_unchecked(time_point, meshes) } // Safe: 키 프레임 스키닝 데이터는 비어있지 않음
    }

    /// 새로운 키 프레임 데이터를 생성합니다.
    /// 
    /// # Unsafe
    /// 주어진 키 프레임 스키닝 데이터가 비어있는 경우 정의되지 않은 동작입니다.
    /// 
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked<I>(time_point: f32, meshes: I) -> Self 
    where I: IntoIterator<Item = Skinning>, I::IntoIter: ExactSizeIterator {
        Self { time_point, meshes: meshes.into_iter().collect() }
    }

    /// 키 프레임의 시각을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn time_point(&self) -> f32 {
        self.time_point
    }

    /// 키 프레임 스키닝 메쉬를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn meshes(&self) -> &[Skinning] {
        &self.meshes
    }
}
