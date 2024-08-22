use std::mem::size_of;


pub type PacketSize = u16;


// 새로운 패킷 추가시 여기에 추가
#[derive(Debug, PartialEq)]
pub struct PacketType(u8);
impl PacketType {
    pub const RAW: Self = Self(0);
    pub const MESSAGE: Self = Self(1);
    pub const MOVE: Self = Self(2);
    pub const ANIMATION: Self = Self(3);
    pub const UPDATE: Self = Self(4);
    pub const INIT: Self = Self(5);

    // 크기가 u8보다 커지면 이 함수 활성화
    // pub fn as_bytes(&self) -> [u8; size_of::<PacketType>()] {
    //     self.0.to_be_bytes()
    // }
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
        bytes[size_of::<PacketSize>()] = self.packet_type.0;
        // bytes[size_of::<PacketSize>()..].copy_from_slice(&self.packet_type.to_be_bytes());   // 크기가 u8보다 커지면 이 코드 활성화
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        let size = PacketSize::from_be_bytes([data[0], data[1]]);       // size가 변하면 이 코드 수정
        let packet_type = PacketType(data[2]);                          // 크기가 u8보다 커지면 이 코드 수정

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