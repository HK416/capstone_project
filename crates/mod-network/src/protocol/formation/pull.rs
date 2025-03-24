use crate::{
    components::{BigEndian, FormationPhasePlayer, Team, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 캐릭터 편성 진행 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct FormationPullPacket {
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
    pub fn new(remaining_time: f32, players: Vec<FormationPhasePlayer>) -> Self {
        assert!(
            players.len() <= MAX_IN_GAME_PLAYERS,
            "There are more people participaing in the game than the capacity!"
        );

        Self {
            remaining_time,
            players,
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 플레이어 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 `panic!`을 호출합니다.
    ///
    pub fn from_iter<I>(remaining_time: f32, players: I) -> Self
    where
        I: IntoIterator<Item = FormationPhasePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(remaining_time, players.into_iter().collect())
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
        let data_size =
            f32::byte_size() + u8::byte_size() + num_players * FormationPhasePlayer::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
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
            size = FormationPhasePlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(FormationPhasePlayer::from_big_endian_bytes(data));
        }

        Some(Self::new(remaining_time, players))
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{CharacterKind, UserAccount, UserId, UserName};

    use super::*;

    #[test]
    fn test_formation_pull_packet() {
        let player_0 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(123145234), UserName::from_str("Aris")),
            Some(CharacterKind::ArisOriginal),
            true,
            Team::Blue,
        );
        let player_1 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(123324134), UserName::from_str("Yuzu")),
            None,
            false,
            Team::Blue,
        );
        let player_2 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(6531234), UserName::from_str("Momoi")),
            Some(CharacterKind::MomoiOriginal),
            true,
            Team::Red,
        );
        let player_3 = FormationPhasePlayer::new(
            UserAccount::new(UserId::new(61234534), UserName::from_str("Midori")),
            Some(CharacterKind::MidoriOriginal),
            true,
            Team::Red,
        );
        let players = vec![player_0, player_1, player_2, player_3];

        let origin = FormationPullPacket::new(3.123, players);
        let raw = origin.as_raw();
        let other = FormationPullPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
