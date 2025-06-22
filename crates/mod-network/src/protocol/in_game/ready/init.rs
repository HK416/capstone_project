//! 인게임 장면을 초기화 하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{
        BigEndian, InGamePlayerInitData, StageKind, TryFromBigEndian, MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 인게임 장면 초기화 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGameDataInitPacket {
    /// 스테이지 종류
    pub stage_kind: StageKind,
    /// 플레이어 초기화 데이터
    pub players: Vec<InGamePlayerInitData>,
}

impl InGameDataInitPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(stage_kind: StageKind, players: Vec<InGamePlayerInitData>) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");

        Self {
            stage_kind,
            players,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(stage_kind: StageKind, iter: I) -> Self
    where
        I: IntoIterator<Item = InGamePlayerInitData>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(stage_kind, iter.into_iter().collect())
    }
}

impl Packet for InGameDataInitPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameDataInit
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let data_size = StageKind::byte_size()
            + u8::byte_size()
            + InGamePlayerInitData::byte_size() * num_players;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.stage_kind.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameDataInitPacket)
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

        // 스테이지 종류를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = StageKind::byte_size();
        let mut data = &bytes[offset..offset + size];
        let stage_kind = StageKind::try_from_big_endian_bytes(data)?;

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 플레이어 초기화 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for _ in 0..num_players {
            offset = offset + size;
            size = InGamePlayerInitData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(InGamePlayerInitData::from_big_endian_bytes(data));
        }

        Some(Self {
            stage_kind,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{
        CharacterKind, LatLon, NetworkState, Permission, Team, UserId, UserName,
    };

    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_in_game_init_packet() {
        InGameDataInitPacket::new(StageKind::City, vec![]);
    }

    #[test]
    fn test_in_game_init_packet() {
        let player_0 = InGamePlayerInitData::new(
            UserId::new(1232154),
            UserName::from_str("아리스"),
            CharacterKind::ArisOriginal,
            Team::Blue,
            0,
            Permission::User,
            true,
            NetworkState::Fair,
            1000,
            4,
            100,
            [0.1341431, 1.2413413, -0.341431241],
            [0.031431241, 0.000213412, 0.8741431241, 0.3414134],
            LatLon::new(2.0241431, 0.03411341),
        );
        let player_1 = InGamePlayerInitData::new(
            UserId::new(4314321),
            UserName::from_str("유즈"),
            CharacterKind::ArisOriginal,
            Team::Red,
            0,
            Permission::Admin,
            true,
            NetworkState::Good,
            1000,
            4,
            100,
            [0.1341431, 1.2413413, -0.341431241],
            [0.031431241, 0.000213412, 0.8741431241, 0.3414134],
            LatLon::new(2.0241431, 0.03411341),
        );

        let players = vec![player_0, player_1];
        let origin = InGameDataInitPacket::new(StageKind::City, players);
        let raw = origin.as_raw();
        let other = InGameDataInitPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
