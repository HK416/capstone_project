use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 서버로 보내는 커스텀 게임 대기실 나가기 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGameLeavePacket {
    pub user_id: UserId,
    pub token: LoginToken,
}

impl CustomGameLeavePacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(user_id: UserId, token: LoginToken) -> Self {
        Self { user_id, token }
    }
}

impl Packet for CustomGameLeavePacket {
    fn packet_type() -> PacketType {
        PacketType::CustomGameLeave
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
                stringify!(CustomGameLeavePacket)
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

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { user_id, token })
    }
}

/// 클라이언트의 커스텀 게임 대기실 상태 갱신 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGamePushStatusPacket {
    pub user_id: UserId,
    pub token: LoginToken,
    pub ready: bool,
}

impl CustomGamePushStatusPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(user_id: UserId, token: LoginToken, ready: bool) -> Self {
        Self {
            user_id,
            token,
            ready,
        }
    }
}

impl Packet for CustomGamePushStatusPacket {
    fn packet_type() -> PacketType {
        PacketType::CustomGamePushStatus
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size() + u8::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(
            &match self.ready {
                true => 0xFFu8,
                false => 0x00u8,
            }
            .to_big_endian_bytes(),
        );

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CustomGamePushStatusPacket)
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

        // 사용자 식별자를 가져옵니다.
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

        // 플레이어 상태를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let ready_field = u8::from_big_endian_bytes(data);
        let ready = ready_field == 0xFF;

        Some(Self {
            user_id,
            token,
            ready,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_game_leave_packet() {
        let user_id = UserId::new(851351);
        let token = LoginToken::new(501859034151);

        let origin = CustomGameLeavePacket::new(user_id, token);
        let raw = origin.as_raw();
        let other = CustomGameLeavePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_custom_game_push_status_packet() {
        let user_id = UserId::new(851351);
        let token = LoginToken::new(501859034151);
        let ready = true;

        let origin = CustomGamePushStatusPacket::new(user_id, token, ready);
        let raw = origin.as_raw();
        let other = CustomGamePushStatusPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
