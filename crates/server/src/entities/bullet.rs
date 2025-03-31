use mod_network::components::{Bullet, BulletKind, ObjectId, Team, UserId};

/// 서버에서 관리하는 총알 데이터
#[derive(Debug, Clone)]
pub struct BulletObject {
    /// 총알의 오브젝트 식별자
    pub object_id: ObjectId,
    /// 총알을 발사한 사용자 식별자
    pub shooter_id: UserId,
    /// 총알을 발사한 사용자의 팀
    pub shooter_team: Team,
    /// 총알의 종류
    pub bullet_kind: BulletKind,
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
}

impl BulletObject {
    pub fn as_bullet(&self) -> Bullet {
        Bullet {
            object_id: self.object_id,
            shooter_id: self.shooter_id,
            bullet_kind: self.bullet_kind,
            translation: self.translation.into(),
            rotation: self.rotation.into(),
            velocity: self.velocity.into(),
            remaining_distance: self.remaining_distance,
        }
    }
}
