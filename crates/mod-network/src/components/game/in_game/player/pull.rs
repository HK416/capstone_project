//! 인게임 단계에서 플레이어 데이터 갱신과 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionStateTimer, BigEndian, MovementStateTimer, NetworkState, Permission, PlayerStateData,
    UserId,
};

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - connected     | 1bit | 서버 연결 여부
/// - invincible    | 1bit | 무적 여부
/// - permission    | 1bit | 권한
/// - overwrite     | 1bit | 덮어쓰기 여부
/// - network_state | 2bit | 네트워크 상태
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u8);

impl Bitfield {
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 0;
    const INVINCIBLE_BIT_MASK: u8 = 0x01;
    const INVINCIBLE_SHIFT: usize = 1;
    const PERMISSION_BIT_MASK: u8 = 0x01;
    const PERMISSION_SHIFT: usize = 2;
    const OVERWRITE_BIT_MASK: u8 = 0x01;
    const OVERWRITE_SHIFT: usize = 3;
    const STATE_BIT_MASK: u8 = 0x03;
    const STATE_SHIFT: usize = 4;

    /// 새로운 비트 필드 데이터를 생성합니다.
    const fn new() -> Self {
        Self(0x00)
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

    /// 무적 여부를 반환합니다.
    fn is_invincible(&self) -> bool {
        (self.0 >> Self::INVINCIBLE_SHIFT) & Self::INVINCIBLE_BIT_MASK == Self::INVINCIBLE_BIT_MASK
    }

    /// 무적 여부를 설정합니다.
    fn with_invincible(mut self, invincible: bool) -> Self {
        self.0 &= !(Self::INVINCIBLE_BIT_MASK << Self::INVINCIBLE_SHIFT);
        self.0 |= ((invincible as u8) & Self::INVINCIBLE_BIT_MASK) << Self::INVINCIBLE_SHIFT;
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

    /// 덮어쓰기 여부를 반환합니다.
    fn is_overwrite(&self) -> bool {
        (self.0 >> Self::OVERWRITE_SHIFT) & Self::OVERWRITE_BIT_MASK == Self::OVERWRITE_BIT_MASK
    }

    /// 덮어쓰기 여부를 설정합니다.
    fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.0 &= !(Self::OVERWRITE_BIT_MASK << Self::OVERWRITE_SHIFT);
        self.0 |= ((overwrite as u8) & Self::OVERWRITE_BIT_MASK) << Self::OVERWRITE_SHIFT;
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
    pub guard_health: u16,
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

    /// 비트 필드 데이터
    bitfield: Bitfield,
    /// 플레이어 상태 데이터
    pub player_states: PlayerStateData,
    /// 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
}

impl InGamePlayerPullData {
    pub fn new(
        uid: UserId,
        kill_count: u16,
        dead_count: u16,
        guard_health: u16,
        current_health: u16,
        current_bullet: u16,
        current_skill_cost: u16,
        translation: [f32; 3],
        rotation: [f32; 4],
        velocity: [f32; 3],
        connected: bool,
        invincible: bool,
        permission: Permission,
        overwrite: bool,
        network_state: NetworkState,
        player_states: PlayerStateData,
        action_state_timer: ActionStateTimer,
        movement_state_timer: MovementStateTimer,
    ) -> Self {
        Self {
            uid,
            kill_count,
            dead_count,
            guard_health,
            current_health,
            current_bullet,
            current_skill_cost,
            translation,
            rotation,
            velocity,
            bitfield: Bitfield::new()
                .with_connected(connected)
                .with_invincible(invincible)
                .with_permission(permission)
                .with_overwrite(overwrite)
                .with_network_state(network_state),
            player_states,
            action_state_timer,
            movement_state_timer,
        }
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
        self.bitfield = self.bitfield.with_connected(connected);
        self
    }

    /// 무적 여부를 반환합니다.
    pub fn is_invincible(&self) -> bool {
        self.bitfield.is_invincible()
    }

    /// 무적 여부를 설정합니다.
    pub fn set_invincible(&mut self, invincible: bool) {
        self.bitfield = self.bitfield.with_invincible(invincible);
    }

    /// 무적 여부를 설정합니다.
    pub fn with_invincible(mut self, invincible: bool) -> Self {
        self.bitfield = self.bitfield.with_invincible(invincible);
        self
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

    /// 데이터 덮어쓰기 여부를 반환합니다.
    pub fn is_overwrite(&self) -> bool {
        self.bitfield.is_overwrite()
    }

    /// 데이터 덮에쓰기 여부를 설정합니다.
    pub fn set_overwrite(&mut self, overwrite: bool) {
        self.bitfield = self.bitfield.with_overwrite(overwrite);
    }

    /// 데이터 덮어쓰기 여부를 설정합니다.
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.bitfield = self.bitfield.with_overwrite(overwrite);
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
        self.bitfield = self.bitfield.with_network_state(state);
        self
    }
}

impl BigEndian for InGamePlayerPullData {
    fn byte_size() -> usize {
        UserId::byte_size()    // 4byte
            + u16::byte_size()    // 6byte
            + u16::byte_size()    // 8byte
            + u16::byte_size()    // 10byte
            + u16::byte_size()    // 12byte
            + u16::byte_size()    // 14byte
            + u16::byte_size()    // 16byte
            + <[f32; 3]>::byte_size()    // 28byte
            + <[f32; 4]>::byte_size()    // 44byte
            + <[f32; 3]>::byte_size()    // 56byte
            + Bitfield::byte_size()    // 57byte
            + PlayerStateData::byte_size()    // 58byte
            + ActionStateTimer::byte_size()    // 60byte
            + MovementStateTimer::byte_size() // 62byte
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

        Self {
            uid,
            kill_count,
            dead_count,
            guard_health,
            current_health,
            current_bullet,
            current_skill_cost,
            translation,
            rotation,
            velocity,
            bitfield,
            player_states: states,
            action_state_timer,
            movement_state_timer,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.dead_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.guard_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_bullet.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_skill_cost.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());
        bytes.extend_from_slice(&self.player_states.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());

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
    fn test_bitfield_overwrite() {
        let bitfield = Bitfield::new().with_overwrite(false);
        assert_eq!(false, bitfield.is_overwrite());

        let bitfield = Bitfield::new().with_overwrite(true);
        assert_eq!(true, bitfield.is_overwrite());
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
