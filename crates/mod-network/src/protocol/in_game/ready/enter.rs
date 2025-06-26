//! 인게임 장면에 진입 하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::BigEndian,
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 인게임 장면 진입 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameEnterNotifyPacket {
    /// 남은 시작 시간
    pub remaining_time_ms: u16,
}

impl InGameEnterNotifyPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(remaining_time_ms: u16) -> Self {
        Self { remaining_time_ms }
    }
}

impl Packet for InGameEnterNotifyPacket {
    fn packet_type() -> PacketType {
        PacketType::InGameReadyNotify
    }

    fn as_raw(&self) -> crate::protocol::RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = u16::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.remaining_time_ms.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameEnterNotifyPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
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

        // 남은 시간을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let remaining_time_ms = u16::from_big_endian_bytes(data);

        Some(Self { remaining_time_ms })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_game_enter_notify_packet() {
        let origin = InGameEnterNotifyPacket::new(10000);
        let raw = origin.as_raw();
        let other = InGameEnterNotifyPacket::from_raw(raw);

        // 원본과 일치하는지 비교합니다.
        assert_eq!(origin, other);
    }
}
