//! 인게임 결과를 전송하는 패킷과 관련된 코드를 관리합니다.
//!
//
//    +--------+                  +--------+
//    | client |                  | server |
//    +--------+                  +--------+
//         |                           |
//         |<-------[FinishStage]------|
//         |                           |
//   +------------+                    |
//   |   switch   |                    |
//   | game scene |                    |
//   +------------+                    |
//         |                           |
//    if <done>-----[Response]-------->|
//         |                           |
//         |                      +----------+
//         |                      |  switch  |
//         |                      |  state   |
//         |                      +----------+
//         |                           |
//         |<------[Custom/Lobby]------|
//         |                           |
//        ...                         ...
//

use crate::{
    components::{
        BigEndian, FinishPhasePlayer, LoginToken, StageKind, Team, TryFromBigEndian, UserId,
        VictoryType, MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 게임 결과 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishStagePacket {
    /// 여러 자료형의 데이터가 포함된 비트 필드입니다.  
    /// 아래 자료형의 데이터가 포함되어 있습니다.
    /// - Team (2bit): 우승 팀 정보
    /// - VictoryType (2bit): 승리 종류
    /// - StageKind (4bit): 스테이지 종류
    ///
    pub bitfield: u8,
    /// 모든 플레이어의 플레이 데이터입니다.
    pub players: Vec<FinishPhasePlayer>,
}

impl FinishStagePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과할 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        winner: Option<Team>,
        victory_type: VictoryType,
        stage_kind: StageKind,
        players: Vec<FinishPhasePlayer>,
    ) -> Self {
        assert!(
            0 < players.len() && players.len() <= MAX_IN_GAME_PLAYERS,
            "there are more people participaing in the game than the capacity!"
        );

        let winner_team_bit = ((winner.map(|t| t as u8).unwrap_or(2)) & 0x3) << 0;
        let victory_type_bit = ((victory_type as u8) & 0x3) << 2;
        let stage_kind_bit = ((stage_kind as u8) & 0xF) << 4;
        let bitfield = winner_team_bit | victory_type_bit | stage_kind_bit;

        Self { bitfield, players }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`가 `MAX_IN_GAME_PLAYER`를 초과할 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(
        winner: Option<Team>,
        victory_type: VictoryType,
        stage_kind: StageKind,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = FinishPhasePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(winner, victory_type, stage_kind, iter.into_iter().collect())
    }

    /// 우승 팀을 설정합니다.
    pub fn with_winner_team(&mut self, winner: Option<Team>) -> &mut Self {
        self.bitfield =
            (self.bitfield & !(0x3 << 0)) | ((winner.map(|t| t as u8).unwrap_or(2)) & 0x3) << 0;
        self
    }

    /// 우승 팀을 반환합니다.
    pub fn winner_team(&self) -> Option<Team> {
        // Safe: 전달되는 정수는 범위를 넘지 않음
        let val = (self.bitfield >> 0) & 0x3;
        if val < 2 {
            Team::new(val)
        } else {
            None
        }
    }

    /// 승리 종류를 설정합니다.
    pub fn with_victory_type(&mut self, victory_type: VictoryType) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x3 << 2)) | ((victory_type as u8) & 0x3) << 2;
        self
    }

    /// 승리 종류를 반환합니다.
    pub fn victory_type(&self) -> VictoryType {
        // Safe: 전달되는 정수는 범위를 넘지 않음
        let val = (self.bitfield >> 2) & 0x3;
        unsafe { VictoryType::new(val).unwrap_unchecked() }
    }

    /// 스테이지 종류를 설정합니다.
    pub fn with_stage_kind(&mut self, stage_kind: StageKind) -> &mut Self {
        self.bitfield = (self.bitfield & !(0xF << 4)) | ((stage_kind as u8) & 0x4) << 4;
        self
    }

    /// 스테이지 종류를 반환합니다.
    pub fn stage_kind(&self) -> StageKind {
        let val = (self.bitfield >> 4) & 0xF;
        StageKind::new(val).unwrap_or_default()
    }
}

impl Packet for FinishStagePacket {
    fn packet_type() -> PacketType {
        PacketType::FinishStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = u8::byte_size() 
            + u8::byte_size() 
            + FinishPhasePlayer::byte_size() * self.players.len();

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
                stringify!(FinishStagePacket)
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
            size = FinishPhasePlayer::byte_size();
            data = &bytes[offset..offset + size];
            players.push(FinishPhasePlayer::try_from_big_endian_bytes(data)?);
            num_players -= 1;
        }

        Some(Self { bitfield, players })
    }
}

/// 클라이언트에서 서버로 전송하는 게임 결과 응답 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishStageResponsePacket {
    /// 사용자 식별자
    pub user_id: UserId,
    /// 사용자 로그인 토큰
    pub token: LoginToken,
}

impl FinishStageResponsePacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(user_id: UserId, token: LoginToken) -> Self {
        Self { user_id, token }
    }
}

impl Packet for FinishStageResponsePacket {
    fn packet_type() -> PacketType {
        PacketType::FinishStageResponse
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FinishStageResponsePacket)
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

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 사용자 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { user_id, token })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{CharacterKind, UserAccount, UserId, UserName};

    use super::*;

    #[test]
    fn test_finish_stage_packet() {
        let player_0 = FinishPhasePlayer::new(
            UserAccount::new(UserId::new(12345), UserName::from_str("유즈유즈")),
            CharacterKind::MomoiOriginal,
            200,
            0,
            1124,
            89,
            110,
            Team::Red,
            2,
            false,
        );

        let origin = FinishStagePacket::from_iter(
            Some(Team::Red),
            VictoryType::JudgmentWin,
            StageKind::City,
            [player_0],
        );
        let raw = origin.as_raw();
        let other = FinishStagePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_finish_stage_response_packet() {
        let origin =
            FinishStageResponsePacket::new(UserId::new(1234566), LoginToken::new(8921543125));
        let raw = origin.as_raw();
        let other = FinishStageResponsePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
