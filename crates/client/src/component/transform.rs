use glam::Vec4Swizzles;

/// ## To Parent Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToParentTrans(pub glam::Mat4);

impl ToParentTrans {
    /// 로컬 변환 행렬의 위치를 주어진 거리만큼 이동시킵니다.
    pub fn translate_world(&mut self, distance: glam::Vec4) {
        debug_assert_eq!(distance.w, 0.0);
        self.0.w_axis += distance;
    }

    /// 로컬 변환 행렬의 위치를 반환합니다.
    pub fn get_translation(&self) -> glam::Vec3 {
        self.0.w_axis.xyz()
    }

    /// 로컬 변환 행렬의 앞쪽 방향 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec3 {
        self.0.z_axis.xyz().normalize()
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
    pub fn get_translation(&self) -> glam::Vec3 {
        self.0.w_axis.xyz()
    }

    /// 월드 변환 행렬의 위쪽 방향 벡터를 반환합니다.
    pub fn get_up_vector(&self) -> glam::Vec3 {
        self.0.y_axis.xyz().normalize()
    }

    /// 월드 변환 행렬의 앞쪽 방향 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec3 {
        self.0.z_axis.xyz().normalize()
    }

    /// 월드 변환 행렬의 뷰 변환 행렬을 반환합니다.
    pub fn to_view_trans(&self) -> glam::Mat4 {
        glam::Mat4::look_to_lh(
            self.get_translation(),
            self.get_look_vector(),
            self.get_up_vector(),
        )
    }
}

impl Default for WorldTransform {
    fn default() -> Self {
        Self(glam::Mat4::IDENTITY)
    }
}
