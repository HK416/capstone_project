use super::PacketType;
use super::RawPacket;
use super::super::game_objects::Player;
use super::super::components::StageKind;
use super::super::components::BigEndian;


#[derive(Debug, PartialEq)]
pub struct InitStagePacket {
    pub num_players: u32,
    pub players: Vec<Player>,
    pub stage_kind: StageKind,
}

impl InitStagePacket {
    pub fn new(players: Vec<Player>, stage_kind: StageKind) -> Self {
        Self {
            num_players: players.len() as u32,
            players,
            stage_kind,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let data = raw.data();
        let num_players = u32::from_big_endian_bytes(&data[..size_of::<u32>()]);
        let boundary = size_of::<u32>() + size_of::<Player>() * num_players as usize;
        let players = data[size_of::<u32>()..boundary]
            .chunks_exact(size_of::<Player>())
            .map(|chunk| Player::from_bytes(chunk))
            .collect::<Vec<_>>();
        let stage_kind = StageKind::from_big_endian_bytes(&data[boundary..]);

        Self {
            num_players,
            players,
            stage_kind,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut bytes = Vec::with_capacity(size_of::<u32>() + size_of::<Player>() * self.num_players as usize + size_of::<StageKind>());
        bytes.extend_from_slice(&self.num_players.to_big_endian_bytes());
        bytes.extend_from_slice(&self.players.iter()
            .flat_map(|player| player.as_bytes())
            .collect::<Vec<u8>>());
        bytes.extend_from_slice(&self.stage_kind.to_big_endian_bytes());

        RawPacket::new(PacketType::InitStage, &bytes)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_stage_packet() {
        let players = vec![
            Player::default(),
            Player::default(),
            Player::default(),
            Player::default(),
            Player::default(),
        ];
        let stage_kind = StageKind::School;
        let packet = InitStagePacket::new(players.clone(), stage_kind);
        let raw = packet.as_raw();
        let packet2 = InitStagePacket::from_raw(raw);

        assert_eq!(packet, packet2);
        assert_eq!(packet2.num_players, players.len() as u32);
        assert_eq!(packet2.players, players);
        assert_eq!(packet2.stage_kind, stage_kind);
    }
}