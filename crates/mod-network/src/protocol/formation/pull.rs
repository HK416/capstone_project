use crate::{
    components::{BigEndian, FormationPhasePlayer, StageKind, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 캐릭터 편성 진행 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct FormationPullPacket {
    /// 여러 자료형의 데이터가 포함된 비트 필드입니다.  
    /// 아래 데이터가 포함되어있습니다.
    /// - bool (1bit): 캐릭터 중복 여부를 나타냅니다.
    /// - StageKind (4bit): 지형의 종류를 나타냅니다.
    pub bitfield: u8,
    /// 남은 캐릭터 편성 시간입니다.
    pub remaining_time: f32,
    /// 플레이어 집합입니다.
    pub players: Vec<FormationPhasePlayer>,
}

impl FormationPullPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 플레이어 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn new(
        allow_duplicates: bool,
        stage_kind: StageKind,
        remaining_time: f32,
        players: Vec<FormationPhasePlayer>,
    ) -> Self {
        assert!(
            players.len() <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );

        let duplicates_bitfield = (allow_duplicates as u8) << 4;
        let stage_kind_bitfield = (stage_kind as u8) << 0;
        let bitfield = duplicates_bitfield | stage_kind_bitfield;

        Self {
            bitfield,
            remaining_time,
            players,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 플레이어 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn from_iter<I>(
        allow_duplicates: bool,
        stage_kind: StageKind,
        remaining_time: f32,
        players: I,
    ) -> Self
    where
        I: IntoIterator<Item = FormationPhasePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(
            allow_duplicates,
            stage_kind,
            remaining_time,
            players.into_iter().collect(),
        )
    }

    /// 캐릭터 중복 여부를 설정합니다.
    pub fn with_allow_duplicates(&mut self, allow_duplicates: bool) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 4)) | (allow_duplicates as u8) << 4;
        self
    }

    /// 캐릭터 중복 여부를 가져옵니다.
    pub fn allow_duplicates(&self) -> bool {
        (self.bitfield >> 4) & 0x1 == 0x1
    }

    /// 스테이지 종류를 설정합니다.
    pub fn with_stage_kind(&mut self, stage_kind: StageKind) -> &mut Self {
        self.bitfield = (self.bitfield & (0xF << 0)) | (stage_kind as u8) << 0;
        self
    }

    /// 스테이지 종류를 가져옵니다.
    pub fn stage_kind(&self) -> StageKind {
        let val = (self.bitfield >> 0) & 0xF;
        StageKind::new(val).unwrap_or_default()
    }
}

impl Packet for FormationPullPacket {
    fn packet_type() -> PacketType {
        PacketType::FormationPull
    }

    /// 패킷을 RawPacket으로 변환합니다.
    ///
    /// # Panics
    /// `blue_team_players`와 `red_team_players`를 합친 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림 레이아웃
        // +-------------------+
        // | 비트 필드           |
        // +-------------------+
        // | 남은 편성 시간       |
        // +-------------------+
        // | 참여 인원 수 (1byte) |
        // +-------------------+
        // | 사용자 정보          |
        // +-------------------+
        //
        let num_players = self.players.len();
        assert!(
            num_players <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );
        let data_size = u8::byte_size()
            + f32::byte_size()
            + u8::byte_size()
            + num_players * FormationPhasePlayer::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.bitfield.to_big_endian_bytes());
        data.extend_from_slice(&self.remaining_time.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self.players.iter() {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationPullPacket)
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
                Self::packet_type(),
            );
            return None;
        }

        // 비트 필드를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let bitfield = u8::from_big_endian_bytes(data);

        // 남은 시간을 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let remaining_time = f32::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 플레이어 정보를 가져옵니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for _ in 0..num_players {
            offset = offset + size;
            size = FormationPhasePlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(FormationPhasePlayer::from_big_endian_bytes(data));
        }

        Some(Self {
            bitfield,
            remaining_time,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{CharacterKind, Team, UserAccount, UserId, UserName};

    use super::*;

    #[test]
    fn test_formation_pull_packet() {
        let player_0 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(123145234), UserName::from_str("Aris")),
            CharacterKind::ArisOriginal,
            true,
            Team::Blue,
        );
        let player_1 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(123324134), UserName::from_str("Yuzu")),
            CharacterKind::default(),
            false,
            Team::Blue,
        );
        let player_2 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(6531234), UserName::from_str("Momoi")),
            CharacterKind::MomoiOriginal,
            true,
            Team::Red,
        );
        let player_3 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(61234534), UserName::from_str("Midori")),
            CharacterKind::MidoriOriginal,
            true,
            Team::Red,
        );
        let players = vec![player_0, player_1, player_2, player_3];

        let origin = FormationPullPacket::new(true, StageKind::City, 3.123, players);
        let raw = origin.as_raw();
        let other = FormationPullPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
