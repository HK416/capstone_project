use crate::components::TryFromBigEndian;

use super::*;
use super::super::game_objects::Player;
use super::super::components::BigEndian;
use mod_math::LatLon;



/// 클라이언트에서 서버로 보내는 
/// 플레이어 정보를 갱신하기 위한 패킷
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct PushStatusPacket {
    pub player: Player,
    pub move_direction: [f32; 3], // XY평면상의 플레이어 이동 방향
    // pub look_direction: LatLon, // 카메라 방향
    // pub move_direction: u8, // 8방향 입력 정보
    // pub jump: bool, // 점프키 입력 여부
}

impl PushStatusPacket {
    /// 패킷의 바이트 단위 크기입니다. 
    pub const SIZE: usize = size_of::<Player>() + 12;
}

impl Default for PushStatusPacket {
    fn default() -> Self {
        Self { 
            player: Player::default(), 
            move_direction: [0.0, 0.0, 1.0] 
        }
    }
}

impl BigEndian for PushStatusPacket {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.player.to_big_endian_bytes());
        bytes.extend_from_slice(&self.move_direction.to_big_endian_bytes());
        bytes
    }
}

impl TryFromBigEndian for PushStatusPacket {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let mut start = 0;
        let mut end = start + size_of::<Player>();
        let player = Player::try_from_big_endian_bytes(&bytes[start..end])?;

        start = end;
        end = start + 12;
        let move_direction = <[f32; 3]>::from_big_endian_bytes(&bytes[start..end]);
        Some(Self { player, move_direction })
    }
}

impl PushStatusPacket {
    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        Self::from_big_endian_bytes(&raw.data())
    }

    pub fn as_raw(&self) -> RawPacket {
        RawPacket::new(PacketType::PushStatus, &self.to_big_endian_bytes())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_status_packet() {
        let packet = PushStatusPacket::default();
        let raw = packet.as_raw();
        let packet2 = PushStatusPacket::from_raw(raw);

        assert_eq!(packet, packet2);
    }
}