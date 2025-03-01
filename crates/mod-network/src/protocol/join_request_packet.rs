use crate::components::{BigEndian, ClientId,WorldId,};
use crate:: components::system::JoinFailedReason;

use crate::protocol::{Packet,PacketType, RawPacket};

/// (커스텀) 게임 접속 또는 생성 요청 패킷
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRequestPacket {
    /// 요청 유저의 클라이언트 식별자
    pub client_id: ClientId,
    /// 참가할 게임 월드의 식별자
    pub world_id: WorldId,
}

impl JoinRequestPacket {
    /// 새로운 JoinRequestPacket을 생성합니다.
    pub fn new(client_id: ClientId, world_id: WorldId) -> Self {
        Self { client_id, world_id }
    }
}

impl Packet for JoinRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::JoinRequest
    }

    fn as_raw(&self) -> RawPacket {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.client_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.world_id.to_big_endian_bytes());

        RawPacket::new(Self::packet_type(), &bytes)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        if raw.packet_type() != Self::packet_type() {
            return None;
        }

        let data = raw.data();
        if data.len() != ClientId::byte_size() + WorldId::byte_size() {
            return None;
        }

        let client_id = ClientId::from_big_endian_bytes(&data[0..ClientId::byte_size()]);
        let world_id = WorldId::from_big_endian_bytes(&data[ClientId::byte_size()..]);

        Some(Self { client_id, world_id })
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::join_failed_packet::JoinFailedPacket;

    use super::*;

    #[test]
    fn validation_test_join_request_packet() {
        let client_id = ClientId::new(123456);
        let world_id = WorldId::new(789);
        let origin = JoinRequestPacket::new(client_id, world_id);

        let raw_packet = origin.as_raw();
        let other = JoinRequestPacket::try_from_raw(raw_packet).unwrap();

        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_join_failed_packet() {
        let origin = JoinFailedPacket::new(JoinFailedReason::Banned);
        let raw_packet = origin.as_raw();
        let other = JoinFailedPacket::try_from_raw(raw_packet).unwrap();

        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_join_failed_reason() {
        let reason = JoinFailedReason::FullCapacity;
        let bytes = reason.to_big_endian_bytes();
        let other = JoinFailedReason::from_big_endian_bytes(&bytes);

        assert_eq!(reason, other);
    }
}
