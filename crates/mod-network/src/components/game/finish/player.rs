//! 플레이어의 게임 결과 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, CharacterKind, Team, TryFromBigEndian, UserId, UserName};

/// 플레이어 비트 필드 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bitfield(u8);

impl Bitfield {
    const TEAM_BIT_MASK: u8 = 0x01;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u8 = 0x07;
    const INDEX_SHIFT: usize = 1;
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 4;

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
    fn with_team(mut self, team: Team) -> Self {
        self.0 &= !(Self::TEAM_BIT_MASK << Self::TEAM_SHIFT);
        self.0 |= ((team as u8) & Self::TEAM_BIT_MASK) << Self::TEAM_SHIFT;
        self
    }

    /// 팀 내 인덱스를 반환합니다.
    fn team_index(&self) -> usize {
        ((self.0 >> Self::INDEX_SHIFT) & Self::INDEX_BIT_MASK) as usize
    }

    /// 팀 내의 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    fn with_team_index(mut self, index: usize) -> Self {
        assert!(index < 5, "index out of ranges!");
        self.0 &= !(Self::INDEX_BIT_MASK << Self::INDEX_SHIFT);
        self.0 |= ((index as u8) & Self::INDEX_BIT_MASK) << Self::INDEX_SHIFT;
        self
    }

    /// 서버 연결 여부를 반환합니다.
    fn is_connected(&self) -> bool {
        (self.0 >> Self::CONNECT_SHIFT) & Self::CONNECT_BIT_MASK == Self::CONNECT_BIT_MASK
    }

    /// 서버 연결 여부를 설정합니다.
    fn with_connected(mut self, connected: bool) -> Self {
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

/// 게임이 끝난 후 플레이어 결과 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameResultPlayerData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 선택한 캐릭터 종류입니다.
    pub character_kind: CharacterKind,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,

    /// 상대 팀을 처치한 횟수입니다.
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수입니다.
    pub dead_count: u16,

    /// 상대 팀에게 입힌 총 데미지입니다.
    pub damage_dealt: u32,
    /// 상대 팀에게 입은 총 데미지입니다.
    pub damage_taken: u32,
    /// 같은 팀에게 회복 시킨 회복량입니다.
    pub healing_given: u32,
}

impl GameResultPlayerData {
    /// 팀 종류를 반환합니다.
    pub fn team(&self) -> Team {
        self.bitfield.team()
    }

    /// 팀 인덱스를 반환합니다.
    pub fn team_index(&self) -> usize {
        self.bitfield.team_index()
    }

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }
}

impl BigEndian for GameResultPlayerData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + UserName::byte_size()
            + CharacterKind::byte_size()
            + Bitfield::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + u32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.name.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.dead_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_dealt.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_taken.to_big_endian_bytes());
        bytes.extend_from_slice(&self.healing_given.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(GameResultPlayerData),
            );
        }

        bytes
    }
}

impl TryFromBigEndian for GameResultPlayerData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(GameResultPlayerData),
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

        // 상대 팀을 처치한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 처치당한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let dead_count = u16::from_big_endian_bytes(data);

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

        Some(Self {
            uid,
            name,
            character_kind,
            bitfield,
            kill_count,
            dead_count,
            damage_dealt,
            damage_taken,
            healing_given,
        })
    }
}

/// 게임 결과 플레이어 데이터의 빌더입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameResultPlayerDataBuilder {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 선택한 캐릭터 종류입니다.
    pub character_kind: CharacterKind,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,

    /// 상대 팀을 처치한 횟수입니다.
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수입니다.
    pub dead_count: u16,

    /// 상대 팀에게 입힌 총 데미지입니다.
    pub damage_dealt: u32,
    /// 상대 팀에게 입은 총 데미지입니다.
    pub damage_taken: u32,
    /// 같은 팀에게 회복 시킨 회복량입니다.
    pub healing_given: u32,
}

