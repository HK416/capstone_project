/// Directional Light의 데이터를 저장합니다.
#[derive(Debug, Clone, Copy)]
pub struct DirectionLight {
    pub direction: glam::Quat,
    pub color: [f32; 3],
}

impl DirectionLight {
    /// 위쪽 방향을 가리키는 벡터를 반환합니다.
    pub fn get_up_vector(&self) -> glam::Vec3A {
        self.direction.mul_vec3a(glam::Vec3A::Y)
    }

    /// 앞쪽 방향을 가리키는 벡터를 반환합니다.
    pub fn get_look_vector(&self) -> glam::Vec3A {
        self.direction.mul_vec3a(glam::Vec3A::Z)
    }
}

impl Default for DirectionLight {
    fn default() -> Self {
        Self {
            direction: glam::Quat::IDENTITY,
            color: [1.0; 3],
        }
    }
}
