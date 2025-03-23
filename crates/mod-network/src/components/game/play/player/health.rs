use crate::components::{BigEndian, TryFromBigEndian};

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
