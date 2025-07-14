//! 인게임 장면을 갱신하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{
        BigEndian, CapturePoint, InGamePlayerStatusPullData, ObjectId, TryFromBigEndian,
        MAX_IN_GAME_BULLETS, MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

#[derive(Debug, Clone, PartialEq)]
pub struct InGameStatusPacket {
    /// 점령 데이터
    pub capture_point: CapturePoint,
    /// 플레이어 데이터
    pub players: Vec<InGamePlayerStatusPullData>,
    /// 제거된 총알 오브젝트 식별자
    pub removed_bullets: Vec<ObjectId>,
}

impl InGameStatusPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `player`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    /// 주어진 `removed_bullets`의 요소 수가 `MAX_IN_GAME_BULLETS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        capture_point: CapturePoint,
        players: Vec<InGamePlayerStatusPullData>,
        removed_bullets: Vec<ObjectId>,
    ) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");
        assert!(
            removed_bullets.len() <= MAX_IN_GAME_BULLETS,
            "too many bullets!"
        );

        Self {
            capture_point,
            players,
            removed_bullets,
        }
    }
}

impl Packet for InGameStatusPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameStatus
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let num_removed_bullets = self.removed_bullets.len();
        let data_size = CapturePoint::byte_size()
            + u8::byte_size()
            + InGamePlayerStatusPullData::byte_size() * num_players
            + u16::byte_size()
            + ObjectId::byte_size() * num_removed_bullets;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.capture_point.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }
        data.extend_from_slice(&(num_removed_bullets as u16).to_big_endian_bytes());
        for id in self.removed_bullets.iter() {
            data.extend_from_slice(&id.to_big_endian_bytes());
        }

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameStatusPacket)
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

        // 플레이어의 수를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = CapturePoint::byte_size();
        let mut data = &bytes[offset..offset + size];
        let capture_point = CapturePoint::try_from_big_endian_bytes(data)?;

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
            size = InGamePlayerStatusPullData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(InGamePlayerStatusPullData::from_big_endian_bytes(data));
        }

        // 제거된 총알 식별자의 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let num_removed_bullets = u16::from_big_endian_bytes(data) as usize;

        // 총알 식별자를 가져옵니다.
        let mut removed_bullets = Vec::with_capacity(num_removed_bullets);
        for _ in 0..num_removed_bullets {
            offset = offset + size;
            size = ObjectId::byte_size();
            data = &bytes[offset..offset + size];
            removed_bullets.push(ObjectId::from_big_endian_bytes(data));
        }

        Some(Self {
            capture_point,
            players,
            removed_bullets,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{NetworkState, Permission, UserId};

    use super::*;

    #[test]
    fn test_in_game_status_packet() {
        let player_0 = InGamePlayerStatusPullData::new(
            UserId::new(81412),
            31,
            12,
            321,
            1341,
            9,
            412,
            Permission::User,
            true,
            false,
            NetworkState::Fair,
        );
        let players = vec![player_0];
        let removed_bullets = vec![ObjectId::new(95143), ObjectId::new(95144)];

        let origin = InGameStatusPacket::new(
            CapturePoint {
                capture_progress: 63.12,
                capture_score: [21.111, 2.111],
                capture_team: None,
            },
            players,
            removed_bullets,
        );
        let raw = origin.as_raw();
        let other = InGameStatusPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
