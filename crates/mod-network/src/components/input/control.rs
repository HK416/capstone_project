use serde::{Deserialize, Serialize};

use crate::components::GameInputBits;

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

/// 게임 입력의 수 입니다.
pub const NUM_GAME_INPUTS: usize = 12;

/// 게임 입력 목록입니다.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum GameInput {
    Left = 0x0001,
    Right = 0x0002,
    Forward = 0x0004,
    Backward = 0x0008,
    Aiming = 0x0010,
    Attack = 0x0020,
    Skill = 0x0040,
    ExSkill = 0x0080,
    Jump = 0x0100,
    Reload = 0x0200,
    Status = 0x0400,
    Emotion = 0x0800,
}

impl GameInput {
    /// `GameInputBits`로 부터 `GameInput`을 생성합니다.
    /// `GameInput`을 생성할 수 없는 경우 `None`을 반환합니다.
    pub fn from_bits(flag: GameInputBits) -> Option<Self> {
        match flag.bits() {
            0x0001 => Some(Self::Left),
            0x0002 => Some(Self::Right),
            0x0004 => Some(Self::Forward),
            0x0008 => Some(Self::Backward),
            0x0010 => Some(Self::Aiming),
            0x0020 => Some(Self::Attack),
            0x0040 => Some(Self::Skill),
            0x0080 => Some(Self::ExSkill),
            0x0100 => Some(Self::Jump),
            0x0200 => Some(Self::Reload),
            0x0400 => Some(Self::Status),
            0x0800 => Some(Self::Emotion),
            _ => {
                log::warn!(
                    "failed to convert `{}` to `{}`! (VALUE:{:?})",
                    stringify!(GameInputBits),
                    stringify!(GameInput),
                    flag
                );
                None
            }
        }
    }

    /// `GameInputBits`로 변환합니다.
    pub fn into_bits(self) -> GameInputBits {
        GameInputBits::from_bits_truncate(self as u16)
    }
}
