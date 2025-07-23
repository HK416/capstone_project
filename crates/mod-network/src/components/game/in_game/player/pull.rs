//! 인게임 단계에서 플레이어 데이터 갱신과 관련된 코드를 관리합니다.
//!

use std::{f32::consts::TAU, i16, num::NonZeroU32};

use crate::components::{
    ActionNotify, ActionState, ActionStateTimer, BigEndian, CharacterAttributes, LatLon,
    MovementState, MovementStateTimer, NetworkState, Permission, PlayerStateData, UserId,
    MAX_JUMP_DURATION, MAX_LATITUDE, MIN_LATITUDE, RESPAWN_DELAY,
};

/// 서버에서 클라이언트로 전달되는 플레이어 갱신 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGamePlayerPullData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 월드 공간 위치
    translation: [i16; 3],
    /// 월드 공간 방향
    rotation: [i16; 4],
    /// 플레이어 상태 데이터
    player_state: PlayerStateData,
    /// 행동 상태 타이머
    action_state_timer: u8,
    /// 움직임 상태 타이머
    movement_state_timer: u8,
    /// 카메라 위도
    latitude: i16,
    /// 카메라 경도
    longitude: i16,
}

impl InGamePlayerPullData {
    ///  새로운 `InGamePlayerPullData`를 생성합니다
    pub fn new(
        uid: UserId,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
        translation: glam::Vec3A,
        rotation: glam::Quat,
        action_state: ActionState,
        action_notify: ActionNotify,
        action_state_timer: ActionStateTimer,
        movement_state: MovementState,
        movement_state_timer: MovementStateTimer,
        attributes: &CharacterAttributes,
        latlon: LatLon,
    ) -> Self {
        let hx = half_size_x.get() as f32;
        let hy = half_size_y.get() as f32;
        let hz = half_size_z.get() as f32;
        let x = translation.x.clamp(-hx, hx) / hx * i16::MAX as f32;
        let y = translation.y.clamp(-hy, hy) / hy * i16::MAX as f32;
        let z = translation.z.clamp(-hz, hz) / hz * i16::MAX as f32;
        let translation = [x as i16, y as i16, z as i16];

        let rotation = rotation.normalize();
        let x = rotation.x * i16::MAX as f32;
        let y = rotation.y * i16::MAX as f32;
        let z = rotation.z * i16::MAX as f32;
        let w = rotation.w * i16::MAX as f32;
        let rotation = [x as i16, y as i16, z as i16, w as i16];

        let duration = match action_state {
            ActionState::Idle => attributes.normal_idle_duration,
            ActionState::Aiming => attributes.normal_idle_duration,
            ActionState::AimAt => attributes.normal_attack_start_duration,
            ActionState::AimOff => attributes.normal_attack_end_duration,
            ActionState::Attack => attributes.normal_attack_ing_duration,
            ActionState::Retreat => RESPAWN_DELAY,
            ActionState::Reload => attributes.normal_reload_duration,
            ActionState::Skill => attributes.skill_duration,
            ActionState::Callsign => attributes.normal_callsign_duration,
            ActionState::VictoryStart => attributes.victory_start_duration,
            ActionState::VictoryEnd => attributes.victory_end_duration,
        };
        let action_state_timer =
            (action_state_timer.0.min(duration) as f32 / duration as f32 * u8::MAX as f32) as u8;

        let duration = match movement_state {
            MovementState::Idle => attributes.normal_attack_ing_duration,
            MovementState::Moving => attributes.move_ing_duration,
            MovementState::MoveToEnd => attributes.move_end_normal_duration,
            MovementState::Jumping => MAX_JUMP_DURATION,
            MovementState::Landing => MAX_JUMP_DURATION,
        };
        let movement_state_timer =
            (movement_state_timer.0.min(duration) as f32 / duration as f32 * u8::MAX as f32) as u8;

        let latitude = latlon.lat.clamp(MIN_LATITUDE, MAX_LATITUDE) / MAX_LATITUDE;
        let latitude = (latitude * i16::MAX as f32) as i16;

        let longitude = (latlon.lon % TAU) / TAU;
        let longitude = (longitude * i16::MAX as f32) as i16;

        Self {
            uid,
            translation,
            rotation,
            player_state: PlayerStateData::new()
                .with_action_state(action_state)
                .with_movement_state(movement_state)
                .with_action_notify(action_notify),
            action_state_timer,
            movement_state_timer,
            latitude,
            longitude,
        }
    }

    /// 플레이어의 월드 공간 위치를 반환합니다.
    pub fn trasnaltion(
        &self,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
    ) -> glam::Vec3A {
        let x = self.translation[0] as f32 / i16::MAX as f32;
        let y = self.translation[1] as f32 / i16::MAX as f32;
        let z = self.translation[2] as f32 / i16::MAX as f32;
        let translation = glam::vec3a(x, y, z);
        let half_size = glam::vec3a(
            half_size_x.get() as f32,
            half_size_y.get() as f32,
            half_size_z.get() as f32,
        );
        translation * half_size
    }

