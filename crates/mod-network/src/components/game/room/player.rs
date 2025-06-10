//! 게임 대기실 플레이어와 관련된 코드를 관리합니다.
//!

use crate::components::{
    BigEndian, CharacterKind, GameTier, Permission, Team, TryFromBigEndian, UserId, UserName,
};

/// 대기 상태에서 사용되는 비트 필드 데이터입니다.
///
/// 이래 데이터가 포함됩니다.
/// - permission            | 1bit | 권한
/// - team                  | 1bit | 팀의 종류
/// - is_ready_to_play      | 1bit | 준비 여부
/// - use_profile_character | 1bit | 프로필 캐릭터 설정 여부
/// - tier                  | 2bit | 게임 티어
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const PERMISSION_BIT_MASK: u8 = 0x01;
    const PERMISSION_SHIFT: usize = 0;
    const TEAM_BIT_MASK: u8 = 0x01;
    const TEAM_SHIFT: usize = 1;
    const READY_BIT_MASK: u8 = 0x01;
    const READY_SHIFT: usize = 2;
    const PROFILE_BIT_MASK: u8 = 0x01;
    const PROFILE_SHIFT: usize = 3;
    const TIER_BIT_MASK: u8 = 0x03;
    const TIER_SHIFT: usize = 4;

    /// 새로운 비트 필드 데이터를 생성합니다.
    const fn new() -> Self {
        Self(0x00)
    }

    /// 권한을 반환합니다.
    fn permission(&self) -> Permission {
        let val = (self.0 >> Self::PERMISSION_SHIFT) & Self::PERMISSION_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { Permission::new(val).unwrap_unchecked() }
    }

    /// 권한을 설정합니다.
    fn with_permission(mut self, permission: Permission) -> Self {
        self.0 &= !(Self::PERMISSION_BIT_MASK << Self::PERMISSION_SHIFT);
        self.0 |= ((permission as u8) & Self::PERMISSION_BIT_MASK) << Self::PERMISSION_SHIFT;
        self
    }

    /// 팀 종류를 반환합니다.
    fn team(&self) -> Team {
        let val = (self.0 >> Self::TEAM_SHIFT) & Self::TEAM_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { Team::new(val).unwrap_unchecked() }
    }

    /// 팀 종류를 설정합니다.
    fn with_team(mut self, team: Team) -> Self {
        self.0 &= !(Self::TEAM_BIT_MASK << Self::TEAM_SHIFT);
        self.0 |= ((team as u8) & Self::TEAM_BIT_MASK) << Self::TEAM_SHIFT;
        self
    }

    /// 준비 여부를 반환합니다.
    fn is_ready_to_play(&self) -> bool {
        (self.0 >> Self::READY_SHIFT) & Self::READY_BIT_MASK == Self::READY_BIT_MASK
    }

    /// 준비 여부를 설정합니다.
    fn with_ready_to_play(mut self, ready: bool) -> Self {
        self.0 &= !(Self::READY_BIT_MASK << Self::READY_SHIFT);
        self.0 |= ((ready as u8) & Self::READY_BIT_MASK) << Self::READY_SHIFT;
        self
    }

    /// 프로필 캐릭터 설정 여부를 반환합니다.
    fn use_profile_character(&self) -> bool {
        (self.0 >> Self::PROFILE_SHIFT) & Self::PROFILE_BIT_MASK == Self::PROFILE_BIT_MASK
    }

    /// 프로필 캐릭터 설정 여부를 설정합니다.
    fn with_use_profile_character(mut self, use_profile_character: bool) -> Self {
        self.0 &= !(Self::PROFILE_BIT_MASK << Self::PROFILE_SHIFT);
        self.0 |= ((use_profile_character as u8) & Self::PROFILE_BIT_MASK) << Self::PROFILE_SHIFT;
        self
    }

    /// 게임 티어를 반환합니다.
    fn tier(&self) -> GameTier {
        let val = (self.0 >> Self::TIER_SHIFT) & Self::TIER_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { GameTier::new(val).unwrap_unchecked() }
    }

    /// 게임 티어를 설정합니다.
    fn with_tier(mut self, tier: GameTier) -> Self {
        self.0 &= !(Self::TIER_BIT_MASK << Self::TIER_SHIFT);
        self.0 |= ((tier as u8) & Self::TIER_BIT_MASK) << Self::TIER_SHIFT;
        self
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

impl Default for Bitfield {
    fn default() -> Self {
        Self(0x00)
    }
}

/// 대기 상태일 떄 플레이어의 정보를 갱신하기 위한 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomRoomPlayerData {
    /// 사용자 식별자입니다.
    pub uid: UserId,
    /// 사용자 이름입니다.
    pub name: UserName,
    /// 사용자 대표 캐릭터 종류
    character_kind: CharacterKind,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,
}

