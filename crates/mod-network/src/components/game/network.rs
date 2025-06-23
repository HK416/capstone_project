//! 네트워크 통신과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 통신 상태를 나타냅니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NetworkState {
    /// 201ms를 초과하는 핑인 경우
    #[default]
    Critical = 0,
    /// 101..=200ms ping
    Poor = 1,
    /// 51..=101ms ping
    Fair = 2,
    /// 0..=50ms ping
    Good = 3,
}

impl NetworkState {
    /// 새로운 통신 상태 데이터를 생성합니다.
    ///
    /// 주어진 값이 범위를 초과하는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Critical),
            1 => Some(Self::Poor),
            2 => Some(Self::Fair),
            3 => Some(Self::Good),
            _ => None,
        }
    }
}

impl BigEndian for NetworkState {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for NetworkState {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_network_state() {
        NetworkState::new(5).unwrap();
    }

    #[test]
    fn test_network_state_critical() {
        let val = NetworkState::Critical as u8;
        let state = NetworkState::new(val).unwrap();
        assert_eq!(NetworkState::Critical, state);
    }

    #[test]
    fn test_network_state_poor() {
        let val = NetworkState::Poor as u8;
        let state = NetworkState::new(val).unwrap();
        assert_eq!(NetworkState::Poor, state);
    }

    #[test]
    fn test_network_state_fair() {
        let val = NetworkState::Fair as u8;
        let state = NetworkState::new(val).unwrap();
        assert_eq!(NetworkState::Fair, state);
    }

    #[test]
    fn test_network_state_good() {
        let val = NetworkState::Good as u8;
        let state = NetworkState::new(val).unwrap();
        assert_eq!(NetworkState::Good, state);
    }
}
