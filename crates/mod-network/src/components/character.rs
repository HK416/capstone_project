use serde::{Deserialize, Serialize};

use super::{BigEndian, Float3, TryFromBigEndian};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterAttributes {
    /// 캐릭터 이동 속도
    pub speed: f32,
    /// 위도가 최소(-60도)이고, `ActionState::Aim`일 때 총구의 상대 위치
    pub muzzle_position_min: Float3,
    /// 위도가 최소(-60도)이고, `ActionState::Aim`일 때 총구가 향하는 방향
    pub muzzle_direction_min: Float3,
    /// 위도가 0도이고, `ActionState::Aim`일 때 총구의 상대 위치
    pub muzzle_position_mid: Float3,
    /// 위도가 0도이고, `ActionState::Aim`일 때 총구가 향하는 방향
    pub muzzle_direction_mid: Float3,
    /// 위도가 최대(60도)이고, `ActionState::Aim`일 때 총구의 상대 위치
    pub muzzle_position_max: Float3,
    /// 위도가 최대(60도)이고, `ActionState::Aim`일 때 총구가 향하는 방향
    pub muzzle_direction_max: Float3,
    /// `MovementState::Moving` 애니메이션 시간 (단위: 초)
    pub move_ing_duration: f32,
    /// `MovementState::MoveToEnd` 애니메이션 시간 (단위: 초)
    pub move_end_normal_duration: f32,
    /// 걷기 애니메이션 시간 (단위: 초)
    pub walk_duration: f32,
    /// `ActionState::Idle` 애니메이션 시간 (단위: 초)
    pub normal_idle_duration: f32,
    /// `ActionState::Aiming` 애니메이션 시간 (단위: 초)
    pub normal_attack_start_duration: f32,
    /// `ActionState::AimOff` 애니메이션 시간 (단위: 초)
    pub normal_attack_end_duration: f32,
    /// `ActionState::Attack` 애니메이션 시간 (단위: 초)
    pub normal_attack_ing_duration: f32,
    /// 일반 공격 총알 발사 시간 (단위: 초)
    pub normal_attack_timing: Vec<f32>,
    /// 일반 공격 총알 발사 수
    pub normal_attack_count: u32,
    pub health_point: u32,
    pub attack_power: u32,
    pub defense_power: u32,
    pub accuracy_stat: u32,
    pub evasion_stat: u32,
    pub critical_rate: u32,
    pub critical_damage: u32,
    pub attack_range: u32,
    pub bullet_radius: f32,
}

impl CharacterAttributes {
    /// 라그랑주 보간법을 사용하여 총구의 위치를 계산합니다.
    ///
    /// # Note
    /// t의 값은 0부터 1사이의 값 입니다.
    ///
    pub fn get_muzzle_position(&self, t: f32) -> (f32, f32, f32) {
        let l1 = ((t - 0.5) * (t - 1.0)) / 0.5;
        let l2 = (t * (t - 1.0)) / -0.25;
        let l3 = (t * (t - 0.5)) / 0.5;

        let (x1, y1, z1): (f32, f32, f32) = self.muzzle_position_min.into();
        let (x2, y2, z2): (f32, f32, f32) = self.muzzle_position_mid.into();
        let (x3, y3, z3): (f32, f32, f32) = self.muzzle_position_max.into();

        let x = x1 * l1 + x2 * l2 + x3 * l3;
        let y = y1 * l1 + y2 * l2 + y3 * l3;
        let z = z1 * l1 + z2 * l2 + z3 * l3;

        (x, y, z)
    }

    /// 라그랑주 보간법을 사용하여 총구의 방향을 계산합니다.
    ///
    /// # Note
    /// t의 값은 0부터 1사이의 값 입니다.
    ///
    pub fn get_muzzle_direction(&self, t: f32) -> (f32, f32, f32) {
        let l1 = ((t - 0.5) * (t - 1.0)) / 0.5;
        let l2 = (t * (t - 1.0)) / -0.25;
        let l3 = (t * (t - 0.5)) / 0.5;

        let (x1, y1, z1): (f32, f32, f32) = self.muzzle_direction_min.into();
        let (x2, y2, z2): (f32, f32, f32) = self.muzzle_direction_mid.into();
        let (x3, y3, z3): (f32, f32, f32) = self.muzzle_direction_max.into();

        let x = x1 * l1 + x2 * l2 + x3 * l3;
        let y = y1 * l1 + y2 * l2 + y3 * l3;
        let z = z1 * l1 + z2 * l2 + z3 * l3;

        (x, y, z)
    }
}

