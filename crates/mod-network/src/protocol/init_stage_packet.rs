use crate::components::{BigEndian, Epoch, ObjectId, Player, StageKind, TryFromBigEndian};

use super::{Packet, PacketType, RawPacket};

/// 클라이언트가 게임에 입장할 때
/// 서버에서 클라이언트로 전송되는 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InitStagePacket {
    /// 게임 스테이지 종류
    pub stage_kind: StageKind,
    /// 게임 월드의 초기 시대
    pub epoch: Epoch,
    /// 클라이언트 캐릭터의 오브젝트 식별자입니다.
    pub object_id: ObjectId,
    /// 클라이언트를 포함한 게임 월드에 존재하는 플레이어의 수
    pub num_players: u16,
    /// 클라이언트를 포함한 게임 월드 플레이어 데이터
    pub players: Vec<Player>,
}

impl InitStagePacket {
    pub fn new(
        stage_kind: StageKind,
        epoch: Epoch,
        object_id: ObjectId,
        players: Vec<Player>,
    ) -> Self {
        Self {
            stage_kind,
            epoch,
            object_id,
            num_players: players.len() as u16,
            players,
        }
    }
}

impl Default for InitStagePacket {
    // object_id 기본 값은 NULL이어야 합니다.
    fn default() -> Self {
        Self {
            stage_kind: StageKind::default(),
            epoch: Epoch::new(0),
            object_id: ObjectId::NULL,
            num_players: 0,
            players: Vec::default(),
        }
    }
}

impl Packet for InitStagePacket {
    fn packet_type() -> PacketType {
        PacketType::InitStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = StageKind::byte_size()
            + Epoch::byte_size()
            + ObjectId::byte_size()
            + u16::byte_size()
            + Player::byte_size() * self.num_players as usize;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.stage_kind.to_big_endian_bytes());
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.object_id.to_big_endian_bytes());
        data.extend_from_slice(&self.num_players.to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
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

        // 스테이지 종류를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = StageKind::byte_size();
        let mut data = &bytes[offset..offset + size];
        let stage_kind = StageKind::try_from_big_endian_bytes(data)?;

        // 서버의 초기 시대를 가져옵니다.
        offset = offset + size;
        size = Epoch::byte_size();
        data = &bytes[offset..offset + size];
        let epoch = Epoch::from_big_endian_bytes(data);

        // 오브젝트 식별자를 가져옵니다.
        offset = offset + size;
        size = ObjectId::byte_size();
        data = &bytes[offset..offset + size];
        let object_id = ObjectId::from_big_endian_bytes(data);

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

        Some(Self {
            stage_kind,
            epoch,
            object_id,
            num_players,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = InitStagePacket::new(
            StageKind::Downtown,
            Epoch::new(0),
            ObjectId::new(123456),
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
        );
        let raw_packet = origin.as_raw();
        let other = InitStagePacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
