//! 인게임 단계에서 플레이어 데이터 갱신과 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionState, ActionStateTimer, BigEndian, LatLon, MovementState, MovementStateTimer,
    NetworkState, Permission, PlayerStateData, TryFromBigEndian, UserId,
};

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - permission    | 1bit | 권한
/// - connected     | 1bit | 서버 연결 여부
/// - grounded      | 1bit | 지면을 밟고 있는 여부
/// - invincible    | 1bit | 무적 여부
/// - network_state | 2bit | 네트워크 상태
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const PERMISSION_BIT_MASK: u8 = 0x01;
    const PERMISSION_SHIFT: usize = 0;
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 1;
    const INVINCIBLE_BIT_MASK: u8 = 0x01;
    const INVINCIBLE_SHIFT: usize = 2;
    const GROUND_BIT_MASK: u8 = 0x01;
    const GROUND_SHIFT: usize = 3;
    const STATE_BIT_MASK: u8 = 0x03;
    const STATE_SHIFT: usize = 4;

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

    /// 무적 여부를 반환합니다.
    fn is_invincible(&self) -> bool {
        (self.0 >> Self::INVINCIBLE_SHIFT) & Self::INVINCIBLE_BIT_MASK == Self::INVINCIBLE_BIT_MASK
    }

    /// 무적 여부를 설정합니다.
    const fn with_invincible(mut self, invincible: bool) -> Self {
        self.0 &= !(Self::INVINCIBLE_BIT_MASK << Self::INVINCIBLE_SHIFT);
        self.0 |= ((invincible as u8) & Self::INVINCIBLE_BIT_MASK) << Self::INVINCIBLE_SHIFT;
        self
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    fn is_grounded(&self) -> bool {
        (self.0 >> Self::GROUND_SHIFT) & Self::GROUND_BIT_MASK == Self::GROUND_BIT_MASK
    }

    /// 지면을 밟고 있는 여부를 설정합니다.
    const fn with_grounded(mut self, grounded: bool) -> Self {
        self.0 &= !(Self::GROUND_BIT_MASK << Self::GROUND_SHIFT);
        self.0 |= ((grounded as u8) & Self::GROUND_BIT_MASK) << Self::GROUND_SHIFT;
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

/// 상태 이벤트입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateChangeEvent {
    ActionState {
        action_state: ActionState,
        play_elapsed_time_ms: u32,
    },
    MovementState {
        movement_state: MovementState,
        play_elapsed_time_ms: u32,
    },
}

impl StateChangeEvent {
    const TIME_BIT_MASK: u32 = 0x00FFFFFF;
    const TIME_SHIFT: usize = 0;
    const STATE_BIT_MASK: u32 = 0x7F;
    const STATE_SHIFT: usize = 24;
    const KIND_BIT_MASK: u32 = 0x1;
    const KIND_SHIFT: usize = 31;
}

impl BigEndian for StateChangeEvent {
    fn byte_size() -> usize {
        u32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        match *self {
            StateChangeEvent::ActionState {
                action_state,
                play_elapsed_time_ms,
            } => {
                let kind = ((true as u32) & Self::KIND_BIT_MASK) << Self::KIND_SHIFT;
                let state = ((action_state as u32) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
                let time = ((play_elapsed_time_ms) & Self::TIME_BIT_MASK) << Self::TIME_SHIFT;
                let bits = kind | state | time;

                let mut bytes = Vec::with_capacity(Self::byte_size());
                bytes.extend_from_slice(&bits.to_big_endian_bytes());
                bytes
            }
            StateChangeEvent::MovementState {
                movement_state,
                play_elapsed_time_ms,
            } => {
                let kind = ((false as u32) & Self::KIND_BIT_MASK) << Self::KIND_SHIFT;
                let state = ((movement_state as u32) & Self::STATE_BIT_MASK) << Self::STATE_SHIFT;
                let time = ((play_elapsed_time_ms) & Self::TIME_BIT_MASK) << Self::TIME_SHIFT;
                let bits = kind | state | time;

                let mut bytes = Vec::with_capacity(Self::byte_size());
                bytes.extend_from_slice(&bits.to_big_endian_bytes());
                bytes
            }
        }
    }
}

impl TryFromBigEndian for StateChangeEvent {
    #[allow(unused_mut)]
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(StateChangeEvent)
            )
        };

        let mut offset = 0;
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let bits = u32::from_big_endian_bytes(data);
        let kind = (bits >> Self::KIND_SHIFT) & Self::KIND_BIT_MASK == Self::KIND_BIT_MASK;
        if kind {
            let val = ((bits >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK) as u8;
            let action_state = ActionState::new(val)?;
            let play_elapsed_time_ms = (bits >> Self::TIME_SHIFT) & Self::TIME_BIT_MASK;
            Some(Self::ActionState {
                action_state,
                play_elapsed_time_ms,
            })
        } else {
            let val = ((bits >> Self::STATE_SHIFT) & Self::STATE_BIT_MASK) as u8;
            let movement_state = MovementState::new(val)?;
            let play_elapsed_time_ms = (bits >> Self::TIME_SHIFT) & Self::TIME_BIT_MASK;
            Some(Self::MovementState {
                movement_state,
                play_elapsed_time_ms,
            })
        }
    }
}

