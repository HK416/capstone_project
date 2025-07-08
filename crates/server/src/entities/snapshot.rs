//! 게임 월드의 스냅샷과 관련된 코드를 관리합니다.
//!

use mod_network::components::{
    ActionState, ActionStateTimer, HeldInput, InputStateTimer, LatLon, MovementState,
    MovementStateTimer, MovingDirection, Velocity,
};

/// 최대 플레이어 스냅샷 데이터의 개수입니다.
pub const MAX_PLAYER_SNAPSHOTS: usize = 127;

/// 플레이어 스냅샷 데이터입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    /// 플레이 경과 시간
    pub play_elapsed_time_ms: u32,
    /// 행동 상태
    pub action_state: ActionState,
    /// 움직임 상태
    pub movement_state: MovementState,
    /// 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 각도
    pub latlon: LatLon,
    /// 플레이어 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 플레이어 월드 공간 방향
    pub rotation: glam::Quat,
    /// 플레이어 월드 공간 이동 속도
    pub velocity: Velocity,
    /// 플레이어 월드 공간 이동 방향
    pub direction: MovingDirection,
    /// 입력 상태 타이머
    pub input_timer: InputStateTimer,
    /// 게임 입력 비트 플래그
    pub held_input: HeldInput,
    /// 무적 여부
    pub is_invincible: bool,
    /// 지면을 밟고 있는 여부
    pub is_grounded: bool,
}
