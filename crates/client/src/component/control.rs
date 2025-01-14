use mod_network::components::{
    ActionState, MovementState, MovementStateTimer, ViewState, ViewStateTimer,
};
use winit::{
    event::MouseButton,
    keyboard::{KeyCode, KeyLocation},
};

use crate::config::UserConfig;

use super::ThirdPersonCamera;

/// 마우스 버튼 입력 이벤트가 발생했을 때, `ActionState`를 갱신하는 함수입니다.
pub fn update_action_state_on_mouse_click(
    config: &UserConfig,
    button: MouseButton,
    action_state: &mut ActionState,
) {
    // TODO:
}

/// 키보드 입력 이벤트가 발생했을 때, `ActionState`를 갱신하는 함수입니다.
pub fn update_action_state_on_keyboard_pressed(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
    action_state: &mut ActionState,
) {
    // TODO:
}

/// 컨트롤러 입력이 지속된 시간을 측정하는 타이머입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ControllerInputTimer(pub f32);

impl ControllerInputTimer {
    /// 타이머가 가질 수 있는 최소 시간입니다.
    pub const MIN_TIME: f32 = 0.0;

    /// 타이머가 가질 수 있는 최대 시간입니다.
    pub const MAX_TIME: f32 = 0.3;

    /// 컨트롤러가 눌려있을 때 타이머를 갱신하는 함수입니다.
    pub fn update_when_controller_preesed(&mut self, fixed_time_sec: f32) {
        self.0 = (self.0 + fixed_time_sec).min(Self::MAX_TIME)
    }

    /// 컨트롤러가 눌려있지 않을 때 타이머를 갱신하는 함수입니다.
    pub fn update_when_controller_released(&mut self, fixed_time_sec: f32) {
        self.0 = (self.0 - fixed_time_sec).max(Self::MAX_TIME)
    }

    /// 타이머의 값을 0에서 1사이의 값을 반환합니다.
    pub fn normalize(&self) -> f32 {
        self.0 / Self::MAX_TIME
    }
}

impl Default for ControllerInputTimer {
    fn default() -> Self {
        Self(Self::MIN_TIME)
    }
}

/// 플레이어 방향 컨트롤러의 상태를 나타냅니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControllerState {
    Idle = 0,
    MovingLeft = 1,
    MovingRight = 2,
    MovingForward = 3,
    MovingBackward = 4,
    MovingLeftForward = 5,
    MovingRightForward = 6,
    MovingLeftBackward = 7,
    MovingRightBackward = 8,
}

impl Default for ControllerState {
    fn default() -> Self {
        ControllerState::Idle
    }
}

impl ControllerState {
    /// 키보드 입력이 발생했을 때 `ControllerState`를 변경합니다.
    pub fn handle_keyboard_pressed(
        &mut self,
        config: &UserConfig,
        keycode: KeyCode,
        location: KeyLocation,
    ) {
        const FUNC_TABLE: [fn(&UserConfig, KeyCode, KeyLocation) -> ControllerState; 9] = [
            handle_keyboard_pressed_idle_state,
            handle_keyboard_pressed_moving_left_state,
            handle_keyboard_pressed_moving_right_state,
            handle_keyboard_pressed_moving_forward_state,
            handle_keyboard_pressed_moving_backward_state,
            handle_keyboard_pressed_moving_left_forward_state,
            handle_keyboard_pressed_moving_right_forward_state,
            handle_keyboard_pressed_moving_left_backward_state,
            handle_keyboard_pressed_moving_right_backward_state,
        ];
        let index = *self as usize;
        *self = FUNC_TABLE[index](config, keycode, location);
    }

    /// 키보드 입력이 해제되었을 때 `ControllerState`를 변경합니다.   
    pub fn handle_keyboard_released(
        &mut self,
        config: &UserConfig,
        keycode: KeyCode,
        location: KeyLocation,
    ) {
        const FUNC_TABLE: [fn(&UserConfig, KeyCode, KeyLocation) -> ControllerState; 9] = [
            handle_keyboard_released_idle_state,
            handle_keyboard_released_moving_left_state,
            handle_keyboard_released_moving_right_state,
            handle_keyboard_released_moving_forward_state,
            handle_keyboard_released_moving_backward_state,
            handle_keyboard_released_moving_left_forward_state,
            handle_keyboard_released_moving_right_forward_state,
            handle_keyboard_released_moving_left_backward_state,
            handle_keyboard_released_moving_right_backward_state,
        ];
        let index = *self as usize;
        *self = FUNC_TABLE[index](config, keycode, location);
    }
}