/// 캐릭터 모델 종류 수 입니다.
pub const NUM_CHARACTERS: usize = 4;

/// 캐릭터 모델 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterKind {
    ArisOriginal = 0,
    MomoiOriginal = 1,
    MidoriOriginal = 2,
    YuukaOriginal = 3,
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
            2 => Some(CharacterKind::MidoriOriginal),
            3 => Some(CharacterKind::YuukaOriginal),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(CharacterKind),
                    index
                );
                None
            }
        }
    }
}

impl ToString for CharacterKind {
    fn to_string(&self) -> String {
        match self {
            CharacterKind::ArisOriginal => "Aris Original",
            CharacterKind::MomoiOriginal => "Momoi Original",
            CharacterKind::MidoriOriginal => "Midori Original",
            CharacterKind::YuukaOriginal => "Yuuka Original",
        }
        .to_string()
    }
}

/// 플레이어의 체력입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HealthPoint(pub u32);

impl HealthPoint {
    /// 체력이 가질 수 있는 최소 값입니다.
    pub const MIN_VALUE: u32 = 0;
}

impl BigEndian for HealthPoint {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for HealthPoint {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let value = u32::from_big_endian_bytes(bytes);
        if value >= Self::MIN_VALUE {
            Some(Self(value))
        } else {
            log::error!(
                "invalid value for `{}`, (VALUE:{})",
                stringify!(HealthPoint),
                value
            );
            None
        }
    }
}

impl Default for HealthPoint {
    fn default() -> Self {
        Self(Self::MIN_VALUE)
    }
}

/// 위도(Latitude)/경도(Longitude)로 구면좌표를 나타냅니다.  
/// 단위는 radian입니다.  
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}

impl LatLon {
    /// 최소 위도 각도입니다. (단위 라디안)
    pub const MIN_LATITUDE: f32 = -core::f32::consts::FRAC_PI_6;
    /// 최대 위도 각도입니다. (단위: 라디안)
    pub const MAX_LATITUDE: f32 = core::f32::consts::FRAC_PI_6;
    /// 위도 각도 범위 입니다. (단위 라디안)
    pub const LATITUDE_RANGE: f32 = Self::MAX_LATITUDE - Self::MIN_LATITUDE;
    /// 위도 각도의 절반 범위입니다. (단위 라디안)
    pub const LATITUDE_HALF_RANGE: f32 = 0.5 * Self::LATITUDE_RANGE;
}

impl BigEndian for LatLon {
    fn byte_size() -> usize {
        f32::byte_size() + f32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(LatLon)
        );

        let lat = f32::from_big_endian_bytes(&bytes[0..4]);
        let lon = f32::from_big_endian_bytes(&bytes[4..8]);
        Self { lat, lon }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.lat.to_big_endian_bytes());
        bytes.extend_from_slice(&self.lon.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(Bullet)
            );
        }

        bytes
    }
}

impl Default for LatLon {
    fn default() -> Self {
        Self { lat: 0.0, lon: 0.0 }
    }
}

/// ActionState의 상태 수 입니다.
pub const NUM_ACTION_STATES: usize = 5;

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

/// MovementState의 상태 수 입니다.
pub const NUM_MOVEMENT_STATES: usize = 7;
/// 최대 점프 지속 시간입니다.
pub const MAX_JUMP_DURATION: f32 = 0.25;

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
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(MovementState),
                    val
                );
                None
            }
        }
    }
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
        Self::new(u8::from_big_endian_bytes(bytes))
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

/// ViewState의 상태 수 입니다.
pub const NUM_VIEW_STATES: usize = 4;

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

impl ViewState {
    /// 주어진 정수로 부터 `ViewState`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(ViewState::Idle),
            1 => Some(ViewState::ZoomIn),
            2 => Some(ViewState::ZoomOut),
            3 => Some(ViewState::Aiming),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(ViewState),
                    val
                );
                None
            }
        }
    }
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
        Self::new(u8::from_big_endian_bytes(bytes))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_character_kind() {
        let origin = CharacterKind::MomoiOriginal;
        let bytes = origin.to_big_endian_bytes();
        let other = CharacterKind::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(CharacterKind::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_health_point() {
        let origin = HealthPoint(2700);
        let bytes = origin.to_big_endian_bytes();
        let other = HealthPoint::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(HealthPoint::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_latlon() {
        let origin = LatLon {
            lat: 3.141592,
            lon: -0.199928,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = LatLon::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(LatLon::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