    /// 플레이어의 월드 공간 방향을 반환합니다.
    pub fn rotation(&self) -> glam::Quat {
        let x = self.rotation[0] as f32 / i16::MAX as f32;
        let y = self.rotation[1] as f32 / i16::MAX as f32;
        let z = self.rotation[2] as f32 / i16::MAX as f32;
        let w = self.rotation[3] as f32 / i16::MAX as f32;
        let rotation = glam::quat(x, y, z, w);
        rotation.normalize()
    }

    /// 플레이어의 행동 상태를 반환합니다.
    pub fn action_state(&self) -> ActionState {
        self.player_state.action_state()
    }

    /// 플레이어의 움직임 상태를 반환합니다.
    pub fn movement_state(&self) -> MovementState {
        self.player_state.movement_state()
    }

    /// 행동 상태 알림을 반환합니다.
    pub fn action_notify(&self) -> ActionNotify {
        self.player_state.action_notify()
    }

    /// 플레이어의 행동 상태 타이머를 반환합니다.
    pub fn action_state_timer(&self, attribute: &CharacterAttributes) -> ActionStateTimer {
        let duration = match self.action_state() {
            ActionState::Idle => attribute.normal_idle_duration,
            ActionState::Aiming => attribute.normal_idle_duration,
            ActionState::AimAt => attribute.normal_attack_start_duration,
            ActionState::AimOff => attribute.normal_attack_end_duration,
            ActionState::Attack => attribute.normal_attack_ing_duration,
            ActionState::Retreat => RESPAWN_DELAY,
            ActionState::Reload => attribute.normal_reload_duration,
            ActionState::Skill => attribute.skill_duration,
            ActionState::Callsign => attribute.normal_callsign_duration,
            ActionState::VictoryStart => attribute.victory_start_duration,
            ActionState::VictoryEnd => attribute.victory_end_duration,
        };
        let t = self.action_state_timer as f32 / u8::MAX as f32;
        let time = (duration as f32 * t).round() as u16;
        ActionStateTimer(time)
    }

    /// 플레이어 움직임 상태 타이머를 반환합니다.
    pub fn movement_state_timer(&self, attribute: &CharacterAttributes) -> MovementStateTimer {
        let duration = match self.movement_state() {
            MovementState::Idle => attribute.normal_idle_duration,
            MovementState::Moving => attribute.move_ing_duration,
            MovementState::MoveToEnd => attribute.move_end_normal_duration,
            MovementState::Jumping => MAX_JUMP_DURATION,
            MovementState::Landing => MAX_JUMP_DURATION,
        };
        let t = self.movement_state_timer as f32 / u8::MAX as f32;
        let time = (duration as f32 * t).round() as u16;
        MovementStateTimer(time)
    }

    /// 플레이어 카메라 방향을 반환합니다.
    pub fn latlon(&self) -> LatLon {
        let lat = self.latitude as f32 / i16::MAX as f32;
        let lat = lat * MAX_LATITUDE;
        let lon = self.longitude as f32 / i16::MAX as f32;
        let lon = lon * TAU;
        LatLon { lat, lon }
    }
}

impl BigEndian for InGamePlayerPullData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + <[i16; 3]>::byte_size()
            + <[i16; 4]>::byte_size()
            + PlayerStateData::byte_size()
            + u8::byte_size()
            + u8::byte_size()
            + i16::byte_size()
            + i16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerPullData),
            )
        };

        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        offset = offset + size;
        size = <[i16; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[i16; 3]>::from_big_endian_bytes(data);

        offset = offset + size;
        size = <[i16; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[i16; 4]>::from_big_endian_bytes(data);

        offset = offset + size;
        size = PlayerStateData::byte_size();
        data = &bytes[offset..offset + size];
        let player_state = PlayerStateData::from_big_endian_bytes(data);

        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let action_state_timer = u8::from_big_endian_bytes(data);

        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state_timer = u8::from_big_endian_bytes(data);

        offset = offset + size;
        size = i16::byte_size();
        data = &bytes[offset..offset + size];
        let latitude = i16::from_big_endian_bytes(data);

        offset = offset + size;
        size = i16::byte_size();
        data = &bytes[offset..offset + size];
        let longitude = i16::from_big_endian_bytes(data);

        Self {
            uid,
            translation,
            rotation,
            player_state,
            action_state_timer,
            movement_state_timer,
            latitude,
            longitude,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.player_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.latitude.to_big_endian_bytes());
        bytes.extend_from_slice(&self.longitude.to_big_endian_bytes());

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

/// 플레이어 비트 필드 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - permission    | 1bit | 권한
/// - connected     | 1bit | 서버 접속 여부
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
    const STATE_BIT_MASK: u8 = 0x03;
    const STATE_SHIFT: usize = 3;

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

/// 서버에서 클라이언트로 전달되는 플레이어 상태 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGamePlayerStatusPullData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 상대 팀 처치 횟수
    pub kill_count: u16,
    /// 상대 팀에게 처치 당한 횟수
    pub retreat_count: u16,
    /// 현재 방어막 체력
    pub shield_health: u16,
    /// 현재 남은 체력
    pub remaining_health: u16,
    /// 현재 남은 총알 수
    pub remaining_bullet: u16,
    /// 현재 스킬 코스트
    pub current_skill_cost: u16,
    /// 비트 필드 데이터
    bitfield: Bitfield,
}

impl InGamePlayerStatusPullData {
    pub const fn new(
        uid: UserId,
        kill_count: u16,
        retreat_count: u16,
        shield_health: u16,
        remaining_health: u16,
        remaining_bullet: u16,
        current_skill_cost: u16,
        permission: Permission,
        connected: bool,
        invincible: bool,
        state: NetworkState,
    ) -> Self {
        Self {
            uid,
            kill_count,
            retreat_count,
            shield_health,
            remaining_health,
            remaining_bullet,
            current_skill_cost,
            bitfield: Bitfield::new()
                .with_permission(permission)
                .with_connected(connected)
                .with_invincible(invincible)
                .with_network_state(state),
        }
    }

    /// 플레이어 권한을 반환합니다.
    pub fn permission(&self) -> Permission {
        self.bitfield.permission()
    }

    /// 플레이어 서버 접속 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.bitfield.is_connected()
    }

    /// 플레이어 무적 여부를 반환합니다.
    pub fn is_invincible(&self) -> bool {
        self.bitfield.is_invincible()
    }

    /// 플레이어 네트워크 상태를 반환합니다.
    pub fn network_state(&self) -> NetworkState {
        self.bitfield.network_state()
    }
}

