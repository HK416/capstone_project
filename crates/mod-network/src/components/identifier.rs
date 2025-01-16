use super::{BigEndian, TryFromBigEndian};

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
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(ActionState::Idle),
            1 => Some(ActionState::Aiming),
            2 => Some(ActionState::AimAt),
            3 => Some(ActionState::AimOff),
            4 => Some(ActionState::Attack),
            _ => None,
        }
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

/// 캐릭터 모델 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterKind {
    ArisOriginal = 0,
    MomoiOriginal = 1,
}

impl BigEndian for CharacterKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for CharacterKind {
    fn default() -> Self {
        CharacterKind::ArisOriginal
    }
}

impl TryFromBigEndian for CharacterKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(CharacterKind::ArisOriginal),
            1 => Some(CharacterKind::MomoiOriginal),
            _ => None,
        }
    }
}

impl ToString for CharacterKind {
    fn to_string(&self) -> String {
        match self {
            CharacterKind::ArisOriginal => "Aris Original",
            CharacterKind::MomoiOriginal => "Momoi Original",
        }
        .to_string()
    }
}

/// 클라이언트를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

impl ClientId {
    /// 비어있는 클라이언트 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 클라이언트 식별자의 최대 값 입니다.
    pub const MAX: Self = Self(u64::MAX);

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정수가 `0` 또는 `u64::MAX`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(num: u64) -> Self {
        assert!(num != 0 && num != u64::MAX, "out of bounds");
        unsafe { Self::new_unchecked(num) }
    }

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(num: u64) -> Self {
        Self(num)
    }
}

impl BigEndian for ClientId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for ClientId {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u64::from_big_endian_bytes(bytes);
        if num != 0 && num != u64::MAX {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            None
        }
    }
}

impl Into<u64> for ClientId {
    fn into(self) -> u64 {
        self.0
    }
}

impl Into<u32> for ClientId {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl Into<ObjectId> for ClientId {
    fn into(self) -> ObjectId {
        ObjectId(self.0 as u32)
    }
}

/// 게임 월드의 시대를 나타냅니다.
///
/// 클라이언트에서 항상 마지막으로 전송된 네트워크 패킷을 처리하기 위해 사용됩니다.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(u64);

impl Epoch {
    /// 게임 월드 시대의 최대 값 입니다.
    pub const MAX: Self = Self(u64::MAX);

    /// 주어진 정수로 새로운 게임 월드 시대를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정수가 `u64::MAX`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(num: u64) -> Self {
        assert!(num != u64::MAX, "out of bounds");
        unsafe { Self::new_unchecked(num) }
    }

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(num: u64) -> Self {
        Self(num)
    }
}

impl BigEndian for Epoch {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for Epoch {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u64::from_big_endian_bytes(bytes);
        if num != u64::MAX {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            None
        }
    }
}

/// 게임 월드 내 오브젝트를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(u32);

impl ObjectId {
    /// 비어있는 오브젝트 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 오브젝트 식별자의 최대 값 입니다.
    pub const MAX: Self = Self(u32::MAX);

    /// 주어진 정수로 새로운 오브젝트 식별자를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정수가 `0` 또는 `u32::MAX`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(num: u32) -> Self {
        assert!(num != 0 && num != u32::MAX, "out of bounds");
        unsafe { Self::new_unchecked(num) }
    }

    /// 주어진 정수로 새로운 오브젝트 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(num: u32) -> Self {
        Self(num)
    }
}

impl BigEndian for ObjectId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for ObjectId {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u32::from_big_endian_bytes(bytes);
        if num != 0 && num != u32::MAX {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            None
        }
    }
}

impl Into<u32> for ObjectId {
    fn into(self) -> u32 {
        self.0
    }
}

/// 캐릭터의 움직임 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementState {
    /// 아무 움직임도 없는 상태
    Idle = 0,
    /// 움직이는 중인 상태
    Moving = 1,
    /// 움직였다 멈춘 상태
    MoveToEnd = 2,
}

impl BigEndian for MovementState {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for MovementState {
    fn default() -> Self {
        MovementState::Idle
    }
}

impl TryFromBigEndian for MovementState {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(MovementState::Idle),
            1 => Some(MovementState::Moving),
            2 => Some(MovementState::MoveToEnd),
            _ => None,
        }
    }
}

/// 플레이어 움직임 상태의 지속 시간을 나타냅니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MovementStateTimer(pub f32);

impl MovementStateTimer {
    /// 타이머가 가질 수 있는 최소 시간입니다.
    pub const MIN_TIME: f32 = 0.0;

    /// 타이머를 초기화합니다.
    pub fn reset(&mut self) {
        self.0 = Self::MIN_TIME
    }
}

impl BigEndian for MovementStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for MovementStateTimer {
    fn default() -> Self {
        Self(Self::MIN_TIME)
    }
}

/// 스테이지 종류 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageKind {
    School = 0,
}

impl BigEndian for StageKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for StageKind {
    fn default() -> Self {
        StageKind::School
    }
}

impl TryFromBigEndian for StageKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(StageKind::School),
            _ => None,
        }
    }
}

/// 플레이어 카메라의 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ViewState {
    /// 아무 것도 하지 않는 상태
    Idle = 0,
    /// 조준을 준비하는 상태
    ZoomIn = 1,
    /// 조준을 해제하는 상태
    ZoomOut = 2,
    /// 조준하는 상태
    Aiming = 3,
}

impl BigEndian for ViewState {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState::Idle
    }
}

impl TryFromBigEndian for ViewState {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(ViewState::Idle),
            1 => Some(ViewState::ZoomIn),
            2 => Some(ViewState::ZoomOut),
            3 => Some(ViewState::Aiming),
            _ => None,
        }
    }
}

/// 플레이어 뷰 상태의 지속 시간을 나타냅니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ViewStateTimer(pub f32);

impl ViewStateTimer {
    /// 타이머가 가질 수 있는 최소 시간입니다.
    pub const MIN_TIME: f32 = 0.0;

    /// 타이머를 초기화합니다.
    pub fn reset(&mut self) {
        self.0 = Self::MIN_TIME
    }
}

impl BigEndian for ViewStateTimer {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for ViewStateTimer {
    fn default() -> Self {
        Self(Self::MIN_TIME)
    }
}
