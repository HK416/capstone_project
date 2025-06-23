//! 패킷 헤더와 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, TryFromBigEndian},
    protocol::PacketType,
};

pub type PacketSize = u16;

/// 고정된 크기를 갖는 패킷의 헤더
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketHeader {
    pub packet_size: PacketSize,
    pub packet_type: PacketType,
}

impl BigEndian for PacketHeader {
    fn byte_size() -> usize {
        PacketSize::byte_size() + PacketType::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.packet_size.to_big_endian_bytes());
        bytes.extend_from_slice(&self.packet_type.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PacketHeader)
            );
        }

        bytes
    }
}

impl Default for PacketHeader {
    fn default() -> Self {
        Self {
            packet_size: 0,
            packet_type: PacketType::default(),
        }
    }
}

impl TryFromBigEndian for PacketHeader {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PacketHeader)
            )
        };

        // 패킷의 크기를 가져옵니다.
        let mut offset = 0;
        let mut size = PacketSize::byte_size();
        let mut data = &bytes[offset..offset + size];
        let packet_size = PacketSize::from_big_endian_bytes(data);

        // 패킷 종류를 가져옵니다.
        offset = offset + size;
        size = PacketType::byte_size();
        data = &bytes[offset..offset + size];
        let packet_type = PacketType::try_from_big_endian_bytes(data)?;

        Some(Self {
            packet_size,
            packet_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header() {
        let origin = PacketHeader {
            packet_size: 65524,
            packet_type: PacketType::Ping,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = PacketHeader::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
