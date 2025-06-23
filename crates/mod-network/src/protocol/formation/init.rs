//! 클라이언트가 캐릭터 편성 장면에 있을 때 참여한 플레이어 데이터 초기화 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, FormationPlayerInitData, StageKind, MAX_IN_GAME_PLAYERS},
    protocol::{Packet, PacketType, RawPacket},
};

/// 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함되어있습니다.
/// - stage_kind       | 4bit | 스테이지 종류
/// - allow_duplicates | 1bit | 캐릭터 중복 허용 여부
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const STAGE_BIT_MASK: u8 = 0x0F;
    const STAGE_SHIFT: usize = 0;
    const DUPLICATE_BIT_MASK: u8 = 0x01;
    const DUPLICATE_SHIFT: usize = 4;

    /// 새로운 비트 필드 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 스테이지 종류를 반환합니다.
    pub fn stage_kind(&self) -> StageKind {
        let val = ((self.0 >> Self::STAGE_SHIFT) & Self::STAGE_BIT_MASK) as u8;
        StageKind::new(val).unwrap_or_default()
    }

    /// 스테이지 종류를 설정합니다.
    pub const fn with_stage_kind(mut self, stage_kind: StageKind) -> Self {
        self.0 &= !(Self::STAGE_BIT_MASK << Self::STAGE_SHIFT);
        self.0 |= ((stage_kind as u8) & Self::STAGE_BIT_MASK) << Self::STAGE_SHIFT;
        self
    }

    /// 캐릭터 중복 여부를 반환합니다.
    pub fn allow_duplicates(&self) -> bool {
        (self.0 >> Self::DUPLICATE_SHIFT) & Self::DUPLICATE_BIT_MASK == Self::DUPLICATE_BIT_MASK
    }

    /// 캐릭터 중복 여부를 설정합니다.
    pub const fn with_allow_duplicates(mut self, duplicates: bool) -> Self {
        self.0 &= !(Self::DUPLICATE_BIT_MASK << Self::DUPLICATE_SHIFT);
        self.0 |= ((duplicates as u8) & Self::DUPLICATE_BIT_MASK) << Self::DUPLICATE_SHIFT;
        self
    }
}

impl Default for Bitfield {
    fn default() -> Self {
        Self(0x00)
    }
}

impl BigEndian for Bitfield {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u8::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

/// 서버에서 클라이언트로 보내는 캐릭터 편성 장면 초기화 패킷입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct FormationDataInitPacket {
    // 총 남은 시간입니다.
    pub remaining_time_sec: f32,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,
    /// 플레이어 초기화 데이터
    pub players: Vec<FormationPlayerInitData>,
}

impl FormationDataInitPacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        remaining_time_sec: f32,
        stage_kind: StageKind,
        duplicates: bool,
        players: Vec<FormationPlayerInitData>,
    ) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");

        Self {
            remaining_time_sec,
            players,
            bitfield: Bitfield::new()
                .with_stage_kind(stage_kind)
                .with_allow_duplicates(duplicates),
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(
        remaining_time_sec: f32,
        stage_kind: StageKind,
        duplicates: bool,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = FormationPlayerInitData>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(
            remaining_time_sec,
            stage_kind,
            duplicates,
            iter.into_iter().collect(),
        )
    }

    /// 캐릭터 중복 여부를 반환합니다.
    pub fn allow_duplicates(&self) -> bool {
        self.bitfield.allow_duplicates()
    }

    /// 스테이지 종류를 반환합니다.
    pub fn stage_kind(&self) -> StageKind {
        self.bitfield.stage_kind()
    }
}

impl Packet for FormationDataInitPacket {
    fn packet_type() -> PacketType {
        PacketType::FormationDataInit
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let data_size = f32::byte_size()
            + u8::byte_size()
            + u8::byte_size()
            + FormationPlayerInitData::byte_size() * num_players;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.remaining_time_sec.to_big_endian_bytes());
        data.extend_from_slice(&self.bitfield.to_big_endian_bytes());
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
                stringify!(FormationDataInitPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
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

        // 게임 월드 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = f32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let remaining_time_sec = f32::from_big_endian_bytes(data);

        // 비트 필드를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 커스텀 게임 플레이어 정보를 가져옵니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for _ in 0..num_players {
            offset = offset + size;
            size = FormationPlayerInitData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(FormationPlayerInitData::from_big_endian_bytes(data));
        }

        Some(Self {
            remaining_time_sec,
            bitfield,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{GameTier, ProfileIcon, Team, UserId, UserName};

    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_formation_data_init_packet() {
        FormationDataInitPacket::new(60.0, StageKind::City, true, vec![]);
    }

    #[test]
    fn test_formation_data_init_packet() {
        let player_0 = FormationPlayerInitData::new(
            UserId::new(13415),
            UserName::from_str("로봇청소기"),
            ProfileIcon::CharacterAris,
            GameTier::Gold,
            Team::Blue,
            0,
        );
        let player_1 = FormationPlayerInitData::new(
            UserId::new(6423651),
            UserName::from_str("모모이"),
            ProfileIcon::CharacterMomoi,
            GameTier::Bronze,
            Team::Blue,
            1,
        );
        let player_2 = FormationPlayerInitData::new(
            UserId::new(845141),
            UserName::from_str("유즈유즈"),
            ProfileIcon::CharacterYuzu,
            GameTier::Platinum,
            Team::Red,
            0,
        );
        let player_3 = FormationPlayerInitData::new(
            UserId::new(213415),
            UserName::from_str("미도리"),
            ProfileIcon::CharacterMidori,
            GameTier::Silver,
            Team::Blue,
            2,
        );

        let players = vec![player_0, player_1, player_2, player_3];
        let origin = FormationDataInitPacket::new(50.0, StageKind::City, true, players);
        let raw = origin.as_raw();
        let other = FormationDataInitPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
