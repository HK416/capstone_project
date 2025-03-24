use mod_physics::object3d::Capsule;

/// 게임 월드 공간 속성을 저장합니다.
#[derive(Debug, Clone)]
pub struct TransformComponent {
    /// 플레이어 캐릭터의 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 플레이어 캐릭터가 바라보는 월드 공간 방향
    /// ※ 플레이어 캐릭터가 움직이는 방향과 다를 수 있음
    pub rotation: glam::Quat,
    /// 플레이어가 움직이는 방향
    pub direction: glam::Vec3A,
    /// 플레이어의 속도
    pub velocity: glam::Vec3A,
    /// 플레이어 충돌체입니다.
    pub collider: Capsule,
}

impl Default for TransformComponent {
    fn default() -> Self {
        Self {
            translation: glam::Vec3A::ZERO,
            rotation: glam::Quat::IDENTITY,
            direction: glam::Vec3A::Z,
            velocity: glam::Vec3A::ZERO,
            collider: Capsule {
                center: glam::Vec3::ZERO,
                height: 0.0,
                radius: 0.0,
            },
        }
    }
}