impl BigEndian for InGamePlayerStatusPullData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerStatusPullData)
            )
        };

        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let retreat_count = u16::from_big_endian_bytes(data);

        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let shield_health = u16::from_big_endian_bytes(data);

        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let remaining_health = u16::from_big_endian_bytes(data);

        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let remaining_bullet = u16::from_big_endian_bytes(data);

        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let current_skill_cost = u16::from_big_endian_bytes(data);

        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        Self {
            uid,
            kill_count,
            retreat_count,
            shield_health,
            remaining_health,
            remaining_bullet,
            current_skill_cost,
            bitfield,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.retreat_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.shield_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining_health.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining_bullet.to_big_endian_bytes());
        bytes.extend_from_slice(&self.current_skill_cost.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerStatusPullData)
            );
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use mod_physics::object3d::Capsule;

    use crate::components::Float3;

    use super::*;

    #[test]
    fn test_in_game_player_pull_data() {
        let attributes = CharacterAttributes {
            speed: 5.0,
            left_weapon: None,
            right_weapon: None,
            attack_head_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            attack_spine_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            attack_spine1_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            skill_head_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            skill_spine_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            skill_spine1_axis: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            camera_def_fov_y: 0.0,
            camera_def_rel_pos: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            camera_zoom_fov_y: 0.0,
            camera_zoom_rel_pos: Float3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            normal_idle_duration: 1200,
            cafe_walk_duration: 0,
            move_ing_duration: 0,
            move_end_normal_duration: 0,
            normal_attack_start_duration: 0,
            normal_attack_end_duration: 0,
            normal_attack_ing_duration: 0,
            vital_death_duration: 0,
            normal_reload_duration: 0,
            skill_duration: 0,
            skill_timing: vec![],
            normal_callsign_duration: 0,
            victory_start_duration: 0,
            victory_end_duration: 0,
            normal_attack_timing: vec![],
            normal_attack_count: 0,
            max_bullets: 0,
            max_health_point: 0,
            attack_power: 0,
            defense_power: 0,
            accuracy_stat: 0,
            evasion_stat: 0,
            critical_rate: 0,
            critical_damage: 0,
            max_skill_cost: 0,
            skill_cost: 0,
            attack_range: 0,
            bullet_radius: 0.0,
            collider: Capsule {
                center: glam::vec3(0.0, 0.0, 0.0),
                height: 0.0,
                radius: 0.0,
            },
        };
        let origin = InGamePlayerPullData::new(
            UserId::new(589141),
            NonZero::new(25).unwrap(),
            NonZero::new(25).unwrap(),
            NonZero::new(25).unwrap(),
            glam::vec3a(-1.4123, 1.3422, 20.411),
            glam::quat(0.0, 0.7071068, 0.0, 0.7071068),
            ActionState::Idle,
            ActionNotify::EnterAttack,
            ActionStateTimer::new(132),
            MovementState::Idle,
            MovementStateTimer(132),
            &attributes,
            LatLon::new(-3f32.to_radians(), 52.134f32.to_radians()),
        );
        let bytes = origin.to_big_endian_bytes();
        let other = InGamePlayerPullData::from_big_endian_bytes(&bytes);

        // 원본가 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    fn test_in_game_player_status_pull_data() {
        let origin = InGamePlayerStatusPullData::new(
            UserId::new(43141),
            12,
            8,
            92,
            6143,
            8,
            721,
            Permission::Admin,
            true,
            false,
            NetworkState::Good,
        );
        let bytes = origin.to_big_endian_bytes();
        let other = InGamePlayerStatusPullData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
