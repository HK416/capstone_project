use super::*;
use super::super::game_objects::Player;



/// 클라이언트에서 서버로 보내는 
/// 플레이어 정보를 갱신하기 위한 패킷
#[derive(Debug, PartialEq)]
pub struct PushStatusPacket {
    // TODO: 이동방향, 시선, 발사 및 스킬사용 정보 등을 포함해야한다.
    pub player: Player, 
}

impl PushStatusPacket {
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
        RawPacket::new(PacketType::PushStatus, &self.player.as_bytes())
    }
}
