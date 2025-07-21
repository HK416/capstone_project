//! 클라이언트가 캐릭터 편성 장면에 있을 때 참여한 플레이어 데이터 갱신 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, FormationPlayerUpdateData, TryFromBigEndian, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 캐릭터 편성 데이터 갱신 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct FormationDataUpdatePacket {
    /// 편성까지 남은 시간
    pub remaining_time_ms: u16,
    /// 플레이어 데이터
    pub players: Vec<FormationPlayerUpdateData>,
}

impl FormationDataUpdatePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(remaining_time_ms: u16, players: Vec<FormationPlayerUpdateData>) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");
        Self {
            remaining_time_ms,
            players,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(remaining_time_ms: u16, iter: I) -> Self
    where
        I: IntoIterator<Item = FormationPlayerUpdateData>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(remaining_time_ms, iter.into_iter().collect())
    }
}

impl Packet for FormationDataUpdatePacket {
    fn packet_type() -> PacketType {
        PacketType::FormationDataUpdate
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let data_size = u16::byte_size()
            + u8::byte_size()
            + FormationPlayerUpdateData::byte_size() * num_players;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.remaining_time_ms.to_big_endian_bytes());
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
                stringify!(FormationDataUpdatePacket)
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

        // 남은 시간을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let remaining_time_ms = u16::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let mut num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        while num_players > 0 {
            offset = offset + size;
            size = FormationPlayerUpdateData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(FormationPlayerUpdateData::try_from_big_endian_bytes(data)?);
            num_players -= 1;
        }

        Some(Self {
            remaining_time_ms,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{CharacterKind, NetworkState, Permission, UserId};

    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_formation_data_update_packet() {
        FormationDataUpdatePacket::new(134, vec![]);
    }

    #[test]
    fn test_formation_data_update_packet() {
        let player_0 = FormationPlayerUpdateData::new(
            UserId::new(1431324),
            true,
            Permission::Admin,
            NetworkState::Good,
            None,
        );
        let player_1 = FormationPlayerUpdateData::new(
            UserId::new(646345),
            false,
            Permission::User,
            NetworkState::Fair,
            Some(CharacterKind::MomoiOriginal),
        );
        let player_2 = FormationPlayerUpdateData::new(
            UserId::new(86453),
            true,
            Permission::User,
            NetworkState::Critical,
            None,
        );
        let player_3 = FormationPlayerUpdateData::new(
            UserId::new(654432),
            true,
            Permission::User,
            NetworkState::Good,
            Some(CharacterKind::YuukaOriginal),
        );

        let players = vec![player_0, player_1, player_2, player_3];
        let origin = FormationDataUpdatePacket::new(42123, players);
        let raw = origin.as_raw();
        let other = FormationDataUpdatePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
