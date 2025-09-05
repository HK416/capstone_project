//! 기본 패킷과 관련된 코드를 관리합니다.
//!

mod header;
mod ping;
mod query;
mod types;

use std::io::{Error, ErrorKind};

use crate::components::{BigEndian, TryFromBigEndian};

pub use self::{header::*, ping::*, query::*, types::*};

/// 클라이언트-서버 간의 통신에 사용되는 기본 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    header: PacketHeader,
    data: Vec<u8>,
}

impl RawPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new<T>(packet_type: PacketType, data: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        let data = data.into();
        let packet_size = (PacketHeader::byte_size() + data.len()) as u16;
        let header = PacketHeader {
            packet_type,
            packet_size,
        };
        Self { header, data }
    }

    /// 패킷 타입을 반환합니다.
    pub fn packet_type(&self) -> PacketType {
        self.header.packet_type
    }

    /// 전체 패킷 크기를 반환합니다. (`PacketHeader::byte_size() + data.len()`)
    pub fn packet_size(&self) -> u16 {
        self.header.packet_size
    }

    /// 데이터를 가져옵니다.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// 바이트 스트림을 반환합니다.
    pub fn as_bytes(&self) -> Vec<u8> {
        let packet_size = self.header.packet_size as usize;
        let mut bytes = Vec::with_capacity(packet_size);
        bytes.extend_from_slice(&self.header.to_big_endian_bytes());
        bytes.extend_from_slice(&self.data);

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                packet_size,
                "the size of the byte array and the size of the packet are different!"
            );
        }

        bytes
    }

    /// 바이트 스트림으로부터 패킷을 생성합니다.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // 바이트 배열의 크기가 패킷 헤더의 크기보다 작은지 확인한다.
        let header_size = PacketHeader::byte_size();
        if bytes.len() < header_size {
            log::error!(
                "the size of the byte array is smaller than `{}`.",
                stringify!(PacketHeader)
            );
            return Err(Error::new(ErrorKind::InvalidData, "invalid data"));
        }

        // 패킷 헤더를 가져옵니다.
        let result = PacketHeader::try_from_big_endian_bytes(&bytes[0..header_size]);
        let header = match result {
            Some(header) => header,
            None => return Err(Error::new(ErrorKind::InvalidData, "invalid data")),
        };

        // 바이트 배열의 크기가 패킷의 크기보다 작은지 확인한다.
        let packet_size = header.packet_size as usize;
        if bytes.len() < packet_size {
            log::error!("the size of the byte array is smaller than the size of the packet");
            return Err(Error::new(ErrorKind::InvalidData, "invalid data"));
        }

        // 데이터를 가져옵니다.
        let data = bytes[header_size..packet_size].to_vec();

        Ok(Self { header, data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_raw_packet() {
        let data = vec![1, 2, 3, 4, 5];
        let origin = RawPacket::new(PacketType::Raw, data);
        let bytes = origin.as_bytes();
        let other = RawPacket::try_from_bytes(&bytes).unwrap();

        // 바이트 배열이 Big-endian인지 확인합니다.
        assert_eq!(bytes, vec![0, 8, 0, 1, 2, 3, 4, 5]);
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
