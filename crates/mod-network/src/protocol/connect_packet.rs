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

        RawPacket::new(PacketType::CONNECT, &bytes)
    }
}