use crate::components::{BigEndian, ClientId, TryFromBigEndian};

use super::{Packet, PacketType, RawPacket};

/// 클라이언트가 서버에 연결되었을 때
/// 서버에서 클라이언트로 전송되는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectPacket {
    pub client_id: ClientId,
}

impl ConnectPacket {
    pub fn new(client_id: ClientId) -> Self {
        Self { client_id }
    }
}

impl Default for ConnectPacket {
    fn default() -> Self {
        // client_id의 기본 값은 NULL이어야 합니다.
        Self {
            client_id: ClientId::NULL,
        }
    }
}

impl Packet for ConnectPacket {
    fn packet_type() -> PacketType {
        PacketType::Connect
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = ClientId::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.client_id.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(ConnectPacket)
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

        // 클라이언트 식별자를 가져옵니다.
        let bytes = raw.data();
        let offset = 0;
        let size = ClientId::byte_size();
        let client_id = ClientId::try_from_big_endian_bytes(&bytes[offset..offset + size])?;

        Some(Self { client_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = ConnectPacket::new(ClientId::new(123456));
        let raw_packet = origin.as_raw();
        let other = ConnectPacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
