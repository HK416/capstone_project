use super::PacketType;
use super::RawPacket;
use super::super::components::ClientId;
use super::super::components::BigEndian;


#[derive(Debug, PartialEq)]
pub struct ConnectPacket {
    pub client_id: ClientId
}

impl ConnectPacket {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let client_id = ClientId::from_big_endian_bytes(&raw.data());

        Self {
            client_id
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let bytes = self.client_id.to_big_endian_bytes();

        RawPacket::new(PacketType::Connect, &bytes)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_packet() {
        let client_id = ClientId::new(1234);
        let packet = ConnectPacket::new(client_id);
        let raw = packet.as_raw();
        let packet2 = ConnectPacket::from_raw(raw);

        assert_eq!(packet, packet2);
        assert_eq!(packet2.client_id, client_id);
    }
}