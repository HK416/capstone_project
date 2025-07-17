//! 플레이어의 게임 결과 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::{
    BigEndian, CharacterKind, GameTier, ProfileIcon, Team, TryFromBigEndian, UserId, UserName,
};

/// 결과 상태에서 사용되는 비트 필드 데이터입니다.
///
/// 이래 데이터가 포함됩니다.
/// - team                  | 1bit | 팀의 종류
/// - team_index            | 3bit | 팀 내의 인덱스
/// - tier                  | 2bit | 게임 티어
/// - connected             | 1bit | 서버 접속 여부
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const TEAM_BIT_MASK: u8 = 0x01;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u8 = 0x07;
    const INDEX_SHIFT: usize = 1;
    const TIER_BIT_MASK: u8 = 0x03;
    const TIER_SHIFT: usize = 3;
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 6;

    /// 새로운 비트 필드 데이터를 생성합니다.
    const fn new() -> Self {
        Self(0x00)
    }

    /// 팀 종류를 반환합니다.
    fn team(&self) -> Team {
        let val = (self.0 >> Self::TEAM_SHIFT) & Self::TEAM_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { Team::new(val).unwrap_unchecked() }
    }

    /// 팀 종류를 설정합니다.
    const fn with_team(mut self, team: Team) -> Self {
        self.0 &= !(Self::TEAM_BIT_MASK << Self::TEAM_SHIFT);
        self.0 |= ((team as u8) & Self::TEAM_BIT_MASK) << Self::TEAM_SHIFT;
        self
    }

    /// 팀 내의 인덱스를 반환합니다.
    fn team_index(&self) -> usize {
        ((self.0 >> Self::INDEX_SHIFT) & Self::INDEX_BIT_MASK) as usize
    }

    /// 팀 내의 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 팀 인덱스가 5이상인 경우 [`panic`]을 호출합니다.
    const fn with_team_index(mut self, index: usize) -> Self {
        assert!(index < 5, "index out of range!");
        self.0 &= !(Self::INDEX_BIT_MASK << Self::INDEX_SHIFT);
        self.0 |= ((index as u8) & Self::INDEX_BIT_MASK) << Self::INDEX_SHIFT;
        self
    }

    /// 게임 티어를 반환합니다.
    fn tier(&self) -> GameTier {
        let val = (self.0 >> Self::TIER_SHIFT) & Self::TIER_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { GameTier::new(val).unwrap_unchecked() }
    }

    /// 게임 티어를 설정합니다.
    const fn with_tier(mut self, tier: GameTier) -> Self {
        self.0 &= !(Self::TIER_BIT_MASK << Self::TIER_SHIFT);
        self.0 |= ((tier as u8) & Self::TIER_BIT_MASK) << Self::TIER_SHIFT;
        self
    }

    /// 서버 접속 여부를 반환합니다.
    fn is_connected(&self) -> bool {
        (self.0 >> Self::CONNECT_SHIFT) & Self::CONNECT_BIT_MASK == Self::CONNECT_BIT_MASK
    }

    /// 서버 접속 여부를 설정합니다.
    const fn with_connected(mut self, connected: bool) -> Self {
        self.0 &= !(Self::CONNECT_BIT_MASK << Self::CONNECT_SHIFT);
        self.0 |= ((connected as u8) & Self::CONNECT_BIT_MASK) << Self::CONNECT_SHIFT;
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

/// 플레이어의 게임 결과 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGamePlayerResultData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 사용자 프로필 아이콘 종류
    pub profile_icon: ProfileIcon,
    /// 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 상대 팀을 처치한 횟수입니다.
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수입니다.
    pub retreat_count: u16,
    /// 상대 팀에게 입힌 총 데미지
    pub damage_dealt: u32,
    /// 상대 팀에게 입은 총 데미지
    pub damage_taken: u32,
    /// 같은 팀을 회복시킨 회복량
    pub healing_given: u32,
    /// 게임 티어 점수 변동량
    pub tier_score_delta: i16,
    /// 비트 필드 데이터
    bitfield: Bitfield,
}

impl InGamePlayerResultData {
    /// 새로운 `InGamePlayerResultData`를 생성합니다.
    ///
    /// # Panics
    /// 주어진 팀 인덱스가 5이상인 경우 [`panic`]을 호출합니다.
    ///
    pub const fn new(
        uid: UserId,
        name: UserName,
        profile_icon: ProfileIcon,
        character_kind: CharacterKind,
        kill_count: u16,
        retreat_count: u16,
        damage_dealt: u32,
        damage_taken: u32,
        healing_given: u32,
        is_connected: bool,
        tier_score_delta: i16,
        team: Team,
        index: usize,
        tier: GameTier,
    ) -> Self {
        Self {
            uid,
            name,
            profile_icon,
            character_kind,
            kill_count,
            retreat_count,
            damage_dealt,
            damage_taken,
            healing_given,
            tier_score_delta,
            bitfield: Bitfield::new()
                .with_team(team)
                .with_team_index(index)
                .with_tier(tier)
                .with_connected(is_connected),
        }
    }

    /// 팀 종류를 반환합니다.
    pub fn team(&self) -> Team {
        self.bitfield.team()
    }

    /// 팀 인덱스를 반환합니다.
    pub fn team_index(&self) -> usize {
        self.bitfield.team_index()
    }

    /// 게임 티어를 반환합니다.
    pub fn tier(&self) -> GameTier {
        self.bitfield.tier()
    }

    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }
}

impl BigEndian for InGamePlayerResultData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + UserName::byte_size()
            + ProfileIcon::byte_size()
            + CharacterKind::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + i16::byte_size()
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
        bytes.extend_from_slice(&self.profile_icon.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.retreat_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_dealt.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_taken.to_big_endian_bytes());
        bytes.extend_from_slice(&self.healing_given.to_big_endian_bytes());
        bytes.extend_from_slice(&self.tier_score_delta.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerResultData),
            );
        }

        bytes
    }
}

impl TryFromBigEndian for InGamePlayerResultData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerResultData),
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

        // 사용자 프로필 아이콘 종류를 가져옵니다.
        offset = offset + size;
        size = ProfileIcon::byte_size();
        data = &bytes[offset..offset + size];
        let profile_icon = ProfileIcon::try_from_big_endian_bytes(data)?;

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 상대 팀을 처치한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 처치당한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let retreat_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 입힌 데미지량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_dealt = u32::from_big_endian_bytes(data);

        // 상대 팀에게 입은 데미지량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_taken = u32::from_big_endian_bytes(data);

        // 같은 팀을 회복시킨 회복량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let healing_given = u32::from_big_endian_bytes(data);

        // 게임 티어 점수 변동량을 가져옵니다.
        offset = offset + size;
        size = i16::byte_size();
        data = &bytes[offset..offset + size];
        let tier_score_delta = i16::from_big_endian_bytes(data);

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Some(Self {
            uid,
            name,
            profile_icon,
            character_kind,
            kill_count,
            retreat_count,
            damage_dealt,
            damage_taken,
            healing_given,
            tier_score_delta,
            bitfield,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_game_player_result_data() {
        let origin = InGamePlayerResultData::new(
            UserId::new(851341),
            UserName::from_str("아리스"),
            ProfileIcon::CharacterAris,
            CharacterKind::ArisOriginal,
            31,
            12,
            51341,
            3112,
            0,
            true,
            -10,
            Team::Blue,
            2,
            GameTier::Gold,
        );
        let bytes = origin.to_big_endian_bytes();
        let other = InGamePlayerResultData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
