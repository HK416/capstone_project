use crate::{
    components::{BigEndian, WorldId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 접속 가능한 월드 리스트 패킷입니다.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableWorldsPacket {
    pub worlds: Vec<WorldId>,
}

impl AvailableWorldsPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(worlds: Vec<WorldId>) -> Self {
        Self { worlds }
    }
}

impl Packet for AvailableWorldsPacket {
    fn packet_type() -> PacketType {
        PacketType::AvailableWorlds
    }

    fn as_raw(&self) -> RawPacket {
        let num_worlds = self.worlds.len();
        let data_size = u8::byte_size() + num_worlds * WorldId::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(num_worlds as u8).to_big_endian_bytes());
        for world in &self.worlds {
            data.extend_from_slice(&world.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(AvailableWorldsPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
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

        let bytes = raw.data();
        
        // 월드 수를 가져옵니다.
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let num_worlds = u8::from_big_endian_bytes(data) as usize;

        // 월드 정보를 가져옵니다.
        let mut worlds = Vec::with_capacity(num_worlds);
        for _ in 0..num_worlds {
            offset = offset + size;
            size = WorldId::byte_size();
            data = &bytes[offset..offset + size];
            worlds.push(WorldId::from_big_endian_bytes(data));
        }

        Some(Self {
            worlds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_select_response_packet() {
        let origin = AvailableWorldsPacket::new(
            vec![
                WorldId::new(1),
                WorldId::new(2),
                WorldId::new(3),
            ],
        );
        let raw = origin.as_raw();
        let other = AvailableWorldsPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
