use crate::components::{BigEndian, Bullet, Epoch, Player, TryFromBigEndian};

use super::{Packet, PacketType, RawPacket};

/// 서버에서 클라이언트로 보내는
/// 월드 정보 갱신을 위한 패킷.
#[derive(Debug, Clone, PartialEq)]
pub struct PullStagePacket {
    pub epoch: Epoch,
    pub num_players: u16,
    pub players: Vec<Player>,
    pub num_bullets: u16,
    pub bullets: Vec<Bullet>,
}

impl PullStagePacket {
    pub fn new(epoch: Epoch, players: Vec<Player>, bullets: Vec<Bullet>) -> Self {
        Self {
            epoch,
            num_players: players.len() as u16,
            players,
            num_bullets: bullets.len() as u16,
            bullets,
        }
    }
}

impl Default for PullStagePacket {
    fn default() -> Self {
        Self {
            epoch: Epoch::new(0),
            num_players: 0,
            players: Vec::default(),
            num_bullets: 0,
            bullets: Vec::default(),
        }
    }
}

impl Packet for PullStagePacket {
    fn packet_type() -> PacketType {
        PacketType::PullStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = Epoch::byte_size()
            + u16::byte_size()
            + Player::byte_size() * self.num_players as usize
            + u16::byte_size()
            + Bullet::byte_size() * self.num_bullets as usize;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.num_players.to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }
        data.extend_from_slice(&self.num_bullets.to_big_endian_bytes());
        for bullet in self.bullets.iter() {
            data.extend_from_slice(&bullet.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InitStagePacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::warn!(
                "invalid packet type. (RAW:{:?}, PACKET:{:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        // 서버의 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = Epoch::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = Epoch::try_from_big_endian_bytes(data)?;

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u16::from_big_endian_bytes(data);

        // 플레이어 데이터를 가져옵니다.
        let mut count = num_players as usize;
        let mut players = Vec::with_capacity(count);
        while count > 0 {
            offset = offset + size;
            size = Player::byte_size();
            data = &bytes[offset..offset + size];
            players.push(Player::try_from_big_endian_bytes(data)?);
            count -= 1;
        }

        // 총알의 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let num_bullets = u16::from_big_endian_bytes(data);

        // 총알 데이터를 가져옵니다.
        let mut count = num_bullets as usize;
        let mut bullets = Vec::with_capacity(count);
        while count > 0 {
            offset = offset + size;
            size = Bullet::byte_size();
            data = &bytes[offset..offset + size];
            bullets.push(Bullet::try_from_big_endian_bytes(data)?);
            count -= 1;
        }

        Some(Self {
            epoch,
            num_players,
            players,
            num_bullets,
            bullets,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{ClientId, ObjectId};

    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = PullStagePacket::new(
            Epoch::new(0),
            vec![
                Player {
                    object_id: ObjectId::new(123456),
                    ..Default::default()
                },
                Player {
                    object_id: ObjectId::new(1),
                    ..Default::default()
                },
            ],
            vec![
                Bullet {
                    object_id: ObjectId::new(123455),
                    shooter_id: ClientId::new(1),
                    ..Default::default()
                },
                Bullet {
                    object_id: ObjectId::new(1),
                    shooter_id: ClientId::new(1),
                    ..Default::default()
                },
            ],
        );
        let raw_packet = origin.as_raw();
        let other = PullStagePacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
