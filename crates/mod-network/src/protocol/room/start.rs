use crate::{
    components::{BigEndian, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 게임 시작 실패 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartFailedReason {
    /// 플레이어 인원 수가 부족합니다.
    NotEnoughPlayers = 0,
    /// 블루 팀 인원이 비어있습니다.
    EmptyBlueTeam = 1,
    /// 레드 팀 인원이 비어있습니다.
    EmptyRedTeam = 2,
    /// 팀 균형이 맞지 않습니다.
    UnbalancedTeams = 3,
    /// 모든 플레이어가 준비되지 않았습니다.
    PlayersNotReady = 4,
    /// 블루 팀 인원이 정원을 초과했습니다.
    LimitExceededBlueTeam = 5,
    /// 레드 팀 인원이 정원을 초과했습니다.
    LimitExceededRedTeam = 6,
}

impl StartFailedReason {
    /// 주어진 정수로 게임 시작 실패 사유를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(StartFailedReason::NotEnoughPlayers),
            1 => Some(StartFailedReason::EmptyBlueTeam),
            2 => Some(StartFailedReason::EmptyRedTeam),
            3 => Some(StartFailedReason::UnbalancedTeams),
            4 => Some(StartFailedReason::PlayersNotReady),
            5 => Some(StartFailedReason::LimitExceededBlueTeam),
            6 => Some(StartFailedReason::LimitExceededRedTeam),
            _ => None,
        }
    }
}

impl BigEndian for StartFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for StartFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 커스텀 게임 시작에 실패했을 때 서버에서 클라이언트로 보내는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGameFailedPacket {
    pub reason: StartFailedReason,
}

impl StartGameFailedPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(reason: StartFailedReason) -> Self {
        Self { reason }
    }
}

impl Packet for StartGameFailedPacket {
    fn packet_type() -> PacketType {
        PacketType::StartGameFailed
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = StartFailedReason::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(StartGameFailedPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
    }

    #[allow(unused_mut)]
    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type. (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 실패 사유를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = StartFailedReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = StartFailedReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

#[allow(unused_macros)]
macro_rules! test_start_failed_reason {
    ($name: ident, $e: expr) => {
        #[test]
        fn $name() {
            let val = $e as u8;
            let reason = StartFailedReason::new(val).unwrap();
            assert_eq!($e, reason);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_start_failed_reason() {
        StartFailedReason::new(123).unwrap();
    }

    test_start_failed_reason!(
        test_start_failed_reason_not_enough_player,
        StartFailedReason::NotEnoughPlayers
    );

    test_start_failed_reason!(
        test_start_failed_reason_empty_blue_team,
        StartFailedReason::EmptyBlueTeam
    );

    test_start_failed_reason!(
        test_start_failed_reason_empty_red_team,
        StartFailedReason::EmptyRedTeam
    );

    test_start_failed_reason!(
        test_start_failed_reason_unbalanced_team,
        StartFailedReason::UnbalancedTeams
    );

    test_start_failed_reason!(
        test_start_failed_reason_players_not_ready,
        StartFailedReason::PlayersNotReady
    );

    test_start_failed_reason!(
        test_start_failed_reason_limit_exceeded_blue_team,
        StartFailedReason::LimitExceededBlueTeam
    );

    test_start_failed_reason!(
        test_start_failed_reason_limit_exceeded_red_team,
        StartFailedReason::LimitExceededRedTeam
    );

    #[test]
    fn test_custom_game_start_failed_packet() {
        let reason = StartFailedReason::PlayersNotReady;

        let origin = StartGameFailedPacket::new(reason);
        let raw = origin.as_raw();
        let other = StartGameFailedPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
