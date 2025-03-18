use crate::{
    components::{BigEndian, Version},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 서버와 연결됐을 때 클라이언트의 버전 정보를 전달하는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVerifyPacket {
    pub version: Version,
}

impl ClientVerifyPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ClientVerifyPacket {
    fn default() -> Self {
        Self {
            version: Version::new(),
        }
    }
}

impl Packet for ClientVerifyPacket {
    fn packet_type() -> PacketType {
        PacketType::ClientVerify
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = Version::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.version.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(ClientVerifyPacket)
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
                Self::packet_type()
            );
            return None;
        }

        // 프로그램 버전 정보를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = Version::byte_size();
        let mut data = &bytes[offset..offset + size];
        let version = Version::from_big_endian_bytes(data);

        Some(Self { version })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_verify_packet() {
        let origin = ClientVerifyPacket::new();
        let raw = origin.as_raw();
        let other = ClientVerifyPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
