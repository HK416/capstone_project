use std::mem::size_of;
use super::*;


#[derive(Debug, PartialEq)]
pub struct RawPacket {
    header: PacketHeader,
    data: Vec<u8>,
}

impl RawPacket {
    pub fn new(packet_type: PacketType, data: &[u8]) -> Self {
        let size = (size_of::<PacketHeader>() + data.len()) as PacketSize;

        Self {
            header: PacketHeader {
                size,
                packet_type,
            },
            data: data.to_vec(),
        }
    }

    pub fn packet_type(&self) -> PacketType {
        self.header.packet_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.header.size as usize);
        data.extend_from_slice(&self.header.as_bytes());
        data.extend_from_slice(&self.data);

        data
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        if data.len() < size_of::<PacketHeader>() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid data"));
        }

        let header = PacketHeader::from_bytes(&data[0..size_of::<PacketHeader>()]);
        if data.len() < header.size as usize {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid data"));
        }

        let data = data[size_of::<PacketHeader>()..header.size as usize].to_vec();

        Ok(Self {
            header,
            data,
        })
    }
}













#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_packet() {
        let data = vec![1, 2, 3, 4, 5];
        let packet = RawPacket::new(PacketType::Raw, &data);

        let serialized = packet.as_bytes();
        assert_eq!(serialized, vec![0, 8, 0, 1, 2, 3, 4, 5]);   // big-endian

        let deserialized = RawPacket::from_bytes(&serialized).unwrap();
        assert_eq!(packet, deserialized);
    }
}