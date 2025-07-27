//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

use crate::components::BigEndian;

/// 행동 상태의 개수입니다.
pub const NUM_ACTION_STATES: usize = 8;

/// 플레이어 행동 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionState {
    /// 아무 것도 하지 않는 상태
    #[default]
    Idle = 0,
    /// 조준하고 있는 동작 상태
    Aiming = 1,
    /// 조준을 시작하는 동작 상태
    AimAt = 2,
    /// 조준을 해제하는 동작 상태
    AimOff = 3,
    /// 공격 동작 상태
    Attack = 4,
    /// 행동 불능 상태
    Retreat = 5,
    /// 재장전 상태
    Reload = 6,
    /// 일반 스킬을 사용하는 상태
    Skill = 7,
    /// 게임 시작 전 콜 싸인 모션 상태
    Callsign = 8,
    /// 승리 시작 모션 상태
    VictoryStart = 9,
    /// 승리 끝 모션 상태
    VictoryEnd = 10,
}

impl ActionState {
    /// 주어진 정수로부터 `ActionState`를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(ActionState::Idle),
            1 => Some(ActionState::Aiming),
            2 => Some(ActionState::AimAt),
            3 => Some(ActionState::AimOff),
            4 => Some(ActionState::Attack),
            5 => Some(ActionState::Retreat),
            6 => Some(ActionState::Reload),
            7 => Some(ActionState::Skill),
            8 => Some(ActionState::Callsign),
            9 => Some(ActionState::VictoryStart),
            10 => Some(ActionState::VictoryEnd),
            _ => None,
        }
    }

    pub fn is_next_state(self, state: Self) -> bool {
        match (self, state) {
            (ActionState::Idle, ActionState::Idle)
            | (ActionState::Idle, ActionState::AimAt)
            | (ActionState::Idle, ActionState::AimOff)
            | (ActionState::Idle, ActionState::Attack)
            | (ActionState::Idle, ActionState::Retreat)
            | (ActionState::Idle, ActionState::Reload)
            | (ActionState::Idle, ActionState::Skill)
            | (ActionState::Aiming, ActionState::Aiming)
            | (ActionState::Aiming, ActionState::AimAt)
            | (ActionState::Aiming, ActionState::AimOff)
            | (ActionState::Aiming, ActionState::Attack)
            | (ActionState::Aiming, ActionState::Retreat)
            | (ActionState::Aiming, ActionState::Skill)
            | (ActionState::AimAt, ActionState::Idle)
            | (ActionState::AimAt, ActionState::Aiming)
            | (ActionState::AimAt, ActionState::AimAt)
            | (ActionState::AimAt, ActionState::AimOff)
            | (ActionState::AimAt, ActionState::Retreat)
            | (ActionState::AimOff, ActionState::Idle)
            | (ActionState::AimOff, ActionState::Aiming)
            | (ActionState::AimOff, ActionState::AimAt)
            | (ActionState::AimOff, ActionState::AimOff)
            | (ActionState::AimOff, ActionState::Retreat)
            | (ActionState::Attack, ActionState::Idle)
            | (ActionState::Attack, ActionState::Aiming)
            | (ActionState::Attack, ActionState::AimAt)
            | (ActionState::Attack, ActionState::AimOff)
            | (ActionState::Attack, ActionState::Attack)
            | (ActionState::Attack, ActionState::Retreat)
            | (ActionState::Attack, ActionState::Reload)
            | (ActionState::Attack, ActionState::Skill)
            | (ActionState::Retreat, ActionState::Idle)
            | (ActionState::Retreat, ActionState::Aiming)
            | (ActionState::Retreat, ActionState::AimAt)
            | (ActionState::Retreat, ActionState::AimOff)
            | (ActionState::Retreat, ActionState::Attack)
            | (ActionState::Retreat, ActionState::Retreat)
            | (ActionState::Retreat, ActionState::Reload)
            | (ActionState::Reload, ActionState::Idle)
            | (ActionState::Reload, ActionState::Aiming)
            | (ActionState::Reload, ActionState::AimAt)
            | (ActionState::Reload, ActionState::AimOff)
            | (ActionState::Reload, ActionState::Attack)
            | (ActionState::Reload, ActionState::Retreat)
            | (ActionState::Reload, ActionState::Reload)
            | (ActionState::Reload, ActionState::Skill)
            | (ActionState::Skill, ActionState::Idle)
            | (ActionState::Skill, ActionState::Aiming)
            | (ActionState::Skill, ActionState::AimAt)
            | (ActionState::Skill, ActionState::AimOff)
            | (ActionState::Skill, ActionState::Attack)
            | (ActionState::Skill, ActionState::Retreat)
            | (ActionState::Skill, ActionState::Reload)
            | (ActionState::Skill, ActionState::Skill) => true,
            _ => false,
        }
    }
}

