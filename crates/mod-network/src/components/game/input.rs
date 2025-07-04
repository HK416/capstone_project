use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::components::BigEndian;

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
pub const NUM_GAME_INPUTS: usize = 11;

/// 게임 입력 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum GameInput {
    Left = 0,
    Right = 1,
    Forward = 2,
    Backward = 3,
    Aiming = 4,
    Attack = 5,
    Skill = 6,
    Jump = 7,
    Reload = 8,
    Status = 9,
    Emotion = 10,
}

impl GameInput {
    /// 주어진 정수로부터 [`GameInput`]을 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2 => Some(Self::Forward),
            3 => Some(Self::Backward),
            4 => Some(Self::Aiming),
            5 => Some(Self::Attack),
            6 => Some(Self::Skill),
            7 => Some(Self::Jump),
            8 => Some(Self::Reload),
            9 => Some(Self::Status),
            10 => Some(Self::Emotion),
            _ => None,
        }
    }

    /// `GameInputBits`로 변환합니다.
    pub const fn into_bits(self) -> GameInputBits {
        match self {
            GameInput::Left => GameInputBits::Left,
            GameInput::Right => GameInputBits::Right,
            GameInput::Forward => GameInputBits::Forward,
            GameInput::Backward => GameInputBits::Backward,
            GameInput::Aiming => GameInputBits::Aiming,
            GameInput::Attack => GameInputBits::Attack,
            GameInput::Skill => GameInputBits::Skill,
            GameInput::Jump => GameInputBits::Jump,
            GameInput::Reload => GameInputBits::Reload,
            GameInput::Status => GameInputBits::Status,
            GameInput::Emotion => GameInputBits::Emotion,
        }
    }
}

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

    pub fn is_moved(self) -> bool {
        const BITS: GameInputBits = GameInputBits::Left
            .union(GameInputBits::Right)
            .union(GameInputBits::Forward)
            .union(GameInputBits::Backward);
        self.intersects(BITS)
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
