use glam::Vec4Swizzles;

/// ## Tag
/// 엔터티가 카메라임을 식별하는 태그입니다.
pub struct CameraTag;

/// ## Third Person Camera Data
#[derive(Debug, Clone, Copy)]
pub struct ThirdPersonCamera {
    /// xz평면에서 카메라가 바라보는 방향을 나타냅니다.
    pub view_matrix_xz: glam::Mat4,

    /// 삼인칭 카메라의 위치 오프셋입니다.
    pub position_offset: glam::Vec4,

    /// 카메라가 대상을 바라보는 각도입니다.
    pub pitch_angle: f32,

    /// 카메라와 대상 사이의 거리입니다.
    pub distance: f32,
}

impl ThirdPersonCamera {
    /// 카메라가 바라보는 방향 벡터를 갱신합니다.
    pub fn update_direction(&mut self, dx: f32, dy: f32, offset: f32) {
        use core::f32::consts::FRAC_PI_3;

        // 삼인칭 카메라가 바라보는 방향을 갱신합니다.
        let angle = (dx * offset).to_radians();
        let mat = glam::Mat4::from_rotation_y(angle);
        self.view_matrix_xz = mat * self.view_matrix_xz;

        // 삼인칭 카메라의 바라보는 회전 각도를 갱신합니다.
        let angle = (dy * offset).to_radians();
        self.pitch_angle = (self.pitch_angle + angle).clamp(-FRAC_PI_3, FRAC_PI_3);
    }

    /// 카메라의 바라보는 방향을 행렬로 반환합니다.
    pub fn to_matrix(&self) -> glam::Mat4 {
        let mut transform = glam::Mat4::from_translation(glam::vec3(0.0, 0.0, -self.distance));
        transform = self.view_matrix_xz * transform;
        let pitch_dir_mat = glam::Mat4::from_axis_angle(transform.x_axis.xyz(), self.pitch_angle);
        transform = pitch_dir_mat * transform;
        let offset_mat = glam::Mat4::from_translation(self.position_offset.xyz());
        transform = transform * offset_mat;
        transform
    }
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            view_matrix_xz: glam::Mat4::IDENTITY,
            position_offset: glam::Vec4::new(0.25, 0.85, 0.0, 0.0),
            pitch_angle: 10f32.to_radians(),
            distance: 1.5,
        }
    }
}

/// ## Projection Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub glam::Mat4);

impl Projection {
    /// 새로운 원근 투영 변환 행렬을 생성합니다.
    pub fn perspective(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        Self(glam::Mat4::perspective_lh(
            fov_y_radians,
            aspect_ratio,
            z_near,
            z_far,
        ))
    }
}