/// 움직임 상태의 개수입니다.
pub const NUM_MOVEMENT_STATES: usize = 5;

/// 캐릭터의 움직임 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementState {
    /// 아무 움직임도 없는 상태
    #[default]
    Idle = 0,
    /// 움직이는 중인 상태
    Moving = 1,
    /// 움직이다가 멈춘 상태
    MoveToEnd = 2,
    /// 점프하는 상태
    Jumping = 3,
    /// 착지하는 상태
    Landing = 4,
}

impl MovementState {
    /// 주어진 정수로 부터 `MovementState`를 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(MovementState::Idle),
            1 => Some(MovementState::Moving),
            2 => Some(MovementState::MoveToEnd),
            3 => Some(MovementState::Jumping),
            4 => Some(MovementState::Landing),
            _ => None,
        }
    }

    pub fn is_next_state(self, state: Self) -> bool {
        match (self, state) {
            (MovementState::Idle, MovementState::Idle)
            | (MovementState::Idle, MovementState::Moving)
            | (MovementState::Idle, MovementState::MoveToEnd)
            | (MovementState::Idle, MovementState::Jumping)
            | (MovementState::Moving, MovementState::Idle)
            | (MovementState::Moving, MovementState::Moving)
            | (MovementState::Moving, MovementState::MoveToEnd)
            | (MovementState::Moving, MovementState::Jumping)
            | (MovementState::MoveToEnd, MovementState::Idle)
            | (MovementState::MoveToEnd, MovementState::Moving)
            | (MovementState::MoveToEnd, MovementState::MoveToEnd)
            | (MovementState::MoveToEnd, MovementState::Jumping)
            | (MovementState::Jumping, MovementState::Idle)
            | (MovementState::Jumping, MovementState::Moving)
            | (MovementState::Jumping, MovementState::MoveToEnd)
            | (MovementState::Jumping, MovementState::Jumping)
            | (MovementState::Jumping, MovementState::Landing)
            | (MovementState::Landing, MovementState::Idle)
            | (MovementState::Landing, MovementState::Moving)
            | (MovementState::Landing, MovementState::MoveToEnd)
            | (MovementState::Landing, MovementState::Jumping)
            | (MovementState::Landing, MovementState::Landing) => true,
            _ => false,
        }
    }
}

/// 카메라 시야 상태의 개수입니다.
pub const NUM_VIEW_STATES: usize = 4;

/// 플레이어 시야 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ViewState {
    /// 아무 것도 하지 않는 상태
    #[default]
    Idle = 0,
    /// 조준을 준비하는 상태
    ZoomIn = 1,
    /// 조준을 해제하는 상태
    ZoomOut = 2,
    /// 조준하는 상태
    Aiming = 3,
}

impl ViewState {
    /// 주어진 정수로 부터 `ViewState`를 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(ViewState::Idle),
            1 => Some(ViewState::ZoomIn),
            2 => Some(ViewState::ZoomOut),
            3 => Some(ViewState::Aiming),
            _ => None,
        }
    }
}

/// 플레이어 행동 상태 데이터입니다.
///
/// 아래와 같은 데이터가 포함되어있습니다.
/// - action_state   | 4bit | 행동 상태
/// - movement_state | 4bit | 움직임 상태
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerStateData(u8);

impl PlayerStateData {
    const ACTION_BIT_MASK: u8 = 0xF;
    const ACTION_SHIFT: usize = 0;
    const MOVEMENT_BIT_MASK: u8 = 0xF;
    const MOVEMENT_SHIFT: usize = 4;

    /// 새로운 플레이어 행동 상태 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 행동 상태를 반환합니다.
    pub fn action_state(&self) -> ActionState {
        let val = (self.0 >> Self::ACTION_SHIFT) & Self::ACTION_BIT_MASK;
        ActionState::new(val).unwrap_or_default()
    }

    /// 행동 상태를 설정합니다.
    pub const fn set_action_state(&mut self, state: ActionState) {
        self.0 &= !(Self::ACTION_BIT_MASK << Self::ACTION_SHIFT); // 기존 값 지우기
        self.0 |= ((state as u8) & Self::ACTION_BIT_MASK) << Self::ACTION_SHIFT;
        // 값 덮어쓰기
    }

    /// 행동 상태를 설정합니다.
    pub const fn with_action_state(mut self, state: ActionState) -> Self {
        self.set_action_state(state);
        self
    }

