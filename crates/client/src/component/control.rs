use winit::{
    event::MouseButton,
    keyboard::{KeyCode, KeyLocation},
};

use crate::config::UserConfig;

/// ## Player Controller States
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

/// ## Player Movement States
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementState {
    Idle = 0,
    Moving = 1,
    MoveToEnd = 2,
}

impl Default for MovementState {
    fn default() -> Self {
        MovementState::Idle
    }
}

/// ## Player Movement State Timer
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MovementStateTimer(pub f32);

impl MovementStateTimer {
    /// 타이머를 초기화합니다.
    pub fn reset(&mut self) {
        self.0 = 0.0
    }
}

impl Default for MovementStateTimer {
    fn default() -> Self {
        Self(0.0)
    }
}

/// ## Player Focus States
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FocusState {
    Idle = 0,
    Aiming = 1,
}

impl Default for FocusState {
    fn default() -> Self {
        FocusState::Idle
    }
}

impl FocusState {
    /// 마우스 버튼 눌림이 발생했을 때 `FocusState`를 변경합니다.
    pub fn handle_mouse_btn_pressed(&mut self, config: &UserConfig, button: MouseButton) {
        const FUNC_TABLE: [fn(&UserConfig, MouseButton) -> FocusState; 2] = [
            handle_mouse_btn_pressed_when_idle_state,
            handle_mouse_btn_pressed_when_aiming_state,
        ];
        let index = *self as usize;
        *self = FUNC_TABLE[index](config, button);
    }

    /// 마우스 버튼 떼임이 발생했을 때 `FocusState`를 변경합니다.
    pub fn handle_mouse_btn_released(&mut self, config: &UserConfig, button: MouseButton) {
        const FUNC_TABLE: [fn(&UserConfig, MouseButton) -> FocusState; 2] = [
            handle_mouse_btn_released_when_idle_state,
            handle_mouse_btn_released_when_aiming_state,
        ];
        let index = *self as usize;
        *self = FUNC_TABLE[index](config, button);
    }
}

/// `FocusState::Idle` 상태에서 마우스 버튼 눌림 이벤트를 처리합니다.
fn handle_mouse_btn_pressed_when_idle_state(
    config: &UserConfig,
    button: MouseButton,
) -> FocusState {
    if config.mouse.aiming == button {
        FocusState::Aiming
    } else {
        FocusState::Idle
    }
}

/// `FocusState::Idle` 상태에서 마우스 버튼 떼임 이벤트를 처리합니다.
fn handle_mouse_btn_released_when_idle_state(
    _config: &UserConfig,
    _button: MouseButton,
) -> FocusState {
    FocusState::Idle
}

/// `FocusState::Aiming` 상태에서 마우스 버튼 눌림 이벤트를 처리합니다.
fn handle_mouse_btn_pressed_when_aiming_state(
    _config: &UserConfig,
    _button: MouseButton,
) -> FocusState {
    FocusState::Aiming
}

/// `FocusState::Aiming` 상태에서 마우스 버튼 떼임 이벤트를 처리합니다.
fn handle_mouse_btn_released_when_aiming_state(
    config: &UserConfig,
    button: MouseButton,
) -> FocusState {
    if config.mouse.aiming == button {
        FocusState::Idle
    } else {
        FocusState::Aiming
    }
}

/// ## Player View States
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ViewState {
    Idle = 0,
    ZoomIn = 1,
    ZoomOut = 2,
    Aiming = 3,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState::Idle
    }
}

/// ## Player View Zoom Length
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ZoomLength {
    pub in_time: f32,
    pub out_time: f32,
}

impl Default for ZoomLength {
    fn default() -> Self {
        Self {
            in_time: 0.0,
            out_time: 0.0,
        }
    }
}

/// ## Player View State Timer
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ViewStateTimer(pub f32);

impl Default for ViewStateTimer {
    fn default() -> Self {
        Self(0.0)
    }
}
