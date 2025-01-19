use super::{BigEndian, TryFromBigEndian};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct HealthPoint(pub f32);

impl HealthPoint {
    /// 체력이 가질 수 있는 최소 값입니다.
    pub const MIN_VALUE: f32 = 0.0;
}

impl BigEndian for HealthPoint {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for HealthPoint {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let value = f32::from_big_endian_bytes(bytes);
        if value >= Self::MIN_VALUE {
            Some(Self(value))
        } else {
            None
        }
    }
}

impl Default for HealthPoint {
    fn default() -> Self {
        Self(Self::MIN_VALUE)
    }
}
