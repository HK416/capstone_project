//! 지형 종류와 관련된 코드를 관리합니다.

use crate::components::{BigEndian, TryFromBigEndian};

/// 스테이지 종류의 수 입니다.
pub const NUM_STAGES: usize = 1;

/// 스테이지 종류 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageKind {
    /// 시가지 지형
    #[default]
    City = 0,
}

impl StageKind {
    /// 주어진 정수로 부터 `StageKind`를 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(StageKind::City),
            _ => None,
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

impl TryFromBigEndian for StageKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_bullet_kind() {
        StageKind::new(123).unwrap();
    }

    #[test]
    fn test_bullet_kind_common() {
        let val = StageKind::City as u8;
        let kind = StageKind::new(val).unwrap();
        assert_eq!(StageKind::City, kind);
    }

    #[test]
    fn test_bullet_kind() {
        let origin = StageKind::City;
        let bytes = origin.to_big_endian_bytes();
        let other = StageKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인한다.
        assert_eq!(origin, other);
    }
}
