use winit::keyboard::{KeyCode, KeyLocation};

use crate::config::UserConfig;

/// ## Player Movement States
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementState {
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

impl MovementState {
    /// 키보드 입력이 발생했을 때 `MovementState`를 변경합니다.
    pub fn handle_keyboard_pressed(
        &mut self,
        config: &UserConfig,
        keycode: KeyCode,
        location: KeyLocation,
    ) {
        const FUNC_TABLE: [fn(&UserConfig, KeyCode, KeyLocation) -> MovementState; 9] = [
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

    /// 키보드 입력이 해제되었을 때 `MovementState`를 변경합니다.   
    pub fn handle_keyboard_released(
        &mut self,
        config: &UserConfig,
        keycode: KeyCode,
        location: KeyLocation,
    ) {
        const FUNC_TABLE: [fn(&UserConfig, KeyCode, KeyLocation) -> MovementState; 9] = [
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

/// `MovementState::Idle` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_idle_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::MovingLeft
    } else if config.keyboard.right == (keycode, location) {
        MovementState::MovingRight
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingForward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingBackward
    } else {
        MovementState::Idle
    }
}

/// `MovementState::Idle` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_idle_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::MovingRight
    } else if config.keyboard.right == (keycode, location) {
        MovementState::MovingLeft
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingForward
    } else {
        MovementState::Idle
    }
}

/// `MovementState::MovingLeft` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_left_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.right == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingLeftForward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingLeftBackward
    } else {
        MovementState::MovingLeft
    }
}

/// `MovementState::MovingLeft` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_left_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingForward
    } else {
        MovementState::MovingLeft
    }
}

/// `MovementState::MovingRight` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_right_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingRightForward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingRightBackward
    } else {
        MovementState::MovingRight
    }
}

/// `MovementState::MovingRight` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_right_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.right == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingForward
    } else {
        MovementState::MovingRight
    }
}

/// `MovementState::MovingForward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.backward == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.left == (keycode, location) {
        MovementState::MovingLeftForward
    } else if config.keyboard.right == (keycode, location) {
        MovementState::MovingRightForward
    } else {
        MovementState::MovingForward
    }
}

/// `MovementState::MovingForward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.forward == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.left == (keycode, location) {
        MovementState::MovingRightForward
    } else if config.keyboard.right == (keycode, location) {
        MovementState::MovingLeftForward
    } else {
        MovementState::MovingForward
    }
}

/// `MovementState::MovingBackward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.forward == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.left == (keycode, location) {
        MovementState::MovingLeftBackward
    } else if config.keyboard.right == (keycode, location) {
        MovementState::MovingRightBackward
    } else {
        MovementState::MovingBackward
    }
}

/// `MovementState::MovingBackward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.backward == (keycode, location) {
        MovementState::Idle
    } else if config.keyboard.left == (keycode, location) {
        MovementState::MovingRightBackward
    } else if config.keyboard.right == (keycode, location) {
        MovementState::MovingLeftBackward
    } else {
        MovementState::MovingBackward
    }
}

/// `MovementState::MovingLeftForward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_left_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.right == (keycode, location) {
        MovementState::MovingForward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingLeft
    } else {
        MovementState::MovingLeftForward
    }
}

/// `MovementState::MovingLeftForward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_left_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::MovingForward
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingLeft
    } else {
        MovementState::MovingLeftForward
    }
}

/// `MovementState::MovingRightForward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_right_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::MovingForward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingRight
    } else {
        MovementState::MovingRightForward
    }
}

/// `MovementState::MovingRightForward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_right_forward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.right == (keycode, location) {
        MovementState::MovingForward
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingRight
    } else {
        MovementState::MovingRightForward
    }
}

/// `MovementState::MovingLeftBackward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_left_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.right == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingLeft
    } else {
        MovementState::MovingLeftBackward
    }
}

/// `MovementState::MovingLeftBackward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_left_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingLeft
    } else {
        MovementState::MovingLeftBackward
    }
}

/// `MovementState::MovingRightBackward` 상태에서 키보드 눌림 이벤트를 처리합니다.
fn handle_keyboard_pressed_moving_right_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.left == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.forward == (keycode, location) {
        MovementState::MovingRight
    } else {
        MovementState::MovingRightBackward
    }
}

/// `MovementState::MovingRightBackward` 상태에서 키보드 떼임 이벤트를 처리합니다.
fn handle_keyboard_released_moving_right_backward_state(
    config: &UserConfig,
    keycode: KeyCode,
    location: KeyLocation,
) -> MovementState {
    if config.keyboard.right == (keycode, location) {
        MovementState::MovingBackward
    } else if config.keyboard.backward == (keycode, location) {
        MovementState::MovingRight
    } else {
        MovementState::MovingRightBackward
    }
}
