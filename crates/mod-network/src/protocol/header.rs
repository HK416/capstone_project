use std::mem::size_of;
use super::super::components::{BigEndian, TryFromBigEndian};


pub type PacketSize = u16;


// 새로운 패킷 추가시 여기에 추가
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PacketType {
    Raw = 0,
    Connect = 1,
    EnterStage = 2,
    InitStage = 3,
    PullStage = 4,
    PushStatus = 5,
}

impl BigEndian for PacketType {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for PacketType {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(PacketType::Raw),
            1 => Some(PacketType::Connect),
            2 => Some(PacketType::EnterStage),
            3 => Some(PacketType::InitStage),
            4 => Some(PacketType::PullStage),
            5 => Some(PacketType::PushStatus),
            _ => None,
        }
    }
}


#[repr(packed)]
pub struct PacketHeader {
    pub size: PacketSize,
    pub packet_type: PacketType,
}

impl PacketHeader {
    pub fn as_bytes(&self) -> [u8; size_of::<PacketHeader>()] {
        let mut bytes = [0; size_of::<PacketHeader>()];
        bytes[..size_of::<PacketSize>()].copy_from_slice(&self.size.to_be_bytes());
        bytes[size_of::<PacketSize>()..].copy_from_slice(&self.packet_type.to_big_endian_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        let size = PacketSize::from_be_bytes([data[0], data[1]]);       // size가 변하면 이 코드 수정
        let packet_type = PacketType::from_big_endian_bytes(&data[2..]);

        Self {
            size,
            packet_type,
        }
    }
}

impl std::fmt::Debug for PacketHeader {     // packed 와 derive(Debug)가 충돌하여 직접 구현
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.size;

        f.debug_struct("PacketHeader")
            .field("size", &size)
            .field("packet_type", &self.packet_type)
            .finish()
    }
}

impl std::cmp::PartialEq for PacketHeader { // packed 와 derive(PartialEq)가 충돌하여 직접 구현
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.packet_type == other.packet_type
    }
}






#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_type() {
        // match로 변환하는 과정에서 순서가 바뀌는 등의 문제 발생시 테스트 실패
        assert_eq!(PacketType::from_big_endian_bytes(&[PacketType::Raw as u8]), PacketType::Raw);
        assert_eq!(PacketType::from_big_endian_bytes(&[PacketType::Connect as u8]), PacketType::Connect);
        assert_eq!(PacketType::from_big_endian_bytes(&[PacketType::EnterStage as u8]), PacketType::EnterStage);
        assert_eq!(PacketType::from_big_endian_bytes(&[PacketType::InitStage as u8]), PacketType::InitStage);
        assert_eq!(PacketType::from_big_endian_bytes(&[PacketType::PullStage as u8]), PacketType::PullStage);
        assert_eq!(PacketType::from_big_endian_bytes(&[PacketType::PushStatus as u8]), PacketType::PushStatus);
    }
}