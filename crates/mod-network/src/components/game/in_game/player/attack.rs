//! 플레이어의 공격과 관련된 코드를 관리합니다.
//!

use crate::components::BigEndian;

/// 남은 총알의 수를 나타냅니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RemainingBullet(u32);

impl RemainingBullet {
    /// 새로운 총알 데이터를 생성합니다.
    pub fn new(max_bullets: u16, num_remaining_bullets: u16) -> Self {
        let max_field = ((max_bullets & 0xFFF) << 12) as u32;
        let remaining_filed = ((num_remaining_bullets & 0xFFF) << 0) as u32;
        Self(max_field | remaining_filed)
    }

    /// 최대 총알의 개수를 반환합니다.
    pub fn max_bullets(&self) -> u16 {
        ((self.0 >> 12) & 0xFFF) as u16
    }

    /// 남은 총알의 개수를 반환합니다.
    pub fn num_remaining_bullets(&self) -> u16 {
        ((self.0 >> 0) & 0xFFF) as u16
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

/// 현재 Ex스킬 코스트를 나타냅니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ExSkillCost(pub f32);

impl ExSkillCost {
    /// 최대 Ex스킬 코스트입니다.
    pub const MAX: Self = Self(100.0);
}

impl BigEndian for ExSkillCost {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(f32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for ExSkillCost {
    fn default() -> Self {
        Self(0.0)
    }
}
