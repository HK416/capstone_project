/// ## To Parent Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToParentTrans(pub glam::Mat4);

impl ToParentTrans {
    /// 로컬 변환 행렬의 z축을 주어진 방향과 같도록 합니다.
    pub fn look_to<PZ, PY>(&mut self, look: PZ, up: PY)
    where
        PZ: Into<glam::Vec3A>,
        PY: Into<glam::Vec3A>,
    {
        // 세 축의 방향 벡터를 계산합니다.
        let mut z_axis: glam::Vec3A = look.into();
        z_axis = z_axis.normalize_or(glam::Vec3A::Z);
        let mut y_axis: glam::Vec3A = up.into();
        y_axis = y_axis.normalize_or(glam::Vec3A::Y);
        let x_axis = y_axis.cross(z_axis);
        y_axis = z_axis.cross(x_axis);

        // 회전 쿼터니언을 생성합니다.
        let rotation = glam::Quat::from_mat3a(&glam::Mat3A::from_cols(x_axis, y_axis, z_axis));

        // 현재 변환 행렬로 부터 스케일과 위치를 가져옵니다.
        let (scale, _, translation) = self.0.to_scale_rotation_translation();

        // 새로운 변환 행렬을 적용합니다.
        self.0 = glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// 로컬 변환 행렬의 방향과 위치를 설정합니다.
    pub fn set_rotation_translation(&mut self, rotation: glam::Quat, translation: glam::Vec3) {
        let (scale, _, _) = self.0.to_scale_rotation_translation();
        self.0 = glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// 로컬 변환 행렬의 위치를 설정합니다.
    pub fn set_translation(&mut self, translation: glam::Vec3) {
        let (scale, rotation, _) = self.0.to_scale_rotation_translation();
        self.0 = glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// 로컬 변환 행렬의 위치를 반환합니다.
    pub fn get_translation(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.w_axis)
    }

    /// 로컬 변환 행렬의 앞쪽 방향 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.z_axis).normalize_or(glam::Vec3A::Z)
    }

    /// 로컬 변환 행렬의 오른쪽 방향 벡터를 반환합니다.
    pub fn get_right_vector(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.x_axis).normalize_or(glam::Vec3A::X)
    }
}

impl Default for ToParentTrans {
    fn default() -> Self {
        Self(glam::Mat4::IDENTITY)
    }
}

/// ## World Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldTransform(pub glam::Mat4);

impl WorldTransform {
    /// 월드 변환 행렬의 위치를 반환합니다.
    pub fn get_translation(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.w_axis)
    }

    /// 월드 변환 행렬의 회전 데이터를 가져옵니다.
    pub fn get_rotation(&self) -> glam::Quat {
        glam::Quat::from_mat4(&self.0).normalize()
    }

    /// 월드 변환 행렬의 오른쪽 방향 벡터를 반환합니다.
    pub fn get_right_vector(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.x_axis).normalize_or(glam::Vec3A::X)
    }

    /// 월드 변환 행렬의 위쪽 방향 벡터를 반환합니다.
    pub fn get_up_vector(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.y_axis).normalize_or(glam::Vec3A::Y)
    }

    /// 월드 변환 행렬의 앞쪽 방향 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec3A {
        glam::Vec3A::from_vec4(self.0.z_axis).normalize_or(glam::Vec3A::Z)
    }

    /// 월드 변환 행렬의 뷰 변환 행렬을 반환합니다.
    pub fn to_view_trans(&self) -> glam::Mat4 {
        glam::Mat4::look_to_lh(
            self.get_translation().into(),
            self.get_look_vector().into(),
            self.get_up_vector().into(),
        )
    }

    /// 월드 좌표계의 벡터를 모델 좌표계로 변환합니다.
    #[allow(dead_code)]
    pub fn world_to_model_vector3a(&self, v: glam::Vec3A) -> glam::Vec3A {
        let (scale, rotation, _) = self.0.to_scale_rotation_translation();
        let rotation_transposed = glam::Mat4::from_quat(rotation).transpose();
        let inverse_scale = glam::Mat4::from_scale(1.0 / scale);
        (rotation_transposed * inverse_scale).transform_vector3a(v)
    }
}

impl Default for WorldTransform {
    fn default() -> Self {
        Self(glam::Mat4::IDENTITY)
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
