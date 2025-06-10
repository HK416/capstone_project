//! 클라이언트가 로비 장면에 있을 때 데이터 갱신 요청에 응답하는 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::BigEndian,
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 로비 장면에 있을 때 클라이언트엣 서버로 보내는 데이터 갱신 응답 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbyPushPacket {
    pub epoch: u64,
}

impl LobbyPushPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(epoch: u64) -> Self {
        Self { epoch }
    }
}

impl Packet for LobbyPushPacket {
    fn packet_type() -> PacketType {
        PacketType::LobbyPush
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = u64::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LobbyPushPacket)
            )
        };

        RawPacket::new(Self::packet_type(), &data)
    }

    #[allow(unused_mut)]
    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type! (SRC:{:?}), DST({:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        // 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u64::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = u64::from_big_endian_bytes(data);

        Some(Self { epoch })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lobby_push_packet() {
        let origin = LobbyPushPacket::new(13);
        let raw = origin.as_raw();
        let other = LobbyPushPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
