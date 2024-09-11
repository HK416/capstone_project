use std::ops;



/// 정사영 투영 변환 행렬을 생성하는 데이터입니다.
#[derive(Debug, Clone, Copy)]
pub struct OrthographicLh {
    pub left: f32, 
    pub right: f32, 
    pub bottom: f32, 
    pub top: f32, 
    pub near: f32, 
    pub far: f32
}

impl OrthographicLh {
    /// 새로운 정사영 투영 변환 행렬 데이터를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 왼쪽 평면의 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_left(mut self, left: f32) -> Self {
        self.left = left;
        self
    }

    /// 오른쪽 평면의 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_right(mut self, right: f32) -> Self {
        self.right = right;
        self
    }

    /// 아래쪽 평면의 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_bottom(mut self, bottom: f32) -> Self {
        self.bottom = bottom;
        self
    }

    /// 위쪽 평면의 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_top(mut self, top: f32) -> Self {
        self.top = top;
        self
    }

    /// 가까운 평면의 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_near(mut self, near: f32) -> Self {
        self.near = near;
        self
    }

    /// 먼 평면의 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_far(mut self, far: f32) -> Self {
        self.far = far;
        self
    }
}

impl Default for OrthographicLh {
    #[inline]
    fn default() -> Self {
        Self { 
            left: -1.0, 
            right: 1.0, 
            bottom: -1.0, 
            top: 1.0, 
            near: 0.0001, 
            far: 1000.0 
        }
    }
}



/// 원근 투영 변환 행렬을 생성하는 데이터입니다.
#[derive(Debug, Clone, Copy)]
pub struct PerspectiveLh {
    pub fov_y: f32, 
    pub aspect_ratio: f32, 
    pub z_near: f32, 
    pub z_far: f32
}

impl PerspectiveLh {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fov를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_fov_y(mut self, fov_y: f32) -> Self {
        self.fov_y = fov_y;
        self
    }

    /// 종횡비를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = aspect_ratio;
        self
    }

    /// 가까운 평면 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_z_near(mut self, z_near: f32) -> Self {
        self.z_near = z_near;
        self
    }

    /// 먼 평면 거리를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_z_far(mut self, z_far: f32) -> Self {
        self.z_far = z_far;
        self
    }
}

impl Default for PerspectiveLh {
    #[inline]
    fn default() -> Self {
        Self { 
            fov_y: 60f32.to_radians(), 
            aspect_ratio: 1.0, 
            z_near: 0.0001, 
            z_far: 1000.0 
        }
    }
}



/// 카메라의 투영 변환 행렬입니다.
#[derive(Debug, Clone, Copy)]
pub struct Projection(gmm::Matrix);

impl From<OrthographicLh> for Projection {
    #[inline]
    fn from(value: OrthographicLh) -> Self {
        Self(gmm::Matrix::orthographic_lh(
            value.left, 
            value.right, 
            value.bottom, 
            value.top, 
            value.near, 
            value.far
        ))
    }
}

impl From<PerspectiveLh> for Projection {
    #[inline]
    fn from(value: PerspectiveLh) -> Self {
        Self(gmm::Matrix::perspective_lh(
            value.fov_y, 
            value.aspect_ratio, 
            value.z_near, 
            value.z_far
        ))
    }
}

impl ops::Deref for Projection {
    type Target = gmm::Matrix;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Projection {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Projection {
    #[inline]
    fn default() -> Self {
        Self(gmm::Float4x4::IDENTITY.into())
    }
}
