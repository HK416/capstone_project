use bitflags::bitflags;

use crate::components::{BigEndian, ControllerState};

/// 게임 입력 상태를 나타내는 16바이트 크기의 비트 플래그입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameInputBits(u16);

bitflags! {
    impl GameInputBits : u16 {
        const Left = 0x0001;
        const Right = 0x0002;
        const Forward = 0x0004;
        const Backward = 0x0008;
        const Aiming = 0x0010;
        const Attack = 0x0020;
        const Skill = 0x0040;
        const Jump = 0x0080;
        const Reload = 0x0100;
        const Status = 0x0200;
        const Emotion = 0x0400;
    }
}

impl GameInputBits {
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

impl BigEndian for GameInputBits {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for GameInputBits {
    fn default() -> Self {
        Self::empty()
    }
}
