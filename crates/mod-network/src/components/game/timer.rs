//! 플레이어 타이머와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, HeldInput};

/// 플레이어 행동 상태 타이머입니다. (단위: ms)
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionStateTimer(pub u16);

impl ActionStateTimer {
    /// 새로운 행동 상태 타이머를 생성합니다.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

impl BigEndian for ActionStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for ActionStateTimer {
    fn default() -> Self {
        Self(0)
    }
}

/// 플레이어 움직임 상태 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MovementStateTimer(pub u16);

impl MovementStateTimer {
    /// 새로운 행동 상태 타이머를 생성합니다.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

impl BigEndian for MovementStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for MovementStateTimer {
    fn default() -> Self {
        Self(0)
    }
}

/// 플레이어 시야 상태 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ViewStateTimer(pub u16);

impl ViewStateTimer {
    /// 새로운 행동 상태 타이머를 생성합니다.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

impl BigEndian for ViewStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for ViewStateTimer {
    fn default() -> Self {
        Self(0)
    }
}

/// 최대 플레이어 입력 상태 타이머 시간입니다. (단위: ms)
pub const MAX_INPUT_STATE_TIME: u16 = 250;

/// 플레이어 입력 상태 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct InputStateTimer(pub u16);

impl InputStateTimer {
    /// 새로운 입력 상태 타이머를 생성합니다.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// 플레이어 입력 상태 타이머를 갱신합니다.
    pub fn update(&mut self, held_input: HeldInput, elapsed_time_ms: u16) {
        if held_input.is_moved() {
            self.0 = self
                .0
                .saturating_add(elapsed_time_ms)
                .min(MAX_INPUT_STATE_TIME);
        } else {
            self.0 = self.0.saturating_sub(elapsed_time_ms);
        }
    }

    /// 0..=1 사이의 값을 반환합니다.
    pub fn perentage(&self) -> f32 {
        self.0 as f32 / MAX_INPUT_STATE_TIME as f32
    }
}

impl BigEndian for InputStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for InputStateTimer {
    fn default() -> Self {
        Self(0)
    }
}
