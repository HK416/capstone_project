//! 클라이언트가 커스텀 게임 대기실 장면에 있을 때 플레이어 강제 퇴장 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트의 커스텀 게임 대기실 강제 퇴장 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomPlayerBanRequestPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 대상 사용자 식별자
    pub target: UserId,
}

impl RoomPlayerBanRequestPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// 주어진 사용자 식별자와 대상 사용자 식별자가 같은 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(uid: UserId, token: LoginToken, target: UserId) -> Self {
        assert_ne!(uid, target);
        Self { uid, token, target }
    }
}

impl Packet for RoomPlayerBanRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::RoomPlayerBanRequest
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size() + LoginToken::byte_size() + UserId::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.target.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RoomPlayerBanRequestPacket)
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

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        // 대상 사용자 식별자를 가져옵니다.
        offset = offset + size;
        size = UserId::byte_size();
        data = &bytes[offset..offset + size];
        let target = UserId::from_big_endian_bytes(data);

        (uid != target).then_some(Self { uid, token, target })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_room_player_ban_request_packet() {
        RoomPlayerBanRequestPacket::new(
            UserId::new(123),
            LoginToken::new(124512351),
            UserId::new(123),
        );
    }

    #[test]
    fn test_room_player_ban_request_packet() {
        let origin = RoomPlayerBanRequestPacket::new(
            UserId::new(12315),
            LoginToken::new(68190513),
            UserId::new(908151431),
        );
        let raw = origin.as_raw();
        let other = RoomPlayerBanRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
