use crate::{
    components::{
        BigEndian, InGameDraftPlayer, LoginToken, Team, TryFromBigEndian, UserId,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 동기화 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncDraftPacket {
    /// 남은 작업의 수
    pub num_remaining_task: u16,
    /// 클라이언트 사용자 식별자
    pub user_id: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl SyncDraftPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(num_remaining_task: u16, user_id: UserId, token: LoginToken) -> Self {
        Self {
            num_remaining_task,
            user_id,
            token,
        }
    }
}

impl Packet for SyncDraftPacket {
    fn packet_type() -> PacketType {
        PacketType::SyncDraft
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = u16::byte_size() + UserId::byte_size() + LoginToken::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.num_remaining_task.to_big_endian_bytes());
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(SyncDraftPacket)
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

        // 남은 작업의 수를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let num_remaining_task = u16::from_big_endian_bytes(data);

        // 사용자 식별자를 가져옵니다.
        offset = offset + size;
        size = UserId::byte_size();
        data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self {
            num_remaining_task,
            user_id,
            token,
        })
    }
}

/// 서버에서 클라이언트로 보내는 캐릭터 편성 진행 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessDraftPacket {
    /// 남은 캐릭터 편성 시간입니다.
    remaining_time: f32,
    /// 블루팀에 속한 플레이어 집합입니다.
    blue_team_players: Vec<InGameDraftPlayer>,
    /// 레드팀에 속한 플레이어 집합입니다.
    red_team_players: Vec<InGameDraftPlayer>,
}

impl ProcessDraftPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 플레이어 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn new(remaining_time: f32, players: Vec<InGameDraftPlayer>) -> Self {
        assert!(
            players.len() <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );

        // 플레이어가 속한 팀에 따라 나눕니다.
        let (blue_team_players, red_team_players) = players
            .into_iter()
            .partition(|player| player.team == Team::Blue);

        Self {
            remaining_time,
            blue_team_players,
            red_team_players,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 플레이어 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn from_iter<I>(remaining_time: f32, players: I) -> Self
    where
        I: IntoIterator<Item = InGameDraftPlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(remaining_time, players.into_iter().collect())
    }
}

impl Packet for ProcessDraftPacket {
    fn packet_type() -> PacketType {
        PacketType::ProcessDraft
    }

    /// 패킷을 RawPacket으로 변환합니다.
    ///
    /// # Panics
    /// `blue_team_players`와 `red_team_players`를 합친 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림 레이아웃
        // +-------------------+
        // | 남은 편성 시간       |
        // +-------------------+
        // | 참여 인원 수 (1byte) |
        // +-------------------+
        // | 사용자 정보          |
        // +-------------------+
        //
        let num_players = self.blue_team_players.len() + self.red_team_players.len();
        assert!(
            num_players <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );
        let data_size =
            f32::byte_size() + u8::byte_size() + num_players * InGameDraftPlayer::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.remaining_time.to_big_endian_bytes());
        data.extend_from_slice(&(num_players as u8).to_big_endian_bytes());
        for player in self
            .blue_team_players
            .iter()
            .chain(self.red_team_players.iter())
        {
            data.extend_from_slice(&player.to_big_endian_bytes());
        }

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(ProcessDraftPacket)
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

        // 남은 시간을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
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
            size = InGameDraftPlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(InGameDraftPlayer::try_from_big_endian_bytes(data)?);
        }

        Some(Self::new(remaining_time, players))
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{CharacterKind, DraftStatus, UserId, UserInfo, UserName};

    use super::*;

    #[test]
    fn test_process_draft_packet() {
        let player_0 = InGameDraftPlayer {
            info: UserInfo::new(UserId::new(1234), UserName::new("Aris")),
            character_kind: CharacterKind::ArisOriginal,
            team: Team::Blue,
            status: DraftStatus::Ready,
        };
        let player_1 = InGameDraftPlayer {
            info: UserInfo::new(UserId::new(5678), UserName::new("Momoi")),
            character_kind: CharacterKind::MomoiOriginal,
            team: Team::Red,
            status: DraftStatus::Wait,
        };
        let player_2 = InGameDraftPlayer {
            info: UserInfo::new(UserId::new(9012), UserName::new("Midori")),
            character_kind: CharacterKind::MidoriOriginal,
            team: Team::Blue,
            status: DraftStatus::Ready,
        };
        let players = vec![player_0, player_1, player_2];

        let origin = ProcessDraftPacket::new(3.123, players);
        let raw = origin.as_raw();
        let other = ProcessDraftPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
