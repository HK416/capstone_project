use std::num::NonZeroU16;

use crate::components::{BigEndian, TryFromBigEndian};

/// 플레이어의 체력입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HealthPoint(pub u16);

impl HealthPoint {
    /// 체력이 가질 수 있는 최소 값입니다.
    pub const MIN: Self = Self::new(0);

    /// 주어진 정수로 플레이어 체력을 생성합니다.
    pub const fn new(val: u16) -> Self {
        Self(val)
    }
}

impl BigEndian for HealthPoint {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::new(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for HealthPoint {
    fn default() -> Self {
        Self::MIN
    }
}

/// 플레이어의 최대 체력입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaxHealthPoint(pub NonZeroU16);

impl MaxHealthPoint {
    /// 최대 체력이 가질 수 있는 최소 값입니다.
    pub const MIN: Self = Self::new(unsafe { NonZeroU16::new_unchecked(1) });

    /// 주어진 정수로 플레이어 체력을 생성합니다.
    pub const fn new(val: NonZeroU16) -> Self {
        Self(val)
    }
}

impl BigEndian for MaxHealthPoint {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.get().to_big_endian_bytes()
    }
}

impl TryFromBigEndian for MaxHealthPoint {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self(NonZeroU16::new(u16::from_big_endian_bytes(bytes))?))
    }
}

impl Default for MaxHealthPoint {
    fn default() -> Self {
        Self::MIN
    }
}
