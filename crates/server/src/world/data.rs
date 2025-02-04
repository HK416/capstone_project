use mod_network::components::{
    ActionState, ActionStateTimer, Bullet, BulletKind, CharacterKind, ClientId, Epoch, HealthPoint,
    LatLon, MovementState, MovementStateTimer, ObjectId, Player, StageKind, ViewState,
    ViewStateTimer,
};

/// 서버에서 관리하는 플레이어 데이터
#[derive(Debug, Clone)]
pub struct ServerPlayer {
    /// 플레이어의 시대
    pub epoch: Epoch,
    /// 플레이어 오브젝트 식별자
    pub object_id: ObjectId,
    /// 플레이어 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 플레이어 캐릭터 체력
    pub health_point: HealthPoint,
    /// 플레이어 캐릭터의 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 플레이어 캐릭터의 월드 공간 방향 (캐릭터가 움직이는 방향과 다를 수 있음)
    pub rotation: glam::Quat,
    /// 플레이어 캐릭터의 월드 공간 속도
    pub velocity: glam::Vec3A,
    /// 플레이어 움직임 방향
    pub direction: glam::Vec3A,
    /// 플레이어 캐릭터 행동 상태
    pub action_state: ActionState,
    /// 플레이어 캐릭터 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터 움직임 상태
    pub movement_state: MovementState,
    /// 플레이어 캐릭터 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태
    pub view_state: ViewState,
    /// 플레이어 카메라 상태 타미어
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터 중심으로 바라보는 방향
    pub view_rotation: LatLon,
    /// 총알을 다시 발사할 수 있는 지연 시간
    pub shot_cool_time: f32,
}

/// 서버에서 관리하는 총알 데이터
#[derive(Debug, Clone)]
pub struct ServerBullet {
    /// 총알의 오브젝트 식별자
    pub object_id: ObjectId,
    /// 총알을 발사한 클라이언트 식별자
    pub shooter_id: ClientId,
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
