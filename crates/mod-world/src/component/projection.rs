#[derive(Debug, Clone, Copy)]
pub struct Projection(pub gmm::Matrix);

impl From<gmm::Matrix> for Projection {
    #[inline]
    fn from(value: gmm::Matrix) -> Self {
        Self(value)
    }
}

impl Into<gmm::Matrix> for Projection {
    #[inline]
    fn into(self) -> gmm::Matrix {
        self.0
    }
}

impl Projection {
    /// 새로운 원근 투영 행렬을 생성합니다.
    /// 
    /// # Panics
    /// 아래 조건을 만족하는 경우 [`panic!`]을 호출합니다.
    /// - 주어진 `fov_y`가 0보다 작거나 같을 경우.
    /// - 주어진 `aspect_ratio`가 0보다 작거나 같을 경우.
    /// - 주어진 `far_z`가 `near_z`보다 작거나 같을 경우.
    /// - 주어진 `near_z`, `far_z`가 0보다 작거나 같을 경우.
    /// 
    #[must_use]
    pub fn perspective(
        fov_y: f32, 
        aspect_ratio: f32, 
        z_near: f32, 
        z_far: f32
    ) -> Self {
        assert!(fov_y > 0.0);
        assert!(aspect_ratio > 0.0);
        assert!(z_far > z_near);
        assert!(z_near > 0.0 && z_far > 0.0);

        Self(gmm::Matrix::perspective_lh(fov_y, aspect_ratio, z_near, z_far))
    }
}
