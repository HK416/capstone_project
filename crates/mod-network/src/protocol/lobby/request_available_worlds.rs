use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

// 클라이언트에서 서버로 보내는 접속 가능한 월드 리스트를 요청하는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestAvailableWorldsPacket {
    /// 사용자 계정 식별자
    pub user_id: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl RequestAvailableWorldsPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(user_id: UserId, token: LoginToken) -> Self {
        Self { user_id, token }
    }
}

impl Packet for RequestAvailableWorldsPacket {
    fn packet_type() -> PacketType {
        PacketType::RequestAvailableWorlds
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RequestAvailableWorldsPacket)
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

        // 사용자 계정 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { user_id, token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_select_packet() {
        let origin =
            RequestAvailableWorldsPacket::new(UserId::new(12351432), LoginToken::new(1513425161));
        let raw = origin.as_raw();
        let other = RequestAvailableWorldsPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
