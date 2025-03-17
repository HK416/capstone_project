use crate::{
    components::{
        BigEndian, CustomGamePlayer, JoinFailedReason, LoginToken, TryFromBigEndian, UserId,
        WorldId, MAX_CUSTOM_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 서버로 보내는 커스텀 게임에 참가를 요청하는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGameJoinRequestPacket {
    pub world_id: WorldId,
    pub user_id: UserId,
    pub token: LoginToken,
}

impl CustomGameJoinRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(world_id: WorldId, user_id: UserId, token: LoginToken) -> Self {
        Self {
            world_id,
            user_id,
            token,
        }
    }
}

impl Packet for CustomGameJoinRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::CustomGameJoinRequest
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = WorldId::byte_size() + UserId::byte_size() + LoginToken::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.world_id.to_big_endian_bytes());
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CustomGameJoinRequestPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
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

        // 월드 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = WorldId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let world_id = WorldId::from_big_endian_bytes(data);

        // 사용자 식별자를 가져옵니다.
        offset = offset + size;
        size = UserId::byte_size();
        data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self {
            world_id,
            user_id,
            token,
        })
    }
}

/// 커스텀 게임 참여에 실패했을 때 서버에서 클라이언트로 보내는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGameJoinFailedPacket {
    pub reason: JoinFailedReason,
}

impl CustomGameJoinFailedPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(reason: JoinFailedReason) -> Self {
        Self { reason }
    }
}

impl Packet for CustomGameJoinFailedPacket {
    fn packet_type() -> PacketType {
        PacketType::CustomGameJoinFailed
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = JoinFailedReason::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CustomGameJoinFailedPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

    #[allow(unused_mut)]
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

        // 실패 사유를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = JoinFailedReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = JoinFailedReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

/// 커스텀 게임 참여에 성공했을 때 서버에서 클라이언트로 보내는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGameJoinSuccessPacket {
    pub players: Vec<CustomGamePlayer>,
}

impl CustomGameJoinSuccessPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_CUSTOM_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn new(players: Vec<CustomGamePlayer>) -> Self {
        assert!(
            players.len() <= MAX_CUSTOM_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );
        Self { players }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_CUSTOM_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = CustomGamePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(iter.into_iter().collect())
    }
}

impl Packet for CustomGameJoinSuccessPacket {
    fn packet_type() -> PacketType {
        PacketType::CustomGameJoinSuccess
    }

    /// 패킷을 RawPacket으로 변환합니다.
    ///
    /// # Panics
    /// `players`의 요소 수가 `MAX_CUSTOM_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림 레이아웃
        // +-------------------+
        // | 참가 인원 수 (1byte) |
        // +-------------------+
        // | 사용자 정보          |
        // +-------------------+
        //
        let num_players = self.players.len();
        assert!(
            num_players <= MAX_CUSTOM_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );
        let data_size = u8::byte_size() + num_players * CustomGamePlayer::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
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
                stringify!(CustomGamejoinSuccessPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
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

        // 플레이어 수를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players > MAX_CUSTOM_GAME_PLAYERS {
            return None;
        }

        // 커스텀 게임 플레이어 정보를 가져옵니다.
        let mut players = Vec::with_capacity(MAX_CUSTOM_GAME_PLAYERS);
        size = CustomGamePlayer::byte_size();
        for _ in 0..num_players {
            offset = offset + size;
            data = &bytes[offset..offset + size];
            players.push(CustomGamePlayer::try_from_big_endian_bytes(data)?);
        }

        Some(Self { players })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_game_join_request_packet() {
        let world_id = WorldId::new(12345);
        let user_id = UserId::new(45678);
        let token = LoginToken::new(3141356);

        let origin = CustomGameJoinRequestPacket::new(world_id, user_id, token);
        let raw = origin.as_raw();
        let other = CustomGameJoinRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_custom_game_join_failed_packet() {
        let reason = JoinFailedReason::InProgress;

        let origin = CustomGameJoinFailedPacket::new(reason);
        let raw = origin.as_raw();
        let other = CustomGameJoinFailedPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
