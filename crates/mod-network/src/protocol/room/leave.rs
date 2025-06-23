//! 클라이언트가 커스텀 게임 대기실 장면에 있을 때 게임 떠남 알림 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 커스텀 게임 떠남 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomLeaveNotifyPacket {
    /// 사용자 식별자
    pub uid: UserId,

    /// 로그인 토큰
    pub token: LoginToken,
}

impl RoomLeaveNotifyPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(uid: UserId, token: LoginToken) -> Self {
        Self { uid, token }
    }
}

impl Packet for RoomLeaveNotifyPacket {
    fn packet_type() -> PacketType {
        PacketType::RoomLeaveNotify
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RoomLeaveNotifyPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
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
    fn test_room_leave_notify_packet() {
        let origin = RoomLeaveNotifyPacket::new(UserId::new(851351), LoginToken::new(501859034151));
        let raw = origin.as_raw();
        let other = RoomLeaveNotifyPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
