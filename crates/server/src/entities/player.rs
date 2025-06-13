//! 게임 월드의 플레이어 데이터와 관련된 코드를 관리합니다.
//!

use mod_network::components::{GameTier, Permission, ProfileIcon, Team, UserName};

/// 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - team          | 1bit | 팀 종류
/// - team_index    | 3bit | 팀 인덱스
/// - ready_to_play | 1bit | 게임 준비 여부
/// - permission    | 1bit | 권한
/// - tier          | 2bit | 티어
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u16);

impl Bitfield {
    const TEAM_BIT_MASK: u16 = 0x0001;
    const TEAM_SHIFT: usize = 0;
    const INDEX_BIT_MASK: u16 = 0x0007;
    const INDEX_SHIFT: usize = 1;
    const READY_BIT_MASK: u16 = 0x0001;
    const READY_SHIFT: usize = 4;
    const PERMISSION_BIT_MASK: u16 = 0x0001;
    const PERMISSION_SHIFT: usize = 5;
    const TIER_BIT_MASK: u16 = 0x0007;
    const TIER_SHIFT: usize = 6;

    /// 새로운 비트 필드 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 팀 종류를 반환합니다.
    pub fn team(&self) -> Team {
        let val = ((self.0 >> Self::TEAM_SHIFT) & Self::TEAM_BIT_MASK) as u8;
        // Safety: 주어진 정수는 범위를 벗어나지 않음
        unsafe { Team::new(val).unwrap_unchecked() }
    }

    /// 팀 종류를 설정합니다.
    pub const fn set_team(&mut self, team: Team) {
        self.0 &= !(Self::TEAM_BIT_MASK << Self::TEAM_SHIFT);
        self.0 |= ((team as u16) & Self::TEAM_BIT_MASK) << Self::TEAM_SHIFT;
    }

    /// 팀 종류를 설정합니다.
    pub const fn with_team(mut self, team: Team) -> Self {
        self.set_team(team);
        self
    }

    /// 팀 인덱스를 반환합니다.
    pub fn team_index(&self) -> usize {
        ((self.0 >> Self::INDEX_SHIFT) & Self::INDEX_BIT_MASK) as usize
    }

    /// 팀 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn set_team_index(&mut self, index: usize) {
        assert!(index < 5, "index out of ranges!");
        self.0 &= !(Self::INDEX_BIT_MASK << Self::INDEX_SHIFT);
        self.0 |= ((index as u16) & Self::INDEX_BIT_MASK) << Self::INDEX_SHIFT;
    }

    /// 팀 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn with_team_index(mut self, index: usize) -> Self {
        self.set_team_index(index);
        self
    }

    /// 준비 여부를 반환합니다.
    pub fn is_ready_to_play(&self) -> bool {
        (self.0 >> Self::READY_SHIFT) & Self::READY_BIT_MASK == Self::READY_BIT_MASK
    }

    /// 준비 여부를 설정합니다.
    pub const fn set_ready_to_play(&mut self, ready: bool) {
        self.0 &= !(Self::READY_BIT_MASK << Self::READY_SHIFT);
        self.0 |= ((ready as u16) & Self::READY_BIT_MASK) << Self::READY_SHIFT;
    }

    /// 준비 여부를 설정합니다.
    pub const fn with_ready_to_play(mut self, ready: bool) -> Self {
        self.set_ready_to_play(ready);
        self
    }

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        let val = ((self.0 >> Self::PERMISSION_SHIFT) & Self::PERMISSION_BIT_MASK) as u8;
        // Safety: 주어진 정수는 범위를 벗어나지 않음
        unsafe { Permission::new(val).unwrap_unchecked() }
    }

    /// 권한을 설정합니다.
    pub const fn set_permission(&mut self, permission: Permission) {
        self.0 &= !(Self::PERMISSION_BIT_MASK << Self::PERMISSION_SHIFT);
        self.0 |= ((permission as u16) & Self::PERMISSION_BIT_MASK) << Self::PERMISSION_SHIFT;
    }

    /// 권한을 설정합니다.
    pub const fn with_permission(mut self, permission: Permission) -> Self {
        self.set_permission(permission);
        self
    }

    /// 게임 티어를 반환합니다.
    pub fn tier(&self) -> GameTier {
        let val = ((self.0 >> Self::TIER_SHIFT) & Self::TIER_BIT_MASK) as u8;
        // Safety: 주어진 정수는 범위를 벗어나지 않음
        unsafe { GameTier::new(val).unwrap_unchecked() }
    }

    /// 게임 티어를 설정합니다.
    pub const fn set_tier(&mut self, tier: GameTier) {
        self.0 &= !(Self::TIER_BIT_MASK << Self::TIER_SHIFT);
        self.0 |= ((tier as u16) & Self::TIER_BIT_MASK) << Self::TIER_SHIFT;
    }

    /// 게임 티어를 설정합니다.
    pub const fn with_tier(mut self, tier: GameTier) -> Self {
        self.set_tier(tier);
        self
    }
}

