use serde::{Deserialize, Serialize};

use crate::assets::Float3;

use super::{BigEndian, TryFromBigEndian};

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

/// 스테이지 종류 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageKind {
    Downtown = 0,
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
        StageKind::Downtown
    }
}

impl TryFromBigEndian for StageKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(StageKind::Downtown),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(StageKind),
                    index
                );
                None
            }
        }
    }
}

impl ToString for StageKind {
    fn to_string(&self) -> String {
        match self {
            StageKind::Downtown => "Downtown",
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
