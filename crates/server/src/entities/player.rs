//! 게임 월드의 플레이어 데이터와 관련된 코드를 관리합니다.
//!

use mod_network::components::{
    ActionState, ActionStateTimer, BulletData, CharacterAttributes, CharacterKind, GameInputBits,
    GameTier, HealthData, InputStateTimer, LatLon, MovementState, MovementStateTimer, NetworkState,
    Permission, PlayerStateData, ProfileIcon, SkillCostData, Team, UserName, ViewStateTimer,
};

use crate::data::get_character_attributes;

/// 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - team          | 1bit | 팀 종류
/// - team_index    | 3bit | 팀 인덱스
/// - ready_to_play | 1bit | 게임 준비 여부
/// - permission    | 1bit | 권한
/// - tier          | 2bit | 티어
/// - network_state | 2bit | 네트워크 상태
/// - invincible    | 1bit | 무적 여부
/// - grounded      | 1bit | 지면을 밟고 있는 여부
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
    const STATE_BIT_MASK: u16 = 0x0003;
    const STATE_SHIFT: usize = 8;
    const INVINCIBLE_BIT_MASK: u16 = 0x0001;
    const INVINCIBLE_SHIFT: usize = 10;
    const GROUND_BIT_MASK: u16 = 0x0001;
    const GROUND_SHIFT: usize = 11;

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

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        let val = ((self.0 >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK) as u8;
        // Safety: 주어진 정수는 범위를 벗어나지 않음
        unsafe { NetworkState::new(val).unwrap_unchecked() }
    }

    /// 네트워크 상태를 설정합니다.
    pub const fn set_network_state(&mut self, state: NetworkState) {
        self.0 &= !(Self::STATE_BIT_MASK << Self::STATE_SHIFT);
        self.0 |= ((state as u16) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
    }

    /// 네트워크 상태를 설정합니다.
    pub const fn with_network_state(mut self, state: NetworkState) -> Self {
        self.set_network_state(state);
        self
    }

    /// 무적 여부를 반환합니다.
    pub fn is_invincible(&self) -> bool {
        ((self.0 >> Self::INVINCIBLE_SHIFT) & Self::INVINCIBLE_BIT_MASK)
            == Self::INVINCIBLE_BIT_MASK
    }

    /// 무적 여부를 설정합니다.
    pub const fn set_invincible(&mut self, invincible: bool) {
        self.0 &= !(Self::INVINCIBLE_BIT_MASK << Self::INVINCIBLE_SHIFT);
        self.0 |= ((invincible as u16) & Self::INVINCIBLE_BIT_MASK) << Self::INVINCIBLE_SHIFT;
    }

    /// 무적 여부를 반환합니다.
    pub const fn with_invincible(mut self, invincible: bool) -> Self {
        self.set_invincible(invincible);
        self
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    pub fn is_grounded(&self) -> bool {
        ((self.0 >> Self::GROUND_SHIFT) & Self::GROUND_BIT_MASK) == Self::GROUND_BIT_MASK
    }

    /// 지면을 밟고 있는 여부를 설정합니다.
    pub const fn set_grounded(&mut self, ground: bool) {
        self.0 &= !(Self::GROUND_BIT_MASK << Self::GROUND_SHIFT);
        self.0 |= ((ground as u16) & Self::GROUND_BIT_MASK) << Self::GROUND_SHIFT;
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    pub const fn with_grounded(mut self, ground: bool) -> Self {
        self.set_grounded(ground);
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
    pub name: UserName, // 34
    /// 사용자 프로필 아이콘
    pub profile_icon: ProfileIcon, // 35
    /// 캐릭터 종류
    character_kind: CharacterKind, // 36
    /// 행동 상태
    pub action_state: ActionState, // 37
    /// 움직임 상태
    pub movement_state: MovementState, // 38
    /// 행동 상태 타이머
    pub action_state_timer: ActionStateTimer, // 40
    /// 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer, // 42
    /// 플레이어 시야 각도
    pub latlon: LatLon, // 46

    /// 비트 필드 데이터입니다.
    bitfield: Bitfield, // 48

    /// 상대 팀을 처치한 횟수
    pub kill_count: u16, // 50
    /// 상대 팀에게 처치 당한 횟수
    pub dead_count: u16, // 52
    /// 체력 데이터
    pub health_data: HealthData, // 58
    /// 총알 데이터
    pub bullet_data: BulletData, // 64

    /// 스킬 코스트 데이터
    pub skill_cost_data: SkillCostData, // 68
    /// 캐릭터 속성 데이터
    attributes: &'static CharacterAttributes, // 72
    /// 스킬 코스트 갱신에 사용되는 타이머입니다. (단위: ms)
    pub skill_cost_timer: u16, // 74
    /// 입력 상태 타이머
    pub input_state_timer: InputStateTimer, // 76
    /// 게임 입력 비트 플래그
    pub input_bits: GameInputBits, // 78

    /// 플레이어 월드 공간 위치
    pub translation: glam::Vec3A, // 96

    /// 플레이어 월드 공간 방향
    pub rotation: glam::Quat, // 112

    /// 플레이어 월드 공간 이동 방향
    pub velocity: glam::Vec3A, // 128

                               // ------ 128byte --------
}

impl Player {
    /// 플레이어 데이터를 생성합니다.
    pub fn new(
        name: UserName,
        profile_icon: ProfileIcon,
        permission: Permission,
        tier: GameTier,
    ) -> Self {
        Self {
            name,
            profile_icon,
            character_kind: CharacterKind::ArisOriginal,
            action_state: ActionState::Idle,
            movement_state: MovementState::Idle,
            action_state_timer: ActionStateTimer(0),
            movement_state_timer: MovementStateTimer(0),
            latlon: LatLon::default(),
            bitfield: Bitfield::new().with_permission(permission).with_tier(tier),
            kill_count: 0,
            dead_count: 0,
            health_data: HealthData::default(),
            bullet_data: BulletData::default(),
            skill_cost_data: SkillCostData::default(),
            attributes: get_character_attributes(CharacterKind::ArisOriginal),
            skill_cost_timer: 0,
            input_state_timer: InputStateTimer(0),
            input_bits: GameInputBits::default(),
            translation: glam::Vec3A::ZERO,
            rotation: glam::Quat::IDENTITY,
            velocity: glam::Vec3A::ZERO,
        }
    }

    /// 프로필 아이콘을 설정합니다.
    pub const fn with_profile_icon(mut self, profile_icon: ProfileIcon) -> Self {
        self.profile_icon = profile_icon;
        self
    }

    /// 캐릭터 종류를 설정합니다.
    pub fn set_character_kind(&mut self, character_kind: CharacterKind) {
        let attributes = get_character_attributes(self.character_kind);
        self.character_kind = character_kind;
        self.attributes = attributes;

        self.health_data = HealthData::splat(attributes.max_health_point);
        self.bullet_data = BulletData::splat(attributes.max_bullets);
        self.skill_cost_data = SkillCostData::new(0, attributes.max_skill_cost);
    }

    /// 캐릭터 종류를 설정합니다.
    pub fn with_character_kind(mut self, character_kind: CharacterKind) -> Self {
        self.set_character_kind(character_kind);
        self
    }

    /// 캐릭터 종류를 반환합니다.
    pub fn character_kind(&self) -> CharacterKind {
        self.character_kind
    }

    /// 캐릭터 속성 데이터를 반환합니다.
    pub fn character_attributes(&self) -> &'static CharacterAttributes {
        self.attributes
    }

    pub fn player_states(&self) -> PlayerStateData {
        PlayerStateData::new()
            .with_action_state(self.action_state)
            .with_movement_state(self.movement_state)
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

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }

    /// 네트워크 상태를 설정합니다.
    pub const fn set_network_state(&mut self, state: NetworkState) {
        self.bitfield.set_network_state(state);
    }

    /// 네트워크 상태를 설정합니다.
    pub const fn with_network_state(mut self, state: NetworkState) -> Self {
        self.set_network_state(state);
        self
    }

    /// 무적 여부를 반환합니다.
    pub fn is_invincible(&self) -> bool {
        self.bitfield.is_invincible()
    }

    /// 무적 여부를 설정합니다.
    pub const fn set_invincible(&mut self, invincible: bool) {
        self.bitfield.set_invincible(invincible);
    }

    /// 무적 여부를 반환합니다.
    pub const fn with_invincible(mut self, invincible: bool) -> Self {
        self.set_invincible(invincible);
        self
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    pub fn is_grounded(&self) -> bool {
        self.bitfield.is_grounded()
    }

    /// 지면을 밟고 있는 여부를 설정합니다.
    pub const fn set_grounded(&mut self, ground: bool) {
        self.bitfield.set_grounded(ground);
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    pub const fn with_grounded(mut self, ground: bool) -> Self {
        self.set_grounded(ground);
        self
    }

    /// 플레이어 캐릭터의 최대 체력을 반환합니다.
    pub fn maximum_health(&self) -> u16 {
        self.attributes.max_health_point
    }

    /// 플레이어 캐릭터의 최대 총알 개수를 반환합니다.
    pub fn maximum_bullet(&self) -> u16 {
        self.attributes.max_bullets
    }

    /// 플레이어 캐릭터의 최대 스킬 코스트를 반환합니다.
    pub fn maximum_skill_cost(&self) -> u16 {
        self.attributes.max_skill_cost
    }
}
