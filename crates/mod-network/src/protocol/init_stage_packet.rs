use super::PacketType;
use super::RawPacket;
use super::super::game_objects::Player;
use super::super::components::StageKind;
use super::super::components::BigEndian;


#[derive(Debug, PartialEq)]
pub struct InitStagePacket {
    pub num_players: u32,
    pub players: [Player; 10],
    pub stage_kind: StageKind,
}

impl InitStagePacket {
    pub fn new(num_players: u32, players: [Player; 10], stage_kind: StageKind) -> Self {
        Self {
            num_players,
            players,
            stage_kind,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let data = raw.data();
        let num_players = u32::from_big_endian_bytes(&data);
        let players = data[size_of::<u32>()..]
            .chunks_exact(size_of::<Player>())
            .map(|chunk| Player::from_bytes(chunk))
            .collect::<Vec<_>>()
            .try_into()                 // [Player; 10]으로 변환한다.
            .expect("out of bounds");
        let stage_kind = StageKind::from_big_endian_bytes(&data[size_of::<u32>() + size_of::<Player>() * 10..]);

        Self {
            num_players,
            players,
            stage_kind,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut bytes = Vec::with_capacity(size_of::<u32>() + size_of::<[Player; 10]>() + size_of::<StageKind>());
        bytes.extend_from_slice(&self.num_players.to_big_endian_bytes());
        bytes.extend_from_slice(&self.players.iter()
            .flat_map(|player| player.as_bytes())
            .collect::<Vec<u8>>());
        bytes.extend_from_slice(&self.stage_kind.to_big_endian_bytes());

        RawPacket::new(PacketType::INITSTAGE, &bytes)
    }
}