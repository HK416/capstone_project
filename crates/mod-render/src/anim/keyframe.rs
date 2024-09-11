use super::Bone;



/// 애니메이션 키 프레임 데이터입니다.
#[derive(Debug)]
pub struct KeyFrame {
    /// 키 프레임 시각입니다.
    time_point: f32, 

    /// 애니메이션 뼈 데이터입니다.
    bones: Vec<Bone>
}

impl KeyFrame {
    /// 애니메이션 키 프레임을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<I>(
        time_point: f32, 
        bones: I
    ) -> Self 
    where 
        I: IntoIterator<Item = Bone>, 
        I::IntoIter: ExactSizeIterator,
    {
        Self { 
            time_point, 
            bones: bones.into_iter().collect() 
        }
    }

    /// 키 프레임 시각을 반환합니다.
    #[inline]
    #[must_use]
    pub fn time_point(&self) -> f32 {
        self.time_point
    }

    /// 키 프레임 뼈 데이터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn bones<'a>(&'a self) -> impl Iterator<Item = &'a Bone> + 'a {
        self.bones.iter()
    }
}
