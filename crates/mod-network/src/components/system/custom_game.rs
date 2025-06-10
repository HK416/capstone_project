use crate::components::{BigEndian, TryFromBigEndian};

/// (커스텀) 게임 접속 실패 사유
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JoinFailedReason {
    /// 해당 커스텀 게임을 찾지 못했습니다.
    NotFound = 0,
    /// 커스텀 게임 수용 인원을 초과했습니다.
    FullCapacity = 1,
    /// 현재 커스텀 게임이 진행 중 입니다.
    InProgress = 2,
    /// 커스텀 게임 관리자에 의해 차단 또는 퇴장 당했습니다.
    Banned = 3,
}

impl JoinFailedReason {
    /// 주어진 정수로 `JoinFailedReason`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::NotFound),
            1 => Some(Self::FullCapacity),
            2 => Some(Self::InProgress),
            3 => Some(Self::Banned),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(JoinFailedReason),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for JoinFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for JoinFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// (커스텀) 게임 시작 실패 사유
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartFailedReason {
    /// 참여 인원이 적은 경우
    NotEnoughPlayers = 0,
    /// 팀 균형이 맞지 않은 경우
    UnbalancedTeams = 1,
    /// 모든 플레이어가 준비되지 않은 경우
    PlayersNotReady = 2,
    /// Blue팀 플레이어가 비어있는 경우
    EmptyBlueTeam = 3,
    /// Red팀 플레이어가 비어있는 경우
    EmptyRedTeam = 4,
}

impl StartFailedReason {
    /// 주어진 정수로 `StartFailedReason`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::NotEnoughPlayers),
            1 => Some(Self::UnbalancedTeams),
            2 => Some(Self::PlayersNotReady),
            3 => Some(Self::EmptyBlueTeam),
            4 => Some(Self::EmptyRedTeam),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(StartFailedReason),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for StartFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for StartFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_failed_reason() {
        let reason = JoinFailedReason::FullCapacity;
        let bytes = reason.to_big_endian_bytes();
        let other = JoinFailedReason::from_big_endian_bytes(&bytes);

        assert_eq!(reason, other);
    }

    #[test]
    fn test_start_failed_reason() {
        let reason = StartFailedReason::UnbalancedTeams;
        let bytes = reason.to_big_endian_bytes();
        let other = StartFailedReason::from_big_endian_bytes(&bytes);

        assert_eq!(reason, other);
    }
}
