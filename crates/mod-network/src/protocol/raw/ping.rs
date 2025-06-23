use crate::{
    components::BigEndian,
    protocol::{Packet, PacketType, RawPacket},
};

/// 핑을 측정하기 위해 전송되는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingTestPacket {
    pub value: u64,
}

impl PingTestPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
}

impl Packet for PingTestPacket {
    fn packet_type() -> PacketType {
        PacketType::Ping
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = u64::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.value.to_big_endian_bytes());

        // 생성된 바이트 스트림이 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PingTestPacket)
            )
        };

        RawPacket::new(Self::packet_type(), data)
    }

    #[allow(unused_mut)]
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

        // 값을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u64::byte_size();
        let mut data = &bytes[offset..offset + size];
        let value = u64::from_big_endian_bytes(data);

        Some(Self { value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_test_packet() {
        let origin = PingTestPacket::new(141513);
        let raw = origin.as_raw();
        let other = PingTestPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
