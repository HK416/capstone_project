use mod_network::components::{BulletKind, Team, UserId};

/// 서버에서 관리하는 총알 데이터
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Bullet {
    /// 총알의 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 총알의 월드 공간 방향
    pub rotation: glam::Quat,
    /// 총알의 월드 공간 속도
    pub velocity: glam::Vec3A,
    /// 총알의 남은 사거리
    pub remaining_distance: f32,
    /// 총알의 반지름
    pub radius: f32,
    /// 총알을 발사한 사용자 식별자
    pub shooter_id: UserId,
    /// 총알을 발사한 사용자의 팀
    pub shooter_team: Team,
    /// 총알의 종류
    pub bullet_kind: BulletKind,
}

impl Bullet {
    /// 새로운 총알 데이터를 생성합니다.
    pub const fn new(
        translation: glam::Vec3A,
        rotation: glam::Quat,
        velocity: glam::Vec3A,
        remaining_distance: f32,
        radius: f32,
        shooter_id: UserId,
        shooter_team: Team,
        bullet_kind: BulletKind,
    ) -> Self {
        Self {
            translation,
            rotation,
            velocity,
            remaining_distance,
            radius,
            shooter_id,
            shooter_team,
            bullet_kind,
        }
    }
}
