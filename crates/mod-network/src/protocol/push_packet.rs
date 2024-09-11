use super::*;
use super::super::game_objects::Player;



/// 클라이언트에서 서버로 보내는 
/// 플레이어 정보를 갱신하기 위한 패킷
#[derive(Debug, PartialEq)]
pub struct PushPacket {
    pub player: Player, 
}

impl PushPacket {
    pub fn new(player: Player) -> Self {
        Self {
            player,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        Self { player: Player::from_bytes(raw.data()) }
    }

    pub fn as_raw(&self) -> RawPacket {
        RawPacket::new(PacketType::PUSH, &self.player.as_bytes())
    }
}
