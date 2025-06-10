//! 게임 로그인 요청 패킷과 관련된 코드를 관리합니다.
//!

use crate::protocol::{Packet, PacketType, RawPacket};

/// 클라이언트가 서버로 보내는 로그인 요청 패킷입니다.
///
/// # Note
/// 현재 이 패킷은 어떤 데이터도 담고 있지 않습니다.
///
/// # Warnings
/// 이 패킷은 암호화 후 전송되어야 합니다.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequestPacket;

impl LoginRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new() -> Self {
        Self
    }
}

impl Packet for LoginRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginRequest
    }

    #[allow(unused_mut)]
    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = 0;
        let mut data = Vec::with_capacity(data_size);

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginRequestPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

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

        Some(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_packet() {
        let origin = LoginRequestPacket::new();
        let raw = origin.as_raw();
        let other = LoginRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