impl RoomRoomPlayerData {
    /// 새로운 대기 상태 플레이어 데이터를 생성합니다.
    pub const fn new(uid: UserId, name: UserName) -> Self {
        Self {
            uid,
            name,
            character_kind: CharacterKind::ArisOriginal,
            bitfield: Bitfield::new(),
        }
    }

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }

    /// 권한을 설정합니다.
    pub fn set_permission(&mut self, permission: Permission) {
        self.bitfield = self.bitfield.with_permission(permission);
    }

    /// 권한을 설정합니다.
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.bitfield = self.bitfield.with_permission(permission);
        self
    }

    /// 팀 종류를 반환합니다.
    pub fn team(&self) -> Team {
        self.bitfield.team()
    }

    /// 팀 종류를 설정합니다.
    pub fn set_team(&mut self, team: Team) {
        self.bitfield = self.bitfield.with_team(team);
    }

    /// 팀 종류를 설정합니다.
    pub fn with_team(mut self, team: Team) -> Self {
        self.bitfield = self.bitfield.with_team(team);
        self
    }

    /// 준비 여부를 반환합니다.
    pub fn is_ready_to_play(&self) -> bool {
        self.bitfield.is_ready_to_play()
    }

    /// 준비 여부를 설정합니다.
    pub fn set_ready_to_play(&mut self, ready: bool) {
        self.bitfield = self.bitfield.with_ready_to_play(ready);
    }

    /// 준비 여부를 설정합니다.
    pub fn with_ready_to_play(mut self, ready: bool) -> Self {
        self.bitfield = self.bitfield.with_ready_to_play(ready);
        self
    }

    /// 프로필 캐릭터 종류를 반환합니다.
    pub fn character_kind(&self) -> Option<CharacterKind> {
        self.bitfield
            .use_profile_character()
            .then_some(self.character_kind)
    }

    /// 프로필 캐릭터 종류를 설정합니다.
    pub fn set_character_kind(&mut self, character_kind: CharacterKind) {
        self.bitfield = self.bitfield.with_use_profile_character(true);
        self.character_kind = character_kind;
    }

    /// 프로필 캐릭터 종류를 설정합니다.
    pub fn with_character_kind(mut self, character_kind: CharacterKind) -> Self {
        self.bitfield = self.bitfield.with_use_profile_character(true);
        self.character_kind = character_kind;
        self
    }

    /// 게임 티어를 반환합니다.
    pub fn tier(&self) -> GameTier {
        self.bitfield.tier()
    }

    /// 게임 티어를 설정합니다.
    pub fn set_tier(&mut self, tier: GameTier) {
        self.bitfield = self.bitfield.with_tier(tier);
    }

    /// 게임 티어를 설정합니다.
    pub fn with_tier(mut self, tier: GameTier) -> Self {
        self.bitfield = self.bitfield.with_tier(tier);
        self
    }
}

impl BigEndian for RoomRoomPlayerData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + UserName::byte_size()
            + CharacterKind::byte_size()
            + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.name.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 생성된 바이트가 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RoomStatePlayerData),
            )
        };

        bytes
    }
}

impl TryFromBigEndian for RoomRoomPlayerData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 주어진 바이트가 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RoomStatePlayerData),
            )
        };

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 사용자 이름을 가져옵니다.
        offset = offset + size;
        size = UserName::byte_size();
        data = &bytes[offset..offset + size];
        let name = UserName::from_big_endian_bytes(data);

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Some(Self {
            uid,
            name,
            character_kind,
            bitfield,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_permission() {
        let val = Permission::User;
        let bitfield = Bitfield::new().with_permission(val);
        assert_eq!(Permission::User, bitfield.permission());

        let val = Permission::Admin;
        let bitfield = Bitfield::new().with_permission(val);
        assert_eq!(Permission::Admin, bitfield.permission());
    }

    #[test]
    fn test_bitfield_team() {
        let val = Team::Blue;
        let bitfield = Bitfield::new().with_team(val);
        assert_eq!(Team::Blue, bitfield.team());

        let val = Team::Red;
        let bitfield = Bitfield::new().with_team(val);
        assert_eq!(Team::Red, bitfield.team());
    }

    #[test]
    fn test_bitfield_ready_to_play() {
        let bitfield = Bitfield::new().with_ready_to_play(false);
        assert_eq!(false, bitfield.is_ready_to_play());

        let bitfield = Bitfield::new().with_ready_to_play(true);
        assert_eq!(true, bitfield.is_ready_to_play());
    }

    #[test]
    fn test_bitfield_use_profile_character() {
        let bitfield = Bitfield::new().with_use_profile_character(false);
        assert_eq!(false, bitfield.use_profile_character());

        let bitfield = Bitfield::new().with_use_profile_character(true);
        assert_eq!(true, bitfield.use_profile_character());
    }

    #[test]
    fn test_bitfield_tier() {
        let val = GameTier::Bronze;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Bronze, bitfield.tier());

        let val = GameTier::Silver;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Silver, bitfield.tier());

        let val = GameTier::Gold;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Gold, bitfield.tier());

        let val = GameTier::Platinum;
        let bitfield = Bitfield::new().with_tier(val);
        assert_eq!(GameTier::Platinum, bitfield.tier());
    }

    #[test]
    fn test_room_state_player_data() {
        let origin =
            RoomRoomPlayerData::new(UserId::new(12345), UserName::from_str("Aris Original"))
                .with_permission(Permission::Admin)
                .with_team(Team::Red)
                .with_character_kind(CharacterKind::MomoiOriginal)
                .with_tier(GameTier::Silver)
                .with_ready_to_play(true);
        let bytes = origin.to_big_endian_bytes();
        let other = RoomRoomPlayerData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 비교
        assert_eq!(origin, other);
    }
}
