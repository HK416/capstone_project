use mod_network::components::{CharacterKind, NetworkState, Permission, Team, UserId, UserName};

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - team          | 1bit | 팀 종류
/// - team_index    | 3bit | 팀 인덱스
/// - connected     | 1bit | 서버 연결 여부
/// - premission    | 1bit | 권한
/// - network_state | 2bit | 네트워크 상태
///
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const TEAM_BIT_MASK: u8 = 0x01;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u8 = 0x03;
    const INDEX_SHIFT: usize = 1;
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 4;
    const PERMISSION_BIT_MASK: u8 = 0x01;
    const PERMISSION_SHIFT: usize = 5;
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

    /// 네트워크 상태를 반환합니다.
    fn network_state(&self) -> NetworkState {
        let val = (self.0 >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK;
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { NetworkState::new(val).unwrap_unchecked() }
    }

    /// 네트워크 상태를 설정합니다.
    fn with_network_state(mut self, state: NetworkState) -> Self {
        self.0 &= !(Self::STATE_BIT_MASK << Self::STATE_SHIFT);
        self.0 |= ((state as u8) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
        self
    }
}

/// 캐릭터 편성 단계의 플레이어 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationPlayerData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 선택한 캐릭터 종류
    pub character_kind: Option<CharacterKind>,
    /// 비트 필드 데이터
    bitfield: Bitfield,
}

impl FormationPlayerData {
    /// 새로운 플레이어 데이터를 생성합니다.
    pub const fn new(uid: UserId, name: UserName, team: Team, index: usize) -> Self {
        Self {
            uid,
            name,
            character_kind: None,
            bitfield: Bitfield::new().with_team(team).with_team_index(index),
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

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }

    /// 서버 연결 여부를 설정합니다.
    pub fn set_connected(&mut self, connected: bool) {
        self.bitfield = self.bitfield.with_connected(connected);
    }

    /// 서버 연결 여부를 설정합니다.
    pub fn with_connected(mut self, connected: bool) -> Self {
        self.set_connected(connected);
        self
    }

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }

    /// 네트워크 상태를 설정합니다.
    pub fn set_network_state(&mut self, state: NetworkState) {
        self.bitfield = self.bitfield.with_network_state(state);
    }

    /// 네트워크 상태를 설정합니다.
    pub fn with_network_state(mut self, state: NetworkState) -> Self {
        self.set_network_state(state);
        self
    }

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }

    /// 권한을 설정합니다.
    pub fn set_permission_state(&mut self, permission: Permission) {
        self.bitfield = self.bitfield.with_permission(permission);
    }

    /// 권한을 설정합니다.
    pub fn with_permission_state(mut self, permission: Permission) -> Self {
        self.set_permission_state(permission);
        self
    }
}
