use gmm::Matrix;



/// 투영 행렬의 정보를 가지고 있습니다.
#[derive(Debug, Clone, Copy)]
pub enum Projection {
    Perspective(Perspective), 
    Orthographic(Orthographic), 
}

impl Projection {
    /// 투영 변환 행렬을 가져옵니다.
    pub fn get_projection_matrix(&self) -> Matrix {
        match self {
            Self::Perspective(it) => it.projection_matrix(), 
            Self::Orthographic(it) => it.projection_matrix(), 
        }
    }
}



/// 원근 투영 변환 행렬의 정보를 가지고 있습니다.
#[derive(Debug, Clone, Copy)]
pub struct Perspective {
    pub fov_y: f32, 
    pub aspect_ratio: f32, 
    pub z_near: f32, 
    pub z_far: f32
}

impl Perspective {
    /// 새로운 원근 투영 변환 행렬을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fov를 설정합니다.
    pub fn set_fov_y(mut self, fov_y: f32) -> Self {
        self.fov_y = fov_y;
        self
    }

    /// 종횡비를 설정합니다.
    pub fn set_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = aspect_ratio;
        self
    }

    /// 가까운 거리를 설정합니다.
    pub fn set_z_near(mut self, z_near: f32) -> Self {
        self.z_near = z_near;
        self
    }

    /// 먼 거리를 설정합니다.
    pub fn set_z_far(mut self, z_far: f32) -> Self {
        self.z_far = z_far;
        self
    }

    /// 원근 투영 변환 행렬을 반환합니다.
    pub fn projection_matrix(&self) -> Matrix {
        Matrix::perspective_rh(
            self.fov_y, 
            self.aspect_ratio, 
            self.z_near, 
            self.z_far
        )
    }
}

impl Default for Perspective {
    #[inline]
    fn default() -> Self {
        Perspective { 
            fov_y: 60_f32.to_radians(), 
            aspect_ratio: 16.0 / 9.0, 
            z_near: 0.001, 
            z_far: 1000.0 
        }
    }
}



/// 정사영 투영 변환 행렬의 정보를 가지고 있습니다.
#[derive(Debug, Clone, Copy)]
pub struct Orthographic {
    pub left: f32, 
    pub right: f32, 
    pub bottom: f32, 
    pub top: f32, 
    pub near: f32, 
    pub far: f32
}

impl Orthographic {
    /// 새로운 정사영 투영 변환 행렬을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }

    /// 왼쪽을 설정합니다.
    pub fn set_left(mut self, left: f32) -> Self {
        self.left = left;
        self
    }

    /// 오른쪽을 설정합니다.
    pub fn set_right(mut self, right: f32) -> Self {
        self.right = right;
        self
    }

    /// 하단을 설정합니다.
    pub fn set_bottom(mut self, bottom: f32) -> Self {
        self.bottom = bottom;
        self
    }

    /// 상단을 설정합니다.
    pub fn set_top(mut self, top: f32) -> Self {
        self.top = top;
        self
    }

    /// 가까운 거리를 설정합니다.
    pub fn set_near(mut self, near: f32) -> Self {
        self.near = near;
        self
    }

    /// 먼 거리를 설정합니다.
    pub fn set_far(mut self, far: f32) -> Self {
        self.far = far;
        self
    }

    /// 정사영 투영 변환 행렬을 반환합니다.
    pub fn projection_matrix(&self) -> Matrix {
        Matrix::orthographic_rh(
            self.left, 
            self.right, 
            self.bottom, 
            self.top, 
            self.near, 
            self.far
        )
    }
}

impl Default for Orthographic {
    #[inline]
    fn default() -> Self {
        Self { 
            left: 1.0, 
            right: -1.0, 
            bottom: -1.0, 
            top: 1.0, 
            near: 0.001, 
            far: 1000.0 
        }
    }
}
