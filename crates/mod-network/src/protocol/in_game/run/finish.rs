//! 게임 결과 전송 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, InGamePlayerResultData, Team, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 인게임 종료 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameFinishPacket {
    /// 게임 플레이 시간 (단위: ms)
    pub play_time_ms: u32,
    /// 우승 팀 정보. `None`인 경우 무승부를 나타냅니다.
    pub winner: Option<Team>,
    /// 플레이어 데이터
    pub players: Vec<InGamePlayerResultData>,
}

impl InGameFinishPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// - 주어진 `players`의 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(
        play_time_ms: u32,
        winner: Option<Team>,
        players: Vec<InGamePlayerResultData>,
    ) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");
        Self {
            play_time_ms,
            winner,
            players,
        }
    }
}

impl Packet for InGameFinishPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameFinish
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let data_size = u32::byte_size()
            + u8::byte_size()
            + u8::byte_size()
            + InGamePlayerResultData::byte_size() * num_players;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.play_time_ms.to_big_endian_bytes());
        let winner = self.winner.map(|team| team as u8).unwrap_or(0x3);
        data.extend_from_slice(&winner.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameFinishPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::warn!(
                "invalid packet type. (RAW:{:?}, PACKET:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 게임 플레이 시간을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let play_time_ms = u32::from_big_endian_bytes(data);

        // 우승 팀 정보를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let winner = Team::new(u8::from_big_endian_bytes(data));

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 플레이어 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(num_players);
        for _ in 0..num_players {
            offset = offset + size;
            size = InGamePlayerResultData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(InGamePlayerResultData::from_big_endian_bytes(data));
        }

        Some(Self {
            play_time_ms,
            winner,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::UserId;

    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_in_game_finish_packet() {
        InGameFinishPacket::new(22313, None, vec![]);
    }

    #[test]
    fn test_in_game_finish_packet() {
        let player_0 =
            InGamePlayerResultData::new(UserId::new(543141), 123, 4321, 4513, 56414, 133412, true);
        let player_1 =
            InGamePlayerResultData::new(UserId::new(1341), 641, 414, 43221, 8154, 4312, false);
        let players = vec![player_0, player_1];

        let origin = InGameFinishPacket::new(84312, None, players);
        let raw = origin.as_raw();
        let other = InGameFinishPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
