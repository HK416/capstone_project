//! 클라이언트가 로비 장면에 있을 때 커스텀 게임에 참여하기 위한 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, LoginToken, UserId, WorldId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 서버로 보내는 커스텀 게임 참가 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRequestPacket {
    /// 게임 월드 식별자
    pub id: WorldId,
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl JoinRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(id: WorldId, uid: UserId, token: LoginToken) -> Self {
        Self { id, uid, token }
    }
}

impl Packet for JoinRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::JoinRequest
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = WorldId::byte_size() + UserId::byte_size() + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.id.to_big_endian_bytes());
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(JoinRequestPacket)
            )
        };

        RawPacket::new(Self::packet_type(), &data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type! (SRC:{:?}, DST:{:?})",
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
        let id = WorldId::from_big_endian_bytes(data);

        // 사용자 식별자를 가져옵니다.
        offset = offset + size;
        size = UserId::byte_size();
        data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { id, uid, token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_request_packet() {
        let origin = JoinRequestPacket::new(
            WorldId::new(12345),
            UserId::new(131543561),
            LoginToken::new(25161643514),
        );
        let raw = origin.as_raw();
        let other = JoinRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
