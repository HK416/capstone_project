use std::mem::size_of;
use super::*;
use super::super::game_objects::Player;


#[derive(Debug, PartialEq)]
pub struct InitPacket {
    pub client_id: u32,
    // pub map_data: MapData,
    pub world: Vec<Player>,
}

impl InitPacket {
    pub fn new(client_id: u32, world: Vec<Player>) -> Self {
        Self {
            client_id,
            world,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let client_id = u32::from_be_bytes(raw.data()[0..size_of::<u32>()].try_into().unwrap());
        let world = raw.data()[size_of::<u32>()..]
            .chunks_exact(size_of::<Player>())
            .map(|chunk| Player::from_bytes(chunk))
            .collect();

        Self {
            client_id,
            world,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut bytes = Vec::with_capacity(size_of::<u32>() + self.world.len() * size_of::<Player>());
        bytes.extend_from_slice(&self.client_id.to_be_bytes());
        bytes.extend_from_slice(&self.world.iter()
            .flat_map(|player| player.as_bytes())
            .collect::<Vec<u8>>());

        RawPacket::new(PacketType::INIT, &bytes)
    }
}
