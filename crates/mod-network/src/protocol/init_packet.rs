use std::mem::size_of;
use super::*;


#[derive(Debug, PartialEq)]
pub struct InitPacket {
    pub client_id: u32,
}

impl InitPacket {
    pub fn new(client_id: u32) -> Self {
        Self {
            client_id,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let client_id = u32::from_be_bytes(raw.data()[0..size_of::<u32>()].try_into().unwrap());

        Self {
            client_id,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        RawPacket::new(PacketType::INIT, &self.client_id.to_be_bytes())
    }
}