/// `ControllerState::Idle` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_idle_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::MovingLeft
    } else if config.keyboard.right == (keycode, location) {
        ControllerState::MovingRight
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingForward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingBackward
    } else {
        ControllerState::Idle
    }
}

/// `ControllerState::Idle` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_idle_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::MovingRight
    } else if config.keyboard.right == (keycode, location) {
        ControllerState::MovingLeft
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingForward
    } else {
        ControllerState::Idle
    }
}

/// `ControllerState::MovingLeft` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_left_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.right == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingLeftForward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingLeftBackward
    } else {
        ControllerState::MovingLeft
    }
}

/// `ControllerState::MovingLeft` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_left_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingForward
    } else {
        ControllerState::MovingLeft
    }
}

/// `ControllerState::MovingRight` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_right_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingRightForward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingRightBackward
    } else {
        ControllerState::MovingRight
    }
}

/// `ControllerState::MovingRight` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_right_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.right == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingForward
    } else {
        ControllerState::MovingRight
    }
}

/// `ControllerState::MovingForward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.backward == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.left == (keycode, location) {
        ControllerState::MovingLeftForward
    } else if config.keyboard.right == (keycode, location) {
        ControllerState::MovingRightForward
    } else {
        ControllerState::MovingForward
    }
}

/// `ControllerState::MovingForward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.forward == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.left == (keycode, location) {
        ControllerState::MovingRightForward
    } else if config.keyboard.right == (keycode, location) {
        ControllerState::MovingLeftForward
    } else {
        ControllerState::MovingForward
    }
}

/// `ControllerState::MovingBackward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.forward == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.left == (keycode, location) {
        ControllerState::MovingLeftBackward
    } else if config.keyboard.right == (keycode, location) {
        ControllerState::MovingRightBackward
    } else {
        ControllerState::MovingBackward
    }
}

/// `ControllerState::MovingBackward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.backward == (keycode, location) {
        ControllerState::Idle
    } else if config.keyboard.left == (keycode, location) {
        ControllerState::MovingRightBackward
    } else if config.keyboard.right == (keycode, location) {
        ControllerState::MovingLeftBackward
    } else {
        ControllerState::MovingBackward
    }
}

/// `ControllerState::MovingLeftForward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_left_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.right == (keycode, location) {
        ControllerState::MovingForward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingLeft
    } else {
        ControllerState::MovingLeftForward
    }
}

/// `ControllerState::MovingLeftForward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_left_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::MovingForward
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingLeft
    } else {
        ControllerState::MovingLeftForward
    }
}

/// `ControllerState::MovingRightForward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_right_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::MovingForward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingRight
    } else {
        ControllerState::MovingRightForward
    }
}

/// `ControllerState::MovingRightForward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_right_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.right == (keycode, location) {
        ControllerState::MovingForward
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingRight
    } else {
        ControllerState::MovingRightForward
    }
}

/// `ControllerState::MovingLeftBackward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_left_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.right == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingLeft
    } else {
        ControllerState::MovingLeftBackward
    }
}

/// `ControllerState::MovingLeftBackward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_left_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingLeft
    } else {
        ControllerState::MovingLeftBackward
    }
}

/// `ControllerState::MovingRightBackward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_right_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.left == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.forward == (keycode, location) {
        ControllerState::MovingRight
    } else {
        ControllerState::MovingRightBackward
    }
}

/// `ControllerState::MovingRightBackward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_right_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> ControllerState {
    if config.keyboard.right == (keycode, location) {
        ControllerState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        ControllerState::MovingRight
    } else {
        ControllerState::MovingRightBackward
    }
}

/// 플레이어가 이동하고자 하는 방향을 나타냅니다. (캐릭터가 바라보는 방향과 다를 수 있습니다)
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveDirection(pub glam::Vec4);

