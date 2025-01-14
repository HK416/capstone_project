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
    // pub look_direction: LatLon, // 카메라 방향
    // pub move_direction: u8, // 8방향 입력 정보
    // pub jump: bool, // 점프키 입력 여부
}

impl PushStatusPacket {
    pub fn new(player: Player, look_direction: LatLon, move_direction: u8, jump: bool) -> Self {
        Self {
            player,
            // look_direction,
            // move_direction,
            // jump,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let data = raw.data();

        let mut start = 0;
        let mut end = start + size_of::<Player>();
        let player = Player::from_big_endian_bytes(&data[start..end]);

        // start = end;
        // end = start + size_of::<LatLon>();
        // let look_direction = LatLon::from_big_endian_bytes(&data[start..end]);

        // start = end;
        // end = start + size_of::<u8>();
        // let move_direction = u8::from_big_endian_bytes(&data[start..end]);

        // start = end;
        // end = start + size_of::<u8>();
        // let jump = u8::from_big_endian_bytes(&data[start..end]);

        Self { 
            player,
            // look_direction,
            // move_direction,
            // jump: jump != 0,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        // let mut bytes = Vec::with_capacity(size_of::<Player>() + size_of::<LatLon>() + size_of::<u8>() * 2);
        let mut bytes = Vec::with_capacity(size_of::<Player>());
        bytes.extend_from_slice(&self.player.to_big_endian_bytes());
        // bytes.extend_from_slice(&self.look_direction.to_big_endian_bytes());
        // bytes.extend_from_slice(&self.move_direction.to_big_endian_bytes());
        // bytes.extend_from_slice(&(self.jump as u8).to_big_endian_bytes());

        RawPacket::new(PacketType::PushStatus, &bytes)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_status_packet() {
        let player = Player::default();
        let look_direction = LatLon::default();
        let move_direction = 0;
        let jump = false;

        let packet = PushStatusPacket::new(player, look_direction, move_direction, jump);
        let raw = packet.as_raw();
        let packet2 = PushStatusPacket::from_raw(raw);

        assert_eq!(packet, packet2);
    }
}