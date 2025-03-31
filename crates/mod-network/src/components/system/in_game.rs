use crate::components::{BigEndian, TryFromBigEndian};

/// 게임에 참여 가능한 최대 인원 수 입니다.
pub const MAX_IN_GAME_PLAYERS: usize = 10;

/// 게임 플레이 중단 사유 목록
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GamePlayStopReason {
    /// 참여 인원이 적은 경우
    NotEnughPlayers = 0,
    /// 한쪽 팀의 인원이 비어있는 경우
    OneTeamEmpty = 1,
}

impl GamePlayStopReason {
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::NotEnughPlayers),
            1 => Some(Self::OneTeamEmpty),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(GamePlayStopReason),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for GamePlayStopReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for GamePlayStopReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_play_stop_reason() {
        let reason = GamePlayStopReason::OneTeamEmpty;
        let bytes = reason.to_big_endian_bytes();
        let other = GamePlayStopReason::from_big_endian_bytes(&bytes);

        assert_eq!(reason, other);
    }
}
