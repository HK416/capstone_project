use std::mem::size_of;
use super::*;


#[derive(Debug, PartialEq)]
pub struct MovePacket {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl MovePacket {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let data = raw.data();
        let x = f32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let y = f32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let z = f32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        Self { x, y, z }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut data = Vec::with_capacity(size_of::<f32>() * 3);
        data.extend_from_slice(&self.x.to_be_bytes());
        data.extend_from_slice(&self.y.to_be_bytes());
        data.extend_from_slice(&self.z.to_be_bytes());

        RawPacket::new(PacketType::MOVE, &data)
    }
}
















#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_packet() {
        let packet = MovePacket::new(1.0, 2.0, 4.0);
        let raw = packet.as_raw();
        
        assert_eq!(raw.packet_type(), PacketType::MOVE);
        assert_eq!(
            raw.data(), 
            &[
                0b00111111, 0b10000000, 0b00000000, 0b00000000, // 0 01111111 00000000000000000000000
                0b01000000, 0b00000000, 0b00000000, 0b00000000, // 0 10000000 00000000000000000000000
                0b01000000, 0b10000000, 0b00000000, 0b00000000, // 0 10000001 00000000000000000000000
            ]
        );
    }
}