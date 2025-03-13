use serde::{Deserialize, Serialize};

use super::{BigEndian, Float2, Float3, Float4, HealthPoint, TryFromBigEndian, UserId};

/// 스테이지 종류의 수 입니다.
pub const NUM_STAGES: usize = 1;

/// 스테이지 종류 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageKind {
    City = 0,
}

impl StageKind {
    /// 주어진 정수로 부터 `StageKind`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(StageKind::City),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(StageKind),
                    val
                );
                None
            }
        }
    }
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
        StageKind::City
    }
}

impl TryFromBigEndian for StageKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

impl ToString for StageKind {
    fn to_string(&self) -> String {
        match self {
            StageKind::City => "City",
        }
        .to_string()
    }
}

/// 게임 월드 스테이지 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageLayoutData {
    /// 게임 월드의 각 지역의 크기입니다.
    pub area_size: Float2,
    /// x축 방향의 지역의 수 입니다.
    pub num_area_width: u32,
    /// z축 방향의 지역의 수 입니다.
    pub num_area_depth: u32,
    /// 게임 월드 스테이지에서 사용되는 모델의 이름입니다.
    pub models: Vec<String>,
    /// 게임 월드 스테이지 지역 데이터입니다.
    pub area: Vec<StageAreaData>,
    /// 게임 월드 스테이지 소품 데이터입니다.
    pub props: Vec<StagePropData>,
}

/// 게임 월드 스테이지를 구성하는 지역 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageAreaData {
    pub model: String,
    pub height: String,
    pub translation: Float3,
    pub rotation: Float4,
}

/// 게임 월드 스테이지를 구성하는 소품 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StagePropData {
    pub model: String,
    pub scale: Float3,
    pub translation: Float3,
    pub rotation: Float4,
}

/// 게임 월드 스테이지의 높이 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageHeight {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
}

/// 게임 월드의 플레이어가 입은 데미지 정보입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DamageLog {
    pub user_id: UserId,
    pub damage: HealthPoint,
}

impl BigEndian for DamageLog {
    fn byte_size() -> usize {
        UserId::byte_size() + HealthPoint::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.user_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage.to_big_endian_bytes());
        bytes
    }
}

impl Default for DamageLog {
    fn default() -> Self {
        Self {
            user_id: UserId::NULL,
            damage: HealthPoint(0),
        }
    }
}

impl TryFromBigEndian for DamageLog {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(DamageLog)
        );

        // 오브젝트 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let object_id = UserId::from_big_endian_bytes(data);

        // 데미지를 가져옵니다.
        offset = offset + size;
        size = HealthPoint::byte_size();
        data = &bytes[offset..offset + size];
        let damage = HealthPoint::try_from_big_endian_bytes(data)?;

        Some(Self {
            user_id: object_id,
            damage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn creation_test_stage_kind() {
        let bytes = [127];
        StageKind::from_big_endian_bytes(&bytes);
    }

    #[test]
    fn validation_test_stage_kind() {
        let origin = StageKind::City;
        let bytes = origin.to_big_endian_bytes();
        let other = StageKind::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(StageKind::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_player() {
        let origin = DamageLog {
            user_id: UserId::new(3141592),
            damage: HealthPoint(2700),
        };
        let bytes = origin.to_big_endian_bytes();
        let other = DamageLog::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(DamageLog::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
