use gmm::Matrix;
use super::Projection;



/// 정사영 투영 변환 행렬을 생성하는 빌더 입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrthographicBuilder {
    pub left: f32, 
    pub right: f32, 
    pub bottom: f32, 
    pub top: f32, 
    pub near: f32, 
    pub far: f32
}

impl OrthographicBuilder {
    /// 새로운 정사영 투영 변환 행렬 빌더를 생성합니다.
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

    /// 투영 변환 행렬을 생성합니다.
    #[inline]
    pub fn build(self) -> Projection {
        Projection(
            Matrix::orthographic_rh(
                self.left, 
                self.right, 
                self.bottom, 
                self.top, 
                self.near, 
                self.far
            ).into()
        )
    }
}

impl Default for OrthographicBuilder {
    #[inline]
    #[must_use]
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
