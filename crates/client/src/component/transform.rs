use glam::Vec4Swizzles;

/// ## To Parent Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToParentTrans(pub glam::Mat4);

impl ToParentTrans {
    /// 로컬 변환 행렬의 z축을 주어진 방향과 같도록 합니다.
    pub fn look_to(&mut self, look: glam::Vec4, up: glam::Vec4) {
        // 세 축의 방향 벡터를 계산합니다.
        let z_axis = glam::Vec3A::from_vec4(look).normalize_or(glam::Vec3A::Z);
        let y_axis = glam::Vec3A::from_vec4(up).normalize_or(glam::Vec3A::Y);
        let x_axis = y_axis.cross(z_axis);
        let y_axis = z_axis.cross(x_axis);

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

    /// 월드 변환 행렬의 앞쪽 방향 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec4 {
        self.0.z_axis.normalize_or(glam::Vec4::Z)
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
    pub fn get_translation(&self) -> glam::Vec4 {
        self.0.w_axis
    }

    /// 월드 변환 행렬의 회전 데이터를 가져옵니다.
    pub fn get_rotation(&self) -> glam::Quat {
        glam::Quat::from_mat4(&self.0).normalize()
    }

    /// 월드 변환 행렬의 위쪽 방향 벡터를 반환합니다.
    pub fn get_up_vector(&self) -> glam::Vec4 {
        self.0.y_axis.normalize_or(glam::Vec4::Y)
    }

    /// 월드 변환 행렬의 앞쪽 방향 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec4 {
        self.0.z_axis.normalize_or(glam::Vec4::Z)
    }

    /// 월드 변환 행렬의 뷰 변환 행렬을 반환합니다.
    pub fn to_view_trans(&self) -> glam::Mat4 {
        glam::Mat4::look_to_lh(
            self.get_translation().xyz(),
            self.get_look_vector().xyz(),
            self.get_up_vector().xyz(),
        )
    }
}

impl Default for WorldTransform {
    fn default() -> Self {
        Self(glam::Mat4::IDENTITY)
    }
}
