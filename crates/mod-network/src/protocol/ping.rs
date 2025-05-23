use crate::{
    components::BigEndian,
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 반응속도 측정 패킷  
/// 서버에서 수신시 그대로 클라이언트에 전송(echo)  
#[derive(Debug, Clone, PartialEq)]
pub struct PingPacket {
    pub send_time: u128,
}

impl PingPacket {
    pub fn new(send_time: u128) -> Self {
        Self { send_time }
    }
}

impl Packet for PingPacket {
    fn packet_type() -> PacketType {
        PacketType::Ping
    }

    fn as_raw(&self) -> RawPacket {
        let data = self.send_time.to_big_endian_bytes();
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

        let send_time = u128::from_big_endian_bytes(raw.data());

        Some(Self { send_time })
    }
}