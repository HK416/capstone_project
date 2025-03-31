//! 모든 단계에서 흔하게 사용되는 코드를 관리합니다.
//!

mod account;
mod character;

use crate::components::{BigEndian, TryFromBigEndian};

pub use self::{account::*, character::*};

/// 플레이어가 속한 팀의 종류
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Team {
    Blue = 0,
    Red = 1,
}

impl Team {
    /// 주어진 정수로 부터 `Team`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Team::Blue),
            1 => Some(Team::Red),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(Team),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for Team {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for Team {
    fn default() -> Self {
        Self::Blue
    }
}

impl TryFromBigEndian for Team {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_create_team() {
        Team::new(5).unwrap();
    }

    #[test]
    fn test_team() {
        let origin = Team::Red;
        let bytes = origin.to_big_endian_bytes();
        let other = Team::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