impl Default for Bitfield {
    fn default() -> Self {
        Self(0x00)
    }
}

/// 플레이어 데이터입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Player {
    /// 사용자 이름
    pub name: UserName,
    /// 사용자 프로필 아이콘
    pub profile_icon: ProfileIcon,
    /// 비트 필드 데이터입니다.
    bitfield: Bitfield,
}

impl Player {
    /// 플레이어 데이터를 생성합니다.
    pub const fn new(name: UserName) -> Self {
        Self {
            name,
            profile_icon: ProfileIcon::GroupSchale,
            bitfield: Bitfield::new(),
        }
    }

    /// 프로필 아이콘을 설정합니다.
    pub const fn with_profile_icon(mut self, profile_icon: ProfileIcon) -> Self {
        self.profile_icon = profile_icon;
        self
    }

    /// 팀 종류를 반환합니다.
    pub fn team(&self) -> Team {
        self.bitfield.team()
    }

    /// 팀 종류를 설정합니다.
    pub const fn set_team(&mut self, team: Team) {
        self.bitfield.set_team(team);
    }

    /// 팀 종류를 설정합니다.
    pub const fn with_team(mut self, team: Team) -> Self {
        self.set_team(team);
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
    pub const fn set_team_index(&mut self, index: usize) {
        self.bitfield.set_team_index(index);
    }

    /// 팀 인덱스를 설정합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 5이상인 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn with_team_index(mut self, index: usize) -> Self {
        self.set_team_index(index);
        self
    }

    /// 준비 여부를 반환합니다.
    pub fn is_ready_to_play(&self) -> bool {
        self.bitfield.is_ready_to_play()
    }

    /// 준비 여부를 설정합니다.
    pub const fn set_ready_to_play(&mut self, ready: bool) {
        self.bitfield.set_ready_to_play(ready);
    }

    /// 준비 여부를 설정합니다.
    pub const fn with_ready_to_play(mut self, ready: bool) -> Self {
        self.set_ready_to_play(ready);
        self
    }

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }

    /// 권한을 설정합니다.
    pub const fn set_permission(&mut self, permission: Permission) {
        self.bitfield.set_permission(permission);
    }

    /// 권한을 설정합니다.
    pub const fn with_permission(mut self, permission: Permission) -> Self {
        self.set_permission(permission);
        self
    }

    /// 게임 티어를 반환합니다.
    pub fn tier(&self) -> GameTier {
        self.bitfield.tier()
    }

    /// 게임 티어를 설정합니다.
    pub const fn set_tier(&mut self, tier: GameTier) {
        self.bitfield.set_tier(tier);
    }

    /// 게임 티어를 설정합니다.
    pub const fn with_tier(mut self, tier: GameTier) -> Self {
        self.set_tier(tier);
        self
    }
}
