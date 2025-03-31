use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 플레이어 동기화 정보를 갱신하기 위한 패킷
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSyncPacket {
    /// 사용자 식별자
    pub user_id: UserId,
    /// 사용자 로그인 토큰
    pub token: LoginToken,
    /// 완료 여부
    pub finish: bool,
}

impl PushSyncPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(user_id: UserId, token: LoginToken, finish: bool) -> Self {
        Self {
            user_id,
            token,
            finish,
        }
    }
}

impl Packet for PushSyncPacket {
    fn packet_type() -> PacketType {
        PacketType::PushSync
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size() + u8::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&(self.finish as u8 & 0x1).to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PushSyncPacket)
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
                Self::packet_type()
            );
            return None;
        }

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 사용자 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        // 완료 여부를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let finish = u8::from_big_endian_bytes(data) == 0x1;

        Some(Self {
            user_id,
            token,
            finish,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_sync_packet() {
        let origin = PushSyncPacket::new(UserId::new(151515), LoginToken::new(984511512), true);
        let raw_packet = origin.as_raw();
        let other = PushSyncPacket::from_raw(raw_packet);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
