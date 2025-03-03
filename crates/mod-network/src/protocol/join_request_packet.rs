use crate::components::{BigEndian, UserId, WorldId};

use crate::protocol::{Packet, PacketType, RawPacket};

/// (커스텀) 게임 접속 또는 생성 요청 패킷
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRequestPacket {
    /// 요청 유저의 사용자 식별자
    pub user_id: UserId,
    /// 참가할 게임 월드의 식별자
    pub world_id: WorldId,
}

impl JoinRequestPacket {
    /// 새로운 JoinRequestPacket을 생성합니다.
    pub fn new(user_id: UserId, world_id: WorldId) -> Self {
        Self { user_id, world_id }
    }
}

impl Packet for JoinRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::JoinRequest
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + WorldId::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.world_id.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(JoinRequestPacket)
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

        // 월드 식별자를 가져옵니다.
        offset = offset + size;
        size = WorldId::byte_size();
        data = &bytes[offset..offset + size];
        let world_id = WorldId::from_big_endian_bytes(data);

        Some(Self { user_id, world_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_join_request_packet() {
        let client_id = UserId::new(123456);
        let world_id = WorldId::new(789);
        let origin = JoinRequestPacket::new(client_id, world_id);

        let raw_packet = origin.as_raw();
        let other = JoinRequestPacket::try_from_raw(raw_packet).unwrap();

        assert_eq!(origin, other);
    }
}
