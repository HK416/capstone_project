//! 게임 팀과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 플레이어가 속한 팀의 종류입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Team {
    #[default]
    Blue = 0,
    Red = 1,
}

impl Team {
    /// 주어진 정수로 부터 `Team`을 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Team::Blue),
            1 => Some(Team::Red),
            _ => None,
        }
    }

    /// 상대방 팀을 반환합니다.
    pub fn opponent(&self) -> Self {
        match self {
            Team::Blue => Team::Red,
            Team::Red => Team::Blue,
        }
    }
}

impl BigEndian for Team {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
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
    fn test_creation_team() {
        Team::new(5).unwrap();
    }

    #[test]
    fn test_creation_team_blue() {
        let val = Team::Blue as u8;
        let team = Team::new(val).unwrap();
        assert_eq!(Team::Blue, team);
    }

    #[test]
    fn test_creation_team_red() {
        let val = Team::Red as u8;
        let team = Team::new(val).unwrap();
        assert_eq!(Team::Red, team);
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
