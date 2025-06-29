use mod_network::components::{
    ActionStateTimer, MovementStateTimer, PlayerStateData, ViewStateTimer,
};

/// 최대 스냅샷의 수
pub const MAX_SNAPSHOTS: usize = 12;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    /// 액션 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 시야 상태 타이머
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 상태
    pub player_states: PlayerStateData,
    /// 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 월드 공간 방향
    pub rotation: glam::Quat,
    /// 월드 공간 속도
    pub velocity: glam::Vec3A,
}

impl Snapshot {
    /// 새로운 스냅샷 데이터를 생성합니다.
    pub const fn new(
        action_state_timer: ActionStateTimer,
        movement_state_timer: MovementStateTimer,
        view_state_timer: ViewStateTimer,
        player_states: PlayerStateData,
        translation: glam::Vec3A,
        rotation: glam::Quat,
        velocity: glam::Vec3A,
    ) -> Self {
        Self {
            action_state_timer,
            movement_state_timer,
            view_state_timer,
            player_states,
            translation,
            rotation,
            velocity,
        }
    }
}
