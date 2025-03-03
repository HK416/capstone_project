use bitflags::bitflags;

use super::BigEndian;

/// `ControllerState` 상태 수 입니다.
pub const NUM_CONTROLLER_STATES: usize = 9;

/// 플레이어 방향 컨트롤러의 상태를 나타냅니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControllerState {
    Idle = 0,
    MovingLeft = 1,
    MovingRight = 2,
    MovingForward = 3,
    MovingBackward = 4,
    MovingLeftForward = 5,
    MovingRightForward = 6,
    MovingLeftBackward = 7,
    MovingRightBackward = 8,
}

impl Default for ControllerState {
    fn default() -> Self {
        ControllerState::Idle
    }
}

/// 게임에서 클라이언트의 컨트롤러 눌림 상태를 나타냅니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameInputFlags(pub u16);

bitflags! {
    impl GameInputFlags : u16 {
        const Left = 0x0001;
        const Right = 0x0002;
        const Forward = 0x0004;
        const Backward = 0x0008;
        const Aiming = 0x0010;
        const Attack = 0x0020;
        const Skill = 0x0040;
        const ExSkill = 0x0080;
        const Jump = 0x0100;
        const Reload = 0x0200;
        const Status = 0x0400;
        const Emotion1 = 0x1000;
        const Emotion2 = 0x2000;
        const Emotion3 = 0x4000;
        const Emotion4 = 0x8000;
    }
}

impl GameInputFlags {
    /// 새로운 `InGameInputFlags`를 생성합니다.
    pub const fn new() -> Self {
        Self::empty()
    }

    /// `InGameInputFlags`를 `ControllerState`로 변환합니다.
    pub fn as_state(&self) -> ControllerState {
        const STATES: [ControllerState; 16] = [
            ControllerState::Idle,
            ControllerState::MovingLeft,
            ControllerState::MovingRight,
            ControllerState::Idle,
            ControllerState::MovingForward,
            ControllerState::MovingLeftForward,
            ControllerState::MovingRightForward,
            ControllerState::MovingForward,
            ControllerState::MovingBackward,
            ControllerState::MovingLeftBackward,
            ControllerState::MovingRightBackward,
            ControllerState::MovingBackward,
            ControllerState::Idle,
            ControllerState::MovingLeft,
            ControllerState::MovingRight,
            ControllerState::Idle,
        ];
        STATES[(self.bits() & 0x000F) as usize]
    }
}

impl BigEndian for GameInputFlags {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for GameInputFlags {
    fn default() -> Self {
        Self::empty()
    }
}
