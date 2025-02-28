use mod_network::components::{Bullet, BulletKind, Epoch, ObjectId, Player, StageKind, UserId};

/// 서버에서 관리하는 총알 데이터
#[derive(Debug, Clone)]
pub struct ServerBullet {
    /// 총알의 오브젝트 식별자
    pub object_id: ObjectId,
    /// 총알을 발사한 사용자 식별자
    pub shooter_id: UserId,
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
}

/// 특정 시점의 스테이지 정보를 저장합니다.
#[derive(Debug, Clone)]
pub struct StageSnapshot {
    pub epoch: Epoch,
    pub total_time_sec: f32,
    pub stage_kind: StageKind,
    pub players: Vec<Player>,
    pub bullets: Vec<Bullet>,
}

impl Default for StageSnapshot {
    fn default() -> Self {
        Self {
            epoch: Epoch::new(0),
            total_time_sec: 0.0,
            stage_kind: StageKind::default(),
            players: Vec::default(),
            bullets: Vec::default(),
        }
    }
}
