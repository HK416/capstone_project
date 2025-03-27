/// 평면을 나타내는 구조체입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    /// 평면의 노멀 벡터
    pub normal: glam::Vec3A,
    /// 평면의 원점으로부터 상대적인 위치  
    /// ※ 평면의 방정식: `Ax + By + Cz + D = 0`
    pub d: f32,
}

impl Plane {
    /// 새로운 평면을 생성합니다.  
    ///
    /// # Panics
    /// 주어진 `normal`이 정규화되지 않은 경우 `panic!`을 호출합니다.
    ///
    pub fn new<N>(normal: N, d: f32) -> Self
    where
        N: Into<glam::Vec3A>,
    {
        let normal: glam::Vec3A = normal.into();
        assert!(
            normal.is_normalized(),
            "the given `normal` must be normalized!"
        );
        unsafe { Self::new_unchecked(normal, d) }
    }

    /// 새로운 평면을 생성합니다.  
    /// 이 함수를 주어진 `normal`이 정규화되었는지 확인하지 않습니다.
    pub const unsafe fn new_unchecked(normal: glam::Vec3A, d: f32) -> Self {
        Self { normal, d }
    }

    /// 4차원 벡터로부터 평면을 생성합니다.
    pub fn from_vec4(v: glam::Vec4) -> Self {
        let normal = v.truncate();
        let d = v.w;
        Self::new(normal.normalize(), d)
    }

    /// 주어진 점과 평면 사이의 거리를 반환합니다.
    pub fn distance<P>(&self, point: P) -> f32
    where
        P: Into<glam::Vec3A>,
    {
        self.normal.dot(point.into()) + self.d
    }
}
