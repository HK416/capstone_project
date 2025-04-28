use crate::components::{BigEndian, TryFromBigEndian};

/// ActionState의 상태 수 입니다.
pub const NUM_ACTION_STATES: usize = 10;

/// 플레이어 행동 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionState {
    /// 아무 것도 하지 않는 상태
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
    /// Ex스킬을 사용하는 상태
    ExSkill = 8,
    /// 게임 시작 전 콜 싸인 상태
    Callsign = 9,
}

impl ActionState {
    /// 주어진 정수로 부터 `ActionState`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
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
            8 => Some(ActionState::ExSkill),
            9 => Some(ActionState::Callsign),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(ActionState),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for ActionState {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for ActionState {
    fn default() -> Self {
        ActionState::Idle
    }
}

impl TryFromBigEndian for ActionState {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 플레이어 캐릭터의 행동이 지속된 시간을 측정하는 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ActionStateTimer(pub f32);

impl ActionStateTimer {
    /// 타이머가 가질 수 있는 최소 시간입니다.
    pub const MIN_TIME: f32 = 0.0;

    /// 타이머를 초기화합니다.
    pub fn reset(&mut self) {
        self.0 = Self::MIN_TIME
    }
}

impl BigEndian for ActionStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for ActionStateTimer {
    fn default() -> Self {
        Self(Self::MIN_TIME)
    }
}
