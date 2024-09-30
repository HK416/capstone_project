/// 원근 투영 변환 행렬을 생성하는 데이터입니다.
#[derive(Debug, Clone, Copy)]
pub struct Perspective {
    pub view_width: f32, 
    pub view_height: f32, 
    pub near_z: f32, 
    pub far_z: f32, 
}

impl Perspective {
    /// 새로운 원근 투영 변한 행렬 데이터를 생성합니다.
    /// 
    /// # Panics
    /// 아래 조건을 만족할 경우 [`panic!`]을 호출합니다.
    /// - 주어진 `far_z`가 `near_z`보다 작거나 같을 경우.
    /// - 주어진 `near_z`, `far_z`가 0보다 작거나 같을 경우.
    /// - 주어진 `view_width`, `view_height`가 0보다 작거나 같을 경우.
    /// 
    #[must_use]
    pub fn new(
        view_width: f32, 
        view_height: f32, 
        near_z: f32, 
        far_z: f32
    ) -> Self {
        assert!(far_z > near_z);
        assert!(near_z > 0.0 && far_z > 0.0);
        assert!(view_width > 0.0 && view_height > 0.0);
        Self { view_width, view_height, near_z, far_z }
    }

    /// 원근 투영 변환 행렬을 생성합니다.
    #[must_use]
    pub fn to_projection_matrix(self) -> gmm::Matrix {
        let two_near_z = self.near_z + self.near_z;
        let range = self.far_z / (self.far_z - self.near_z);

        gmm::Matrix::new(
            two_near_z / self.view_width, 0.0, 0.0, 0.0, 
            0.0, two_near_z / self.view_height, 0.0, 0.0, 
            0.0, 0.0, range, 1.0, 
            0.0, 0.0, -range * self.near_z, 0.0
        )
    }
}
