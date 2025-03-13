use crate::components::{BigEndian, Epoch, InGamePlayer, StageKind, TryFromBigEndian};

use crate::protocol::{Packet, PacketType, RawPacket};

/// 클라이언트가 게임에 입장할 때
/// 서버에서 클라이언트로 전송되는 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InitStagePacket {
    /// 게임 월드의 초기 시대
    pub epoch: Epoch,
    /// 게임 스테이지 종류
    pub stage_kind: StageKind,
    /// 클라이언트를 포함한 게임 월드에 존재하는 플레이어의 수
    pub num_players: u16,
    /// 클라이언트를 포함한 게임 월드 플레이어 데이터
    pub players: Vec<InGamePlayer>,
}

impl InitStagePacket {
    pub fn new(epoch: Epoch, stage_kind: StageKind, players: Vec<InGamePlayer>) -> Self {
        Self {
            epoch,
            stage_kind,
            num_players: players.len() as u16,
            players,
        }
    }
}

impl Default for InitStagePacket {
    // object_id 기본 값은 NULL이어야 합니다.
    fn default() -> Self {
        Self {
            epoch: Epoch::new(0),
            stage_kind: StageKind::default(),
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
        let data_size = Epoch::byte_size()
            + StageKind::byte_size()
            + u16::byte_size()
            + InGamePlayer::byte_size() * self.num_players as usize;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.epoch.to_big_endian_bytes());
        data.extend_from_slice(&self.stage_kind.to_big_endian_bytes());
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

        // 서버의 초기 시대를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = Epoch::byte_size();
        let mut data = &bytes[offset..offset + size];
        let epoch = Epoch::from_big_endian_bytes(data);

        // 스테이지 종류를 가져옵니다.
        offset = offset + size;
        size = StageKind::byte_size();
        data = &bytes[offset..offset + size];
        let stage_kind = StageKind::try_from_big_endian_bytes(data)?;

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
            size = InGamePlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(InGamePlayer::try_from_big_endian_bytes(data)?);
            count -= 1;
        }

        Some(Self {
            stage_kind,
            epoch,
            num_players,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{UserId, UserInfo, UserName};

    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = InitStagePacket::new(
            Epoch::new(0),
            StageKind::City,
            vec![
                InGamePlayer {
                    info: UserInfo::new(UserId::new(123456), UserName::new("Foo")),
                    ..Default::default()
                },
                InGamePlayer {
                    info: UserInfo::new(UserId::new(654321), UserName::new("Bar")),
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