    /// 움직임 상태를 반환합니다.
    pub fn movement_state(&self) -> MovementState {
        let val = (self.0 >> Self::MOVEMENT_SHIFT) & Self::MOVEMENT_BIT_MASK;
        MovementState::new(val).unwrap_or_default()
    }

    /// 움직임 상태를 설정합니다.
    pub const fn set_movement_state(&mut self, state: MovementState) {
        self.0 &= !(Self::MOVEMENT_BIT_MASK << Self::MOVEMENT_SHIFT); // 기존 값 지우기
        self.0 |= ((state as u8) & Self::MOVEMENT_BIT_MASK) << Self::MOVEMENT_SHIFT;
        // 값 덮어쓰기
    }

    /// 움직임 상태를 설정합니다.
    pub const fn with_movement_state(mut self, state: MovementState) -> Self {
        self.set_movement_state(state);
        self
    }
}

impl BigEndian for PlayerStateData {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u8::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for PlayerStateData {
    fn default() -> Self {
        Self(0x00)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_action_state() {
        ActionState::new(100).unwrap();
    }

    #[test]
    fn test_creation_action_state_idle() {
        let val = ActionState::Idle as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Idle, state);
    }

    #[test]
    fn test_creation_action_state_aiming() {
        let val = ActionState::Aiming as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Aiming, state);
    }

    #[test]
    fn test_creation_action_state_aim_at() {
        let val = ActionState::AimAt as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::AimAt, state);
    }

    #[test]
    fn test_creation_action_state_aim_off() {
        let val = ActionState::AimOff as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::AimOff, state);
    }

    #[test]
    fn test_creation_action_state_attack() {
        let val = ActionState::Attack as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Attack, state);
    }

    #[test]
    fn test_creation_action_state_dead() {
        let val = ActionState::Retreat as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Retreat, state);
    }

    #[test]
    fn test_creation_action_state_reload() {
        let val = ActionState::Reload as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Reload, state);
    }

    #[test]
    fn test_creation_action_state_skill() {
        let val = ActionState::Skill as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Skill, state);
    }

    #[test]
    fn test_creation_action_state_callsign() {
        let val = ActionState::Callsign as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::Callsign, state);
    }

    #[test]
    fn test_creation_action_state_victory_start() {
        let val = ActionState::VictoryStart as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::VictoryStart, state);
    }

    #[test]
    fn test_creation_action_state_victory_end() {
        let val = ActionState::VictoryEnd as u8;
        let state = ActionState::new(val).unwrap();
        assert_eq!(ActionState::VictoryEnd, state);
    }

    #[test]
    #[should_panic]
    fn test_creation_movement_state() {
        MovementState::new(15).unwrap();
    }

    #[test]
    fn test_creation_movement_state_idle() {
        let val = MovementState::Idle as u8;
        let state = MovementState::new(val).unwrap();
        assert_eq!(MovementState::Idle, state);
    }

    #[test]
    fn test_creation_movement_state_moving() {
        let val = MovementState::Moving as u8;
        let state = MovementState::new(val).unwrap();
        assert_eq!(MovementState::Moving, state);
    }

    #[test]
    fn test_creation_movement_state_jumping() {
        let val = MovementState::Jumping as u8;
        let state = MovementState::new(val).unwrap();
        assert_eq!(MovementState::Jumping, state);
    }

    #[test]
    fn test_creation_movement_state_landing() {
        let val = MovementState::Landing as u8;
        let state = MovementState::new(val).unwrap();
        assert_eq!(MovementState::Landing, state);
    }

    #[test]
    #[should_panic]
    fn test_creation_view_state() {
        ViewState::new(6).unwrap();
    }

    #[test]
    fn test_creation_view_state_idle() {
        let val = ViewState::Idle as u8;
        let state = ViewState::new(val).unwrap();
        assert_eq!(ViewState::Idle, state);
    }

    #[test]
    fn test_creation_view_state_zoom_in() {
        let val = ViewState::ZoomIn as u8;
        let state = ViewState::new(val).unwrap();
        assert_eq!(ViewState::ZoomIn, state);
    }

    #[test]
    fn test_creation_view_state_zoom_out() {
        let val = ViewState::ZoomOut as u8;
        let state = ViewState::new(val).unwrap();
        assert_eq!(ViewState::ZoomOut, state);
    }

    #[test]
    fn test_creation_view_state_aiming() {
        let val = ViewState::Aiming as u8;
        let state = ViewState::new(val).unwrap();
        assert_eq!(ViewState::Aiming, state);
    }

    #[test]
    fn test_player_state_data() {
        let origin = PlayerStateData::new()
            .with_action_state(ActionState::Retreat)
            .with_movement_state(MovementState::Jumping);
        let bytes = origin.to_big_endian_bytes();
        let other = PlayerStateData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
