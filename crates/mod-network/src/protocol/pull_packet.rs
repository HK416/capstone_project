use std::mem::size_of;
use super::*;
use super::super::game_objects::Player;



/// 서버에서 클라이언트로 보내는 
/// 월드 정보 갱신을 위한 패킷.
#[derive(Debug, PartialEq)]
pub struct PullPacket {
    pub world: Vec<Player>,
}

impl PullPacket {
    pub fn new(world: Vec<Player>) -> Self {
        Self {
            world,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        Self {
            world: bytemuck::cast_slice(raw.data()).iter().cloned().collect()
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        RawPacket::new(PacketType::PULL, bytemuck::cast_slice(&self.world))
    }
}
