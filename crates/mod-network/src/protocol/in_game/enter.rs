use crate::{
    components::{BigEndian, PlayPhasePlayer, StageKind, TryFromBigEndian, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 스테이지 로드 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InitStagePacket {
    /// 여러 자료형의 데이터가 포함된 비트 필드입니다.  
    /// 아래 자료형의 데이터가 포함되어있습니다.
    /// - StageKind (4bit): 스테이지 종류를 나타냅니다.
    pub bitfield: u8,
    /// 게임 월드의 플레이어 데이터
    pub players: Vec<PlayPhasePlayer>,
}

impl InitStagePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과할 경우 `panic!`을 호출합니다.
    ///
    pub fn new(stage_kind: StageKind, players: Vec<PlayPhasePlayer>) -> Self {
        assert!(
            0 < players.len() && players.len() <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );

        let stage_kind_bitfield = (stage_kind as u8) << 0;
        let bitfield = stage_kind_bitfield;

        Self { bitfield, players }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과할 경우 `panic!`을 호출합니다.
    ///
    pub fn from_iter<I>(stage_kind: StageKind, iter: I) -> Self
    where
        I: IntoIterator<Item = PlayPhasePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(stage_kind, iter.into_iter().collect())
    }

    /// 스테이지 종류를 설정합니다.
    pub fn with_stage_kind(&mut self, stage_kind: StageKind) -> &mut Self {
        self.bitfield = (self.bitfield & !(0xF << 0)) | (stage_kind as u8) << 0;
        self
    }

    /// 스테이지 종류를 가져옵니다.
    pub fn stage_kind(&self) -> StageKind {
        let val = (self.bitfield >> 0) & 0xF;
        StageKind::new(val).unwrap_or_default()
    }
}

impl Packet for InitStagePacket {
    fn packet_type() -> PacketType {
        PacketType::InitStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size =
            u8::byte_size() + u8::byte_size() + PlayPhasePlayer::byte_size() * self.players.len();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.bitfield.to_big_endian_bytes());
        data.extend_from_slice(&(self.players.len() as u8).to_big_endian_bytes());
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

        // 비트 필드 데이터를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let bitfield = u8::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let mut num_players = u8::from_big_endian_bytes(data);

        // 플레이어 데이터를 가져옵니다.
        let mut players = Vec::with_capacity(num_players as usize);
        while num_players > 0 {
            offset = offset + size;
            size = PlayPhasePlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(PlayPhasePlayer::try_from_big_endian_bytes(data)?);
            num_players -= 1;
        }

        Some(Self { bitfield, players })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{
        ActionState, ActionStateTimer, CharacterKind, ExSkillCost, HealthPoint, LatLon,
        MovementState, MovementStateTimer, RemainingBullet, Team, UserAccount, UserId, UserName,
        ViewState, ViewStateTimer,
    };

    use super::*;

    #[test]
    fn test_init_stage_packet() {
        let player_0 = PlayPhasePlayer::new(
            UserAccount::new(UserId::new(1412512), UserName::from_str("Aris")),
            1,
            2,
            3,
            CharacterKind::ArisOriginal,
            RemainingBullet::new(10, 30),
            HealthPoint::new(1413, 1414),
            [1.1512351, 2.4151616, 1.16561651],
            [1.5415151, 0.16551351, 0.9513515, 1.0515161],
            Team::Blue,
            1,
            ExSkillCost(55.31),
            ActionState::Aiming,
            ActionStateTimer(3.03151),
            MovementState::InPlaceLanding,
            MovementStateTimer(2.1515),
            ViewState::ZoomIn,
            ViewStateTimer(6.1412),
            LatLon {
                lat: 1.3151613,
                lon: 0.0154123,
            },
        );

        let origin = InitStagePacket::from_iter(StageKind::City, [player_0]);
        let raw = origin.as_raw();
        let other = InitStagePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
