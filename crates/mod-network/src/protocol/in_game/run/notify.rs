//! 게임 조작 초기화 요청 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameControlLosePacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl InGameControlLosePacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(uid: UserId, token: LoginToken) -> Self {
        Self { uid, token }
    }
}

impl Packet for InGameControlLosePacket {
    fn packet_type() -> PacketType {
        PacketType::InGameControlLose
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size() + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameControlLosePacket)
            )
        };

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

        // 로그인 토근을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { uid, token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_game_control_lose_packet() {
        let origin = InGameControlLosePacket::new(UserId::new(5413241), LoginToken::new(148941241));
        let raw = origin.as_raw();
        let other = InGameControlLosePacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
