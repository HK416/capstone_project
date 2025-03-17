use crate::components::{BigEndian, TryFromBigEndian};

/// 커스텀 게임에 참여 가능한 최대 인원 수 입니다.
pub const MAX_CUSTOM_GAME_PLAYERS: usize = 10;

/// (커스텀) 게임 접속 실패 사유
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JoinFailedReason {
    /// 해당 (커스텀) 게임을 찾지 못했습니다.
    NotFound = 0,
    /// (커스텀) 게임 수용 인원을 초과했습니다.
    FullCapacity = 1,
    /// 현재 (커스텀) 게임이 진행 중 입니다.
    InProgress = 2,
    /// (커스텀) 게임 관리자에 의해 차단(퇴장)당했습니다.
    Banned = 3,
}

impl BigEndian for JoinFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid JoinFailedReason")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

impl TryFromBigEndian for JoinFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes.get(0)? {
            0 => Some(Self::NotFound),
            1 => Some(Self::FullCapacity),
            2 => Some(Self::InProgress),
            3 => Some(Self::Banned),
            _ => {
                log::error!(
                    "invalid value for `{}`, (VALUE:{})",
                    stringify!(JoinFailedReason),
                    bytes[0]
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_join_failed_reason() {
        let reason = JoinFailedReason::FullCapacity;
        let bytes = reason.to_big_endian_bytes();
        let other = JoinFailedReason::from_big_endian_bytes(&bytes);

        assert_eq!(reason, other);
    }
}
