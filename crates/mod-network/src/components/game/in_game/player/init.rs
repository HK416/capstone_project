//! 인게임 단계에 진입할 때 플레이어 데이터 초기화와 관련된 코드를 관리합니다.
//!

use crate::components::{
    BigEndian, CharacterKind, LatLon, NetworkState, Permission, Team, TryFromBigEndian, UserId,
    UserName,
};

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - team          | 1bit | 팀 종류
/// - team_index    | 3bit | 팀 인덱스
/// - permission    | 1bit | 권한
/// - connected     | 1bit | 서버 연결 여부
/// - network_state | 2bit | 네트워크 상태
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const TEAM_BIT_MASK: u8 = 0x01;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u8 = 0x07;
    const INDEX_SHIFT: usize = 1;
    const PERMISSION_BIT_MASK: u8 = 0x01;
    const PERMISSION_SHIFT: usize = 4;
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 5;
    const STATE_BIT_MASK: u8 = 0x03;
    const STATE_SHIFT: usize = 6;

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

    /// 권한을 반환합니다.
    fn permission(&self) -> Permission {
        let val = (self.0 >> Self::PERMISSION_SHIFT) & Self::PERMISSION_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { Permission::new(val).unwrap_unchecked() }
    }

    /// 권한을 설정합니다.
    const fn with_permission(mut self, permission: Permission) -> Self {
        self.0 &= !(Self::PERMISSION_BIT_MASK << Self::PERMISSION_SHIFT);
        self.0 |= ((permission as u8) & Self::PERMISSION_BIT_MASK) << Self::PERMISSION_SHIFT;
        self
    }

    /// 서버 연결 여부를 반환합니다.
    fn is_connected(&self) -> bool {
        (self.0 >> Self::CONNECT_SHIFT) & Self::CONNECT_BIT_MASK == Self::CONNECT_BIT_MASK
    }

    /// 서버 연결 여부를 설정합니다.
    const fn with_connected(mut self, connected: bool) -> Self {
        self.0 &= !(Self::CONNECT_BIT_MASK << Self::CONNECT_SHIFT);
        self.0 |= ((connected as u8) & Self::CONNECT_BIT_MASK) << Self::CONNECT_SHIFT;
        self
    }

    /// 네트워크 상태를 반환합니다.
    fn network_state(&self) -> NetworkState {
        let val = (self.0 >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { NetworkState::new(val).unwrap_unchecked() }
    }

    /// 네트워크 상태를 설정합니다.
    const fn with_network_state(mut self, state: NetworkState) -> Self {
        self.0 &= !(Self::STATE_BIT_MASK << Self::STATE_SHIFT);
        self.0 |= ((state as u8) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
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

/// 인게임 플레이어 초기화 데이터입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePlayerInitData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 비트 필드 데이터
    bitfield: Bitfield,

    /// 체력 데이터
    pub maximum_health: u16,
    /// 총알 데이터
    pub maximum_bullet: u16,
    /// 스킬 코스트 데이터
    pub maximum_skill_cost: u16,

    /// 월드 공간 위치 (플레이어 캐릭터 스폰 위치)
    pub translation: [f32; 3],
    /// 월드 공간 방향 (플레이어 캐릭터 스폰 방향)
    pub rotation: [f32; 4],
    /// 카메라 방향 (플레이어 카메라 스폰 방향)
    pub latlon: LatLon,
}

impl InGamePlayerInitData {
    /// 새로운 플레이어 초기화 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 팀 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(
        uid: UserId,
        name: UserName,
        character_kind: CharacterKind,
        team: Team,
        team_index: usize,
        permission: Permission,
        connected: bool,
        network_state: NetworkState,
        maximum_health: u16,
        maximum_bullet: u16,
        maximum_skill_cost: u16,
        translation: [f32; 3],
        rotation: [f32; 4],
        latlon: LatLon,
    ) -> Self {
        Self {
            uid,
            name,
            character_kind,
            bitfield: Bitfield::new()
                .with_team(team)
                .with_team_index(team_index)
                .with_permission(permission)
                .with_connected(connected)
                .with_network_state(network_state),
            maximum_health,
            maximum_bullet,
            maximum_skill_cost,
            translation,
            rotation,
            latlon,
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

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }
}

impl BigEndian for InGamePlayerInitData {
    fn byte_size() -> usize {
        UserId::byte_size()    // 4byte
            + UserName::byte_size()    // 37byte
            + CharacterKind::byte_size()    // 38byte
            + Bitfield::byte_size()    // 39byte
            + u16::byte_size()    // 41byte
            + u16::byte_size()    // 43byte
            + u16::byte_size()    // 45byte
            + <[f32; 3]>::byte_size()    // 57byte
            + <[f32; 4]>::byte_size()    // 73byte
            + LatLon::byte_size() // 77byte
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
        bytes.extend_from_slice(&self.maximum_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum_bullet.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum_skill_cost.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.latlon.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerInitData)
            );
        }

        bytes
    }
}

impl TryFromBigEndian for InGamePlayerInitData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerInitData)
            )
        };

        // 사용자 식별자 데이터를 가져옵니다.
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
        let character_kind = CharacterKind::from_big_endian_bytes(data);

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        // 최대 체력을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum_health = u16::from_big_endian_bytes(data);

        // 최대 총알 개수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum_bullet = u16::from_big_endian_bytes(data);

        // 최대 스킬 코스트를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum_skill_cost = u16::from_big_endian_bytes(data);

        // 월드 공간 위치를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[f32; 3]>::from_big_endian_bytes(data);

        // 월드 공간 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[f32; 4]>::from_big_endian_bytes(data);

        // 카메라 방향을 가져옵니다.
        offset = offset + size;
        size = LatLon::byte_size();
        data = &bytes[offset..offset + size];
        let latlon = LatLon::from_big_endian_bytes(data);

        Some(Self {
            uid,
            name,
            character_kind,
            bitfield,
            maximum_health,
            maximum_bullet,
            maximum_skill_cost,
            translation,
            rotation,
            latlon,
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
    fn test_bitfield_permission() {
        let val = Permission::Admin;
        let bitfield = Bitfield::new().with_permission(val);
        assert_eq!(Permission::Admin, bitfield.permission());

        let val = Permission::User;
        let bitfield = Bitfield::new().with_permission(val);
        assert_eq!(Permission::User, bitfield.permission());
    }

    #[test]
    fn test_in_game_player_init_data() {
        let origin = InGamePlayerInitData::new(
            UserId::new(123515),
            UserName::from_str("블붕이"),
            CharacterKind::MomoiOriginal,
            Team::Red,
            2,
            Permission::Admin,
            true,
            NetworkState::Good,
            12354,
            123,
            1234,
            [0.14532151, 3.134151, -1.02515614],
            [0.00013412, 0.00134141, 0.91413541, 0.004312451],
            LatLon::new(0.00034115, 1.024111),
        );
        let bytes = origin.to_big_endian_bytes();
        let other = InGamePlayerInitData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
