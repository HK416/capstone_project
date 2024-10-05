use super::*;
use super::super::game_objects::Bullet;



/// 클라이언트에서 서버로 보내는 총알을 생성했음을 알리는 패킷
#[derive(Debug, PartialEq)]
pub struct ShotPacket {
    pub bullet: Bullet, 
}

impl ShotPacket {
    pub fn new(bullet: Bullet) -> Self {
        Self {
            bullet,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        Self { bullet: Bullet::from_bytes(raw.data()) }
    }

    pub fn as_raw(&self) -> RawPacket {
        RawPacket::new(PacketType::FIRED, &self.bullet.as_bytes())
    }
}
