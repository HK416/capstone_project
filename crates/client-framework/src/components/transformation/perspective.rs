use gmm::Matrix;

use crate::components::Projection;



/// 원근 투영 변환 행렬을 생성하는 빌더 입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveBuilder {
    pub fov_y: f32, 
    pub aspect: f32, 
    pub z_near: f32, 
    pub z_far: f32, 
}

impl PerspectiveBuilder {
    /// 새로운 투영 변환 행렬 빌더를 생성합니다.
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
    pub fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect = aspect;
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

    /// 원근 투영 변환 행렬을 생성합니다.
    #[inline]
    pub fn build(self) -> Projection {
        Projection(
            Matrix::perspective_rh(
                self.fov_y, 
                self.aspect, 
                self.z_near, 
                self.z_far
            ).into()
        )
    }
}

impl Default for PerspectiveBuilder {
    #[inline]
    #[must_use]
    fn default() -> Self {
        Self { 
            fov_y: 60f32.to_radians(), 
            aspect: 1.0, 
            z_near: 0.0001, 
            z_far: 1000.0 
        }
    }
}
