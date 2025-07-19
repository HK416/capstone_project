use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::components::{BigEndian, TryFromBigEndian};

/// `DirectionKind` 상태 수 입니다.
pub const NUM_DIRECTION_KINDS: usize = 9;

/// 컨트롤러의 방향 종류를 나타냅니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectionKind {
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

impl Default for DirectionKind {
    fn default() -> Self {
        DirectionKind::Idle
    }
}

/// 게임 입력의 수 입니다.
pub const NUM_INPUT_KINDS: usize = 11;

/// 게임 입력 키 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum InputKind {
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

impl InputKind {
    /// 주어진 정수로부터 [`InputKind`]을 생성합니다.
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
    pub const fn into_bits(self) -> HeldInput {
        match self {
            InputKind::Left => HeldInput::Left,
            InputKind::Right => HeldInput::Right,
            InputKind::Forward => HeldInput::Forward,
            InputKind::Backward => HeldInput::Backward,
            InputKind::Aiming => HeldInput::Aiming,
            InputKind::Attack => HeldInput::Attack,
            InputKind::Skill => HeldInput::Skill,
            InputKind::Jump => HeldInput::Jump,
            InputKind::Reload => HeldInput::Reload,
            InputKind::Status => HeldInput::Status,
            InputKind::Emotion => HeldInput::Emotion,
        }
    }
}

impl BigEndian for InputKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for InputKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 지속 입력 상태를 나타내는 16바이트 크기의 비트 플래그입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeldInput(u16);

bitflags! {
    impl HeldInput : u16 {
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

impl HeldInput {
    /// 새로운 `HeldInputKind`를 생성합니다.
    pub const fn new() -> Self {
        Self::empty()
    }

    pub fn is_moved(self) -> bool {
        const BITS: HeldInput = HeldInput::Left
            .union(HeldInput::Right)
            .union(HeldInput::Forward)
            .union(HeldInput::Backward);
        self.intersects(BITS)
    }

    /// `HeldInputKind`를 `DirectionKind`로 변환합니다.
    pub fn to_direction(self) -> DirectionKind {
        const STATES: [DirectionKind; 16] = [
            DirectionKind::Idle,
            DirectionKind::MovingLeft,
            DirectionKind::MovingRight,
            DirectionKind::Idle,
            DirectionKind::MovingForward,
            DirectionKind::MovingLeftForward,
            DirectionKind::MovingRightForward,
            DirectionKind::MovingForward,
            DirectionKind::MovingBackward,
            DirectionKind::MovingLeftBackward,
            DirectionKind::MovingRightBackward,
            DirectionKind::MovingBackward,
            DirectionKind::Idle,
            DirectionKind::MovingLeft,
            DirectionKind::MovingRight,
            DirectionKind::Idle,
        ];
        STATES[(self.bits() & 0x000F) as usize]
    }
}

impl BigEndian for HeldInput {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for HeldInput {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_input_kind() {
        InputKind::new(123).unwrap();
    }

    #[test]
    fn test_input_kind_left() {
        let val = InputKind::Left as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Left, kind);
    }

    #[test]
    fn test_input_kind_right() {
        let val = InputKind::Right as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Right, kind);
    }

    #[test]
    fn test_input_kind_forward() {
        let val = InputKind::Forward as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Forward, kind);
    }

    #[test]
    fn test_input_kind_backward() {
        let val = InputKind::Backward as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Backward, kind);
    }

    #[test]
    fn test_input_kind_aiming() {
        let val = InputKind::Aiming as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Aiming, kind);
    }

    #[test]
    fn test_input_kind_attack() {
        let val = InputKind::Attack as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Attack, kind);
    }

    #[test]
    fn test_input_kind_skill() {
        let val = InputKind::Skill as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Skill, kind);
    }

    #[test]
    fn test_input_kind_jump() {
        let val = InputKind::Jump as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Jump, kind);
    }

    #[test]
    fn test_input_kind_reload() {
        let val = InputKind::Reload as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Reload, kind);
    }

    #[test]
    fn test_input_kind_status() {
        let val = InputKind::Status as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Status, kind);
    }

    #[test]
    fn test_input_kind_emotion() {
        let val = InputKind::Emotion as u8;
        let kind = InputKind::new(val).unwrap();
        assert_eq!(InputKind::Emotion, kind);
    }

    #[test]
    fn test_input_event_kind() {
        let origin = InputKind::Reload;
        let bytes = origin.to_big_endian_bytes();
        let other = InputKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
