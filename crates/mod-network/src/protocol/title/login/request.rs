//! 게임 로그인 요청 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 서버로 보내는 로그인 요청 패킷입니다.
///
/// # Note
/// 현재 이 패킷은 어떤 데이터도 담고 있지 않습니다.
///
/// # Warnings
/// 이 패킷은 암호화 후 전송되어야 합니다.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequestPacket {
    /// NULL을 받으면 새로운 아이디를 할당합니다.
    pub uid: UserId,
}

impl LoginRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(uid: UserId) -> Self {
        Self { uid }
    }
}

impl Packet for LoginRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginRequest
    }

    #[allow(unused_mut)]
    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginRequestPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
    }

    #[allow(unused_mut)]
    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type. (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        // uid를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        Some(Self { uid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_packet() {
        let origin = LoginRequestPacket::new(UserId::new(123456));
        let raw = origin.as_raw();
        let other = LoginRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
