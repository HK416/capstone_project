//! 플레이어의 공격과 관련된 코드를 관리합니다.
//! 

use crate::components::BigEndian;

/// 남은 총알의 수를 나타냅니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RemainingBullet(u32);

impl RemainingBullet {
    /// 새로운 총알 데이터를 생성합니다.
    pub fn new(max_bullets: u32, num_remaining_bullets: u32) -> Self {
        let max_field = (max_bullets & 0xFFF) << 12;
        let remaining_filed = (num_remaining_bullets & 0xFFF) << 0;
        Self(max_field | remaining_filed)
    }

    /// 최대 총알의 개수를 반환합니다.
    pub fn max_bullets(&self) -> u32 {
        (self.0 >> 12) & 0xFFF
    }

    /// 남은 총알의 개수를 반환합니다.
    pub fn num_remaining_bullets(&self) -> u32 {
        (self.0 >> 0) & 0xFFF
    }
}

impl BigEndian for RemainingBullet {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for RemainingBullet {
    fn default() -> Self {
        Self(0)
    }
}