impl GameResultPlayerDataBuilder {
    /// 새로운 빌더를 생성합니다.
    pub const fn new(uid: UserId, name: UserName) -> Self {
        Self {
            uid,
            name,
            character_kind: CharacterKind::ArisOriginal,
            bitfield: Bitfield::new(),
            kill_count: 0,
            dead_count: 0,
            damage_dealt: 0,
            damage_taken: 0,
            healing_given: 0,
        }
    }

    /// 캐릭터 종류를 설정합니다.
    pub fn with_character_kind(mut self, character_kind: CharacterKind) -> Self {
        self.character_kind = character_kind;
        self
    }

    /// 팀 종류를 설정합니다.
    pub fn with_team(mut self, team: Team) -> Self {
        self.bitfield = self.bitfield.with_team(team);
        self
    }

    /// 팀 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn with_team_index(mut self, index: usize) -> Self {
        self.bitfield = self.bitfield.with_team_index(index);
        self
    }

    /// 서버 연결 여부를 설정합니다.
    pub fn with_connected(mut self, connected: bool) -> Self {
        self.bitfield = self.bitfield.with_connected(connected);
        self
    }

    /// 상대 팀 처치 횟수를 설정합니다.
    pub fn with_kill_count(mut self, kill_count: u16) -> Self {
        self.kill_count = kill_count;
        self
    }

    /// 상대 팀에게 처치당한 횟수를 설정합니다.
    pub fn with_dead_count(mut self, dead_count: u16) -> Self {
        self.dead_count = dead_count;
        self
    }

    /// 상대 팀에게 입힌 데미지 량을 설정합니다.
    pub fn with_damage_dealt(mut self, damage_dealt: u32) -> Self {
        self.damage_dealt = damage_dealt;
        self
    }

    /// 상대 팀에게 입은 데미지 량을 설정합니다.
    pub fn with_damage_taken(mut self, damage_taken: u32) -> Self {
        self.damage_taken = damage_taken;
        self
    }

    /// 아군에게 준 회복량을 설정합니다.
    pub fn with_healing_given(mut self, healing_given: u32) -> Self {
        self.healing_given = healing_given;
        self
    }

    /// 플레이어 게임 결과 데이터를 생성합니다.
    pub fn build(self) -> GameResultPlayerData {
        GameResultPlayerData {
            uid: self.uid,
            name: self.name,
            character_kind: self.character_kind,
            bitfield: self.bitfield,
            kill_count: self.kill_count,
            dead_count: self.dead_count,
            damage_dealt: self.damage_dealt,
            damage_taken: self.damage_taken,
            healing_given: self.healing_given,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    #[should_panic]
    fn test_creation_bitfield_team_index() {
        Bitfield::new().with_team_index(5);
    }

    #[test]
    fn test_bitfield_team_index() {
        let bitfield = Bitfield::new().with_team_index(0);
        assert_eq!(0, bitfield.team_index());

        let bitfield = Bitfield::new().with_team_index(1);
        assert_eq!(1, bitfield.team_index());

        let bitfield = Bitfield::new().with_team_index(2);
        assert_eq!(2, bitfield.team_index());

        let bitfield = Bitfield::new().with_team_index(3);
        assert_eq!(3, bitfield.team_index());

        let bitfield = Bitfield::new().with_team_index(4);
        assert_eq!(4, bitfield.team_index());
    }

    #[test]
    fn test_bitfield_connected() {
        let bitfield = Bitfield::new().with_connected(false);
        assert_eq!(false, bitfield.is_connected());

        let bitfield = Bitfield::new().with_connected(true);
        assert_eq!(true, bitfield.is_connected());
    }

    #[test]
    fn test_game_result_player_data() {
        let origin =
            GameResultPlayerDataBuilder::new(UserId::new(15132451), UserName::from_str("블붕이"))
                .with_character_kind(CharacterKind::MidoriOriginal)
                .with_team(Team::Red)
                .with_team_index(4)
                .with_kill_count(13)
                .with_dead_count(2)
                .with_damage_dealt(888431)
                .with_damage_taken(9341)
                .with_healing_given(0)
                .build();
        let bytes = origin.to_big_endian_bytes();
        let other = GameResultPlayerData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