impl MoveDirection {
    /// 삼인칭 카메라 시점을 기준으로 컨트롤러 상태에 따라 플레이어 이동 방향을 갱신합니다.
    pub fn update_from_third_person_camera(
        &mut self,
        controller: ControllerState,
        third_person_camera: &ThirdPersonCamera,
    ) {
        const FUNC_TABLE: [fn(glam::Vec4, glam::Vec4, &mut MoveDirection); 9] = [
            update_move_direction_when_idle_state,
            update_move_direction_when_moving_left_state,
            update_move_direction_when_moving_right_state,
            update_move_direction_when_moving_forward_state,
            update_move_direction_when_moving_backward_state,
            update_move_direction_when_moving_left_forward_state,
            update_move_direction_when_moving_right_forward_state,
            update_move_direction_when_moving_left_backward_state,
            update_move_direction_when_moving_right_backward_state,
        ];

        // 카메라가 바라보는 방향을 계산합니다.
        let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
        let view_right = mat.x_axis.normalize_or(glam::Vec4::X);
        let view_forward = mat.z_axis.normalize_or(glam::Vec4::Z);

        let index = controller as usize;
        FUNC_TABLE[index](view_right, view_forward, self);
    }
}

impl Default for MoveDirection {
    fn default() -> Self {
        Self(glam::Vec4::Z)
    }
}

/// `ControllerState::Idle`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_idle_state(
    _view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    _direction: &mut MoveDirection,
) {
    /* empty */
}

/// `ControllerState::MovingLeft`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_left_state(
    view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = -view_right;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingRight`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_right_state(
    view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = view_right;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_forward_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = view_forward;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_backward_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = -view_forward;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingLeftForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_left_forward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right + SQRT_2 * view_forward;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingRightForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_right_forward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right + SQRT_2 * view_forward;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingLeftBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_left_backward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right - SQRT_2 * view_forward;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingRightBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_right_backward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right - SQRT_2 * view_forward;
    debug_assert!(
        dir.is_normalized(),
        "the given vector must be a unit vector."
    );

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = glam::Vec3A::from_vec4(dir);
    let src_v = glam::Vec3A::from_vec4(direction.0);
    let s = dst_v.angle_between(src_v) / PI;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState`에 따라 `MovementState`를 갱신하는 함수입니다.
pub fn update_movement_state_by_controller_state(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    controller: ControllerState,
) {
    const STATE_TABLE: [[MovementState; 9]; 3] = [
        // `MovementState::Idle`일 때
        [
            MovementState::Idle,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
        ],
        // `MovementState::Moving`일 떄
        [
            MovementState::MoveToEnd,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
        ],
        // `MovementState::MoveToEnd`일 떄
        [
            MovementState::MoveToEnd,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
            MovementState::Moving,
        ],
    ];

    let previous_state = movement_state.clone();

    // `MovementState`를 갱신합니다.
    let i = *movement_state as usize;
    let j = controller as usize;
    *movement_state = STATE_TABLE[i][j];

    // `MovementStateTimer`를 갱신합니다.
    if previous_state != *movement_state {
        movement_state_timer.reset();
    }
}

/// `ViewStateTimer`를 갱신하는 함수입니다.
pub fn update_view_state_timer(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    const FUNC_TABLE: [fn(&mut ViewState, &mut ViewStateTimer, f32); 4] = [
        update_timer_when_idle_state,
        update_timer_when_zoom_in_state,
        update_timer_when_zoom_out_state,
        update_timer_when_aiming_state,
    ];

    let i = *view_state as usize;
    FUNC_TABLE[i](view_state, view_state_timer, fixed_time_sec);
}

/// `ViewState::Idle`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_idle_state(_: &mut ViewState, _: &mut ViewStateTimer, _: f32) {
    /* empty */
}

/// `ViewState::ZoomIn`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_zoom_in_state(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    view_state_timer.update(fixed_time_sec);
    if view_state_timer.0 >= ViewStateTimer::MAX_TIME {
        *view_state = ViewState::Aiming
    }
}

/// `ViewState::ZoomOut`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_zoom_out_state(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    view_state_timer.update(fixed_time_sec);
    if view_state_timer.0 >= ViewStateTimer::MAX_TIME {
        *view_state = ViewState::Idle
    }
}

/// `ViewState::Aiming`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_aiming_state(_: &mut ViewState, _: &mut ViewStateTimer, _: f32) {
    /* empty */
}
