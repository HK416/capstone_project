//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

use crate::components::BigEndian;

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
    /// 사망 상태
    Dead = 5,
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
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(ActionState::Idle),
            1 => Some(ActionState::Aiming),
            2 => Some(ActionState::AimAt),
            3 => Some(ActionState::AimOff),
            4 => Some(ActionState::Attack),
            5 => Some(ActionState::Dead),
            6 => Some(ActionState::Reload),
            7 => Some(ActionState::Skill),
            8 => Some(ActionState::Callsign),
            9 => Some(ActionState::VictoryStart),
            10 => Some(ActionState::VictoryEnd),
            _ => None,
        }
    }
}

/// 캐릭터의 움직임 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementState {
    /// 아무 움직임도 없는 상태
    #[default]
    Idle = 0,
    /// 움직이는 중인 상태
    Moving = 1,
    /// 움직였다 멈춘 상태
    MoveToEnd = 2,
    /// 제자리에서 점프하는 상태
    InPlaceJumping = 3,
    /// 제자리에서 착지하는 상태
    InPlaceLanding = 4,
    /// 움직이면서 점프하는 상태
    MovingJumping = 5,
    /// 움직이면서 착지하는 상태
    MovingLanding = 6,
}

impl MovementState {
    /// 주어진 정수로 부터 `MovementState`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(MovementState::Idle),
            1 => Some(MovementState::Moving),
            2 => Some(MovementState::MoveToEnd),
            3 => Some(MovementState::InPlaceJumping),
            4 => Some(MovementState::InPlaceLanding),
            5 => Some(MovementState::MovingJumping),
            6 => Some(MovementState::MovingLanding),
            _ => None,
        }
    }
}

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
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
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
/// - action_state   | 3bit | 행동 상태
/// - movement_state | 3bit | 움직임 상태
/// - view_state     | 2bit | 시야 상태
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerStateData(u8);

impl PlayerStateData {
    const ACTION_BIT_MASK: u8 = 0x7;
    const ACTION_SHIFT: usize = 0;
    const MOVEMENT_BIT_MASK: u8 = 0x7;
    const MOVEMENT_SHIFT: usize = 3;
    const VIEW_BIT_MASK: u8 = 0x3;
    const VIEW_SHIFT: usize = 6;

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
    pub fn with_action_state(mut self, state: ActionState) -> Self {
        self.0 &= !(Self::ACTION_BIT_MASK << Self::ACTION_SHIFT); // 기존 값 지우기
        self.0 |= (state as u8) << Self::ACTION_SHIFT; // 값 덮어쓰기
        self
    }

    /// 움직임 상태를 반환합니다.
    pub fn movement_state(&self) -> MovementState {
        let val = (self.0 >> Self::MOVEMENT_SHIFT) & Self::MOVEMENT_BIT_MASK;
        MovementState::new(val).unwrap_or_default()
    }

    /// 움직임 상태를 설정합니다.
    pub fn with_movement_state(mut self, state: MovementState) -> Self {
        self.0 &= !(Self::MOVEMENT_BIT_MASK << Self::MOVEMENT_SHIFT); // 기존 값 지우기
        self.0 |= (state as u8) << Self::MOVEMENT_SHIFT; // 값 덮어쓰기
        self
    }

    /// 시야 상태를 반환합니다.
    pub fn view_state(&self) -> ViewState {
        let val = (self.0 >> Self::VIEW_SHIFT) & Self::VIEW_BIT_MASK;
        ViewState::new(val).unwrap_or_default()
    }

    /// 시야 상태를 설정합니다.
    pub fn with_view_state(mut self, state: ViewState) -> Self {
        self.0 &= !(Self::VIEW_BIT_MASK << Self::VIEW_SHIFT); // 기존 값 지우기
        self.0 |= (state as u8) << Self::VIEW_SHIFT; // 값 덮어쓰기
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
    #[should_panic]
    fn test_creation_movement_state() {
        MovementState::new(15).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_creation_view_state() {
        ViewState::new(6).unwrap();
    }

    #[test]
    fn test_player_state_data() {
        let origin = PlayerStateData::new()
            .with_action_state(ActionState::Dead)
            .with_movement_state(MovementState::MoveToEnd)
            .with_view_state(ViewState::ZoomIn);
        let bytes = origin.to_big_endian_bytes();
        let other = PlayerStateData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