/// 인게임 플레이어 갱신 데이터입니다.
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePlayerPullData {
    /// 사용자 식별자
    pub uid: UserId,

    /// 상대 팀 처치 횟수
    pub kill_count: u16,
    /// 상태 팀에게 처치 당한 횟수
    pub dead_count: u16,

    /// 현재 방어막 체력
    pub shield_health: u16,
    /// 현재 체력
    pub current_health: u16,
    /// 현재 남은 총알 수
    pub current_bullet: u16,
    /// 현재 스킬 코스트
    pub current_skill_cost: u16,

    /// 월드 공간 위치
    pub translation: [f32; 3],
    /// 월드 공간 방향
    pub rotation: [f32; 4],
    /// 월드 공간 속도
    pub velocity: [f32; 3],
    /// 월드 공간 이동 방향
    pub direction: [f32; 3],

    /// 비트 필드 데이터
    bitfield: Bitfield,
    /// 플레이어 상태 데이터
    player_states: PlayerStateData,
    /// 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 카메라 시점 각도
    pub latlon: LatLon,
}

impl InGamePlayerPullData {
    /// 새로운 플레이어 데이터를 생성합니다.
    pub const fn new(
        uid: UserId,
        kill_count: u16,
        dead_count: u16,
        shield_health: u16,
        current_health: u16,
        current_bullet: u16,
        current_skill_cost: u16,
        translation: [f32; 3],
        rotation: [f32; 4],
        velocity: [f32; 3],
        direction: [f32; 3],
        permission: Permission,
        connected: bool,
        grounded: bool,
        invincible: bool,
        network_state: NetworkState,
        player_states: PlayerStateData,
        action_state_timer: ActionStateTimer,
        movement_state_timer: MovementStateTimer,
        latlon: LatLon,
    ) -> Self {
        Self {
            uid,
            kill_count,
            dead_count,
            shield_health,
            current_health,
            current_bullet,
            current_skill_cost,
            translation,
            rotation,
            velocity,
            direction,
            bitfield: Bitfield::new()
                .with_permission(permission)
                .with_connected(connected)
                .with_grounded(grounded)
                .with_invincible(invincible)
                .with_network_state(network_state),
            player_states,
            action_state_timer,
            movement_state_timer,
            latlon,
        }
    }

    /// 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    pub fn is_grounded(&self) -> bool {
        self.bitfield.is_grounded()
    }

    /// 무적 여부를 반환합니다.
    pub fn is_invincible(&self) -> bool {
        self.bitfield.is_invincible()
    }

    /// 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }

    /// 행동 상태를 반환합니다.
    pub fn action_state(&self) -> ActionState {
        self.player_states.action_state()
    }

    /// 움직임 상태를 반환합니다.
    pub fn movement_state(&self) -> MovementState {
        self.player_states.movement_state()
    }
}

impl BigEndian for InGamePlayerPullData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 3]>::byte_size()
            + Bitfield::byte_size()
            + PlayerStateData::byte_size()
            + ActionStateTimer::byte_size()
            + MovementStateTimer::byte_size()
            + LatLon::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerInitData)
            )
        };

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 상대 팀 처치 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 처치 당한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let dead_count = u16::from_big_endian_bytes(data);

        // 현재 방어막 체력을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let guard_health = u16::from_big_endian_bytes(data);

        // 현재 체력을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let current_health = u16::from_big_endian_bytes(data);

        // 현재 남은 총알 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let current_bullet = u16::from_big_endian_bytes(data);

        // 현재 스킬 코스트를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let current_skill_cost = u16::from_big_endian_bytes(data);

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

        // 월드 공간 속도를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let velocity = <[f32; 3]>::from_big_endian_bytes(data);

        // 월드 공간 이동 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let direction = <[f32; 3]>::from_big_endian_bytes(data);

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        // 플레이어 상태 데이터를 가져옵니다.
        offset = offset + size;
        size = PlayerStateData::byte_size();
        data = &bytes[offset..offset + size];
        let states = PlayerStateData::from_big_endian_bytes(data);

        // 행동 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ActionStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let action_state_timer = ActionStateTimer::from_big_endian_bytes(data);

        // 움직임 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = MovementStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state_timer = MovementStateTimer::from_big_endian_bytes(data);

        // 카메라 시점을 가져옵니다.
        offset = offset + size;
        size = LatLon::byte_size();
        data = &bytes[offset..offset + size];
        let latlon = LatLon::from_big_endian_bytes(data);

        Self {
            uid,
            kill_count,
            dead_count,
            shield_health: guard_health,
            current_health,
            current_bullet,
            current_skill_cost,
            translation,
            rotation,
            velocity,
            direction,
            bitfield,
            player_states: states,
            action_state_timer,
            movement_state_timer,
            latlon,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.dead_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.shield_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_bullet.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_skill_cost.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.direction.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());
        bytes.extend_from_slice(&self.player_states.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.latlon.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerPullData)
            );
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_connected() {
        let bitfield = Bitfield::new().with_connected(false);
        assert_eq!(false, bitfield.is_connected());

        let bitfield = Bitfield::new().with_connected(true);
        assert_eq!(true, bitfield.is_connected());
    }

    #[test]
    fn test_bitfield_invincible() {
        let bitfield = Bitfield::new().with_invincible(false);
        assert_eq!(false, bitfield.is_invincible());

        let bitfield = Bitfield::new().with_invincible(true);
        assert_eq!(true, bitfield.is_invincible());
    }

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
    fn test_bitfield_grounded() {
        let bitfield = Bitfield::new().with_grounded(false);
        assert_eq!(false, bitfield.is_grounded());

        let bitfield = Bitfield::new().with_grounded(true);
        assert_eq!(true, bitfield.is_grounded());
    }

    #[test]
    fn test_bitfield_network_state() {
        let state = NetworkState::Critical;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Critical, bitfield.network_state());

        let state = NetworkState::Poor;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Poor, bitfield.network_state());

        let state = NetworkState::Fair;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Fair, bitfield.network_state());

        let state = NetworkState::Good;
        let bitfield = Bitfield::new().with_network_state(state);
        assert_eq!(NetworkState::Good, bitfield.network_state());
    }
}
