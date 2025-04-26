//! 플레이어의 공격과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 플레이어의 총알의 수 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RemainingBullet {
    /// 현재 플레이어의 총알 수 입니다.
    pub current: u16,
    /// 플레이어의 최대 총알 수 입니다.  
    /// 최대 총알 수가 0인 경우 무한대를 나타냅니다.
    pub maximum: u16,
}

impl RemainingBullet {
    /// 새로운 총알 수 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `current`가 `maximum`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(current: u16, maximum: u16) -> Self {
        assert!(
            current <= maximum,
            "the number of bullets is out of bounds!"
        );
        unsafe { Self::new_unchecked(current, maximum) }
    }

    /// 새로운 총알 수 데이터를 생성합니다.
    ///
    /// # Safety
    /// 주어진 `current`가 `maximum`보다 클 경우 정의되지 않은 동작을 수행할 수 있습니다.
    ///
    pub unsafe fn new_unchecked(current: u16, maximum: u16) -> Self {
        Self { current, maximum }
    }

    /// 새로운 총알 수 데이터를 생성합니다.
    pub fn splat(maximum: u16) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }

    /// 플레이어 총알 수 비율을 0부터 1사이의 값으로 반환합니다.  
    /// 플레이어 현재 총알 수가 최대 총알 수 보다 클 경우 1이상의 값을 반환합니다.  
    /// 플레이어 최대 총알 수가 0인 경우 [`f32::INFINITY`]를 반환합니다.
    pub fn percent(&self) -> f32 {
        if self.maximum == 0 {
            f32::INFINITY
        } else {
            self.current as f32 / self.maximum as f32
        }
    }
}

impl BigEndian for RemainingBullet {
    fn byte_size() -> usize {
        u16::byte_size() + u16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.current.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RemainingBullet)
            );
        }

        bytes
    }
}

impl Default for RemainingBullet {
    fn default() -> Self {
        Self {
            current: 0,
            maximum: 0,
        }
    }
}

impl TryFromBigEndian for RemainingBullet {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(RemainingBullet)
        );

        // 현재 총알 수를 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let current = u16::from_big_endian_bytes(data);

        // 최대 총알 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum = u16::from_big_endian_bytes(data);

        if current <= maximum {
            Some(Self { current, maximum })
        } else {
            None
        }
    }
}

/// 플레이어 Ex스킬의 코스트 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ExSkillCost(pub f32);

impl ExSkillCost {
    /// 최대 Ex스킬 코스트입니다.
    pub const MAX_COST: f32 = 100.0;

    /// Ex스킬 코스트 비율을 0부터 1사이의 값으로 반환합니다.
    pub fn percent(&self) -> f32 {
        (self.0 / Self::MAX_COST).clamp(0.0, 1.0)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_remaining_bullets() {
        RemainingBullet::new(1, 0);
    }

    #[test]
    fn test_remaining_bullets() {
        let origin = RemainingBullet::new(123, 456);
        let bytes = origin.to_big_endian_bytes();
        let other = RemainingBullet::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
