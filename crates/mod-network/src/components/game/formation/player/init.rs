//! 각 팀의 캐릭터를 편성하는 단계에 진입할 때 플레이어 데이터 초기화와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, Team, TryFromBigEndian, UserId, UserName};

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - team       | 1bit | 팀 종류
/// - team_index | 3bit | 팀 인덱스
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const TEAM_BIT_MASK: u8 = 0x01;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u8 = 0x07;
    const INDEX_SHIFT: usize = 1;

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

    /// 팀 내 인덱스를 반환합니다.
    fn team_index(&self) -> usize {
        ((self.0 >> Self::INDEX_SHIFT) & Self::INDEX_BIT_MASK) as usize
    }

    /// 팀 내의 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    const fn with_team_index(mut self, index: usize) -> Self {
        assert!(index < 5, "index out of ranges!");
        self.0 &= !(Self::INDEX_BIT_MASK << Self::INDEX_SHIFT);
        self.0 |= ((index as u8) & Self::INDEX_BIT_MASK) << Self::INDEX_SHIFT;
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

/// 캐릭터 편성 단계에 진입할 때 플레이어 초기화 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationPlayerInitData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 플레이어 비트 필드 데이터
    bitfield: Bitfield,
}

impl FormationPlayerInitData {
    /// 새로운 플레이어 초기화 데이터를 생성합니다.
    pub const fn new(uid: UserId, name: UserName, team: Team, index: usize) -> Self {
        Self {
            uid,
            name,
            bitfield: Bitfield::new().with_team(team).with_team_index(index),
        }
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

    /// 팀 인덱스를 반환합니다.
    pub fn team_index(&self) -> usize {
        self.bitfield.team_index()
    }

    /// 팀 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn set_team_index(&mut self, index: usize) {
        self.bitfield = self.bitfield.with_team_index(index);
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
}

impl BigEndian for FormationPlayerInitData {
    fn byte_size() -> usize {
        UserId::byte_size() + UserName::byte_size() + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.name.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 생성된 바이트가 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationPlayerInitData),
            )
        };

        bytes
    }
}

impl TryFromBigEndian for FormationPlayerInitData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 주어진 바이트가 유효한지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationPlayerInitData),
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

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Some(Self {
            uid,
            name,
            bitfield,
        })
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
    fn test_formation_player_init_data() {
        let origin = FormationPlayerInitData::new(
            UserId::new(12345),
            UserName::from_str("Aris Original"),
            Team::Red,
            2,
        );
        let bytes = origin.to_big_endian_bytes();
        let other = FormationPlayerInitData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 비교
        assert_eq!(origin, other);
    }
}
