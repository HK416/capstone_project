use std::mem::size_of;
use super::*;
use super::super::game_objects::{Player, BulletBlob};



/// 서버에서 클라이언트로 보내는 
/// 월드 정보 갱신을 위한 패킷.
#[derive(Debug, PartialEq)]
pub struct PullPacket {
    pub num_players: u16,   // max 65535 로 충분
    pub num_bullets: u16,
    pub players: Vec<Player>,
    pub bullets: Vec<BulletBlob>,
}

impl PullPacket {
    pub fn new(players: Vec<Player>, bullets: Vec<BulletBlob>) -> Self {
        Self {
            num_players: players.len() as u16,
            num_bullets: bullets.len() as u16,
            players,
            bullets,
        }
    }

    /// RawPacket내의 데이터는 유효하다고 가정한다.
    pub fn from_raw(raw: RawPacket) -> Self {
        let num_players = u16::from_be_bytes(raw.data()[0..2].try_into().unwrap());
        let num_bullets = u16::from_be_bytes(raw.data()[2..4].try_into().unwrap());

        let player_end = 4 + num_players as usize * size_of::<Player>();

        let players = raw.data()[4..player_end]
            .chunks_exact(size_of::<Player>())
            .map(|chunk| Player::from_bytes(chunk))
            .collect();

        let bullets = raw.data()[player_end..]
            .chunks_exact(size_of::<BulletBlob>())
            .map(|chunk| BulletBlob::from_bytes(chunk))
            .collect();

        Self {
            num_players,
            num_bullets,
            players,
            bullets,
        }
    }

    pub fn as_raw(&self) -> RawPacket {
        let mut bytes = Vec::with_capacity(
            4 + self.num_players as usize * size_of::<Player>()
              + self.num_bullets as usize * size_of::<BulletBlob>()
        );
        
        bytes.extend_from_slice(&self.num_players.to_be_bytes());
        bytes.extend_from_slice(&self.num_bullets.to_be_bytes());
        bytes.extend_from_slice(&self.players.iter()
            .flat_map(|player| player.as_bytes())
            .collect::<Vec<u8>>());
        bytes.extend_from_slice(&self.bullets.iter()
            .flat_map(|bullet| bullet.as_bytes())
            .collect::<Vec<u8>>());

        RawPacket::new(PacketType::PULL, &bytes)
    }
}
