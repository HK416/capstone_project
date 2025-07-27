use crate::{
    components::{
        BigEndian, CustomRoomPlayerData, StageKind, TryFromBigEndian, WorldId, MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함되어있습니다.
/// - stage_king       | 4bit | 스테이지 종류
/// - allow_duplicates | 1bit | 캐릭터 중복 허용 여부
/// - unbalanced       | 1bit | 팀 균형 여부
/// - fill_empty_slot  | 1bit | AI 플레이어 사용 여부
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const STAGE_BIT_MASK: u8 = 0x0F;
    const STAGE_SHIFT: usize = 0;
    const DUPLICATE_BIT_MASK: u8 = 0x01;
    const DUPLICATE_SHIFT: usize = 4;
    const UNBALANCE_BIT_MASK: u8 = 0x01;
    const UNBALANCE_SHIFT: usize = 5;
    const FILL_BIT_MASK: u8 = 0x01;
    const FILL_SHIFT: usize = 6;

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

    /// 팀 불균형 허용 여부를 반환합니다.
    pub fn allow_unbalanced(&self) -> bool {
        (self.0 >> Self::UNBALANCE_SHIFT) & Self::UNBALANCE_BIT_MASK == Self::UNBALANCE_BIT_MASK
    }

    /// 팀 불균형 여부를 설정합니다.
    pub const fn with_allow_unbalanced(mut self, unbalanced: bool) -> Self {
        self.0 &= !(Self::UNBALANCE_BIT_MASK << Self::UNBALANCE_SHIFT);
        self.0 |= ((unbalanced as u8) & Self::UNBALANCE_BIT_MASK) << Self::UNBALANCE_SHIFT;
        self
    }

    /// AI 플레이어로 채울 것인지 여부를 반환합니다.
    pub fn fill_empty_slot(&self) -> bool {
        (self.0 >> Self::FILL_SHIFT) & Self::FILL_BIT_MASK == Self::FILL_BIT_MASK
    }

    /// AI 플레이어로 채울 것인지 여부를 설정합니다.
    pub const fn with_fill_empty_slot(mut self, fill: bool) -> Self {
        self.0 &= !(Self::FILL_BIT_MASK << Self::FILL_SHIFT);
        self.0 |= ((fill as u8) & Self::FILL_BIT_MASK) << Self::FILL_SHIFT;
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

/// 서버에서 클라이언트로 보내는 커스텀 게임 갱신 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDataUpdatePacket {
    /// 게임 월드 식별자입니다.
    pub id: WorldId,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,
    /// 참여한 플레이어 목록입니다.
    pub players: Vec<CustomRoomPlayerData>,
}

impl RoomDataUpdatePacket {
    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(
        id: WorldId,
        stage_kind: StageKind,
        duplicates: bool,
        unbalanced: bool,
        fill_ai: bool,
        players: Vec<CustomRoomPlayerData>,
    ) -> Self {
        assert!(!players.is_empty(), "the given data is empty!");
        assert!(players.len() <= MAX_IN_GAME_PLAYERS, "too many players!");

        Self {
            id,
            players,
            bitfield: Bitfield::new()
                .with_stage_kind(stage_kind)
                .with_allow_duplicates(duplicates)
                .with_allow_unbalanced(unbalanced)
                .with_fill_empty_slot(fill_ai),
        }
    }

    /// 새로운 패킷을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `players`의 요소 수가 `MAX_IN_GAME_PLAYERS`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_iter<I>(
        id: WorldId,
        stage_kind: StageKind,
        duplicates: bool,
        unbalanced: bool,
        fill_ai: bool,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = CustomRoomPlayerData>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::new(
            id,
            stage_kind,
            duplicates,
            unbalanced,
            fill_ai,
            iter.into_iter().collect(),
        )
    }

    /// 캐릭터 중복 여부를 반환합니다.
    pub fn allow_duplicates(&self) -> bool {
        self.bitfield.allow_duplicates()
    }

    /// 팀 불균형 허용 여부를 반환합니다.
    pub fn allow_unbalanced(&self) -> bool {
        self.bitfield.allow_unbalanced()
    }

    /// AI 허용 여부를 반환합니다.
    pub fn allow_using_ai(&self) -> bool {
        self.bitfield.fill_empty_slot()
    }

    /// 스테이지 종류를 반환합니다.
    pub fn stage_kind(&self) -> StageKind {
        self.bitfield.stage_kind()
    }
}

impl Packet for RoomDataUpdatePacket {
    fn packet_type() -> PacketType {
        PacketType::RoomDataUpdate
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let num_players = self.players.len();
        let data_size = WorldId::byte_size()
            + u8::byte_size()
            + u8::byte_size()
            + CustomRoomPlayerData::byte_size() * num_players;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.id.to_big_endian_bytes());
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
                stringify!(RoomDataUpdatePacket)
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
        let mut size = WorldId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let id = WorldId::from_big_endian_bytes(data);

        // 비트 필드를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        // 플레이어 수를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let mut num_players = u8::from_big_endian_bytes(data) as usize;
        if num_players <= 0 || num_players > MAX_IN_GAME_PLAYERS {
            return None;
        }

        // 커스텀 게임 플레이어 정보를 가져옵니다.
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        while num_players > 0 {
            offset = offset + size;
            size = CustomRoomPlayerData::byte_size();
            data = &bytes[offset..offset + size];
            players.push(CustomRoomPlayerData::try_from_big_endian_bytes(data)?);
            num_players -= 1;
        }

        Some(Self {
            id,
            bitfield,
            players,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{
        CustomRoomPlayerData, GameTier, Permission, ProfileIcon, Team, UserId, UserName,
    };

    use super::*;

    #[test]
    fn test_room_data_update_packet() {
        let player_0 = CustomRoomPlayerData::new(
            UserId::new(12341),
            UserName::from_str("아리수"),
            ProfileIcon::CharacterAris,
            12,
            Permission::Admin,
            Team::Blue,
            GameTier::Silver,
            false,
        );
        let player_1 = CustomRoomPlayerData::new(
            UserId::new(21321),
            UserName::from_str("유즈퀸"),
            ProfileIcon::CharacterYuzu,
            23,
            Permission::User,
            Team::Red,
            GameTier::Platinum,
            true,
        );
        let player_2 = CustomRoomPlayerData::new(
            UserId::new(34121),
            UserName::from_str("데스모모이"),
            ProfileIcon::CharacterMomoi,
            34,
            Permission::User,
            Team::Blue,
            GameTier::Gold,
            false,
        );
        let player_3 = CustomRoomPlayerData::new(
            UserId::new(14211),
            UserName::from_str("미도리"),
            ProfileIcon::CharacterMidori,
            45,
            Permission::User,
            Team::Red,
            GameTier::Bronze,
            true,
        );
        let players = vec![player_0, player_1, player_2, player_3];

        let origin = RoomDataUpdatePacket::new(
            WorldId::new(12312451),
            StageKind::City,
            true,
            false,
            true,
            players,
        );
        let raw = origin.as_raw();
        let other = RoomDataUpdatePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
