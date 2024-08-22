use std::mem::size_of;
use super::*;


#[derive(Debug, PartialEq)]
pub struct MessagePacket {
    pub time: u128,
    pub msg: String,
}

impl MessagePacket {
    pub fn new(time: u128, msg: &str) -> Self {
        Self {
            time,
            msg: msg.to_string(),
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let time = u128::from_be_bytes(raw.data()[0..size_of::<u128>()].try_into().unwrap());
        let msg = String::from_utf8_lossy(&raw.data()[size_of::<u128>()..]);

        Self {
            time, 
            msg: msg.to_string(),
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut data = Vec::with_capacity(size_of::<u128>() + self.msg.len());
        data.extend_from_slice(&self.time.to_be_bytes());
        data.extend_from_slice(self.msg.as_bytes());

        // let data = [&self.time.to_be_bytes(), self.msg.as_bytes()].concat();     // 위 코드가 더 빠르다

        RawPacket::new(PacketType::MESSAGE, &data)
    }
}





















#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_packet() {
        let time = 0x0123456789abcdef;
        let msg = "Hello, World!";
        let packet = MessagePacket::new(time, msg);
        let raw = packet.as_raw();

        let len = size_of::<u128>() + msg.len();
        assert_eq!(len, raw.data().len());

        let bytes = raw.as_bytes();
        assert_eq!([0, (len + size_of::<PacketHeader>()) as u8, 1], bytes[..size_of::<PacketHeader>()]);

        assert_eq!(
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF], 
            bytes[size_of::<PacketHeader>()..size_of::<PacketHeader>() + size_of::<u128>()]
        );

        let new_packet = MessagePacket::from_raw(raw);

        assert_eq!(packet, new_packet);
    }
}