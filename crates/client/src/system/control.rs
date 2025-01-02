use hecs::{Entity, World};

use crate::component::{
    Direction, FocusState, MovementState, ThirdPersonCamera, Timer, ViewState,
    MAX_CONTROL_INPUT_TIME, MAX_IN_OUT_TIME,
};

/// 컨트롤러가 눌려있을 때 입력 지연 시간을 갱신하는 함수입니다.
fn update_ctrl_timer_when_pressed(keyboard_input_time: &mut Timer, fixed_time_sec: f32) {
    // 키보드 입력 시간을 갱신합니다.
    keyboard_input_time.0 = (keyboard_input_time.0 + fixed_time_sec).min(MAX_CONTROL_INPUT_TIME);
}

/// 컨트롤러가 눌려있지 않을 때 입력 지연 시간을 갱신하는 함수입니다.
fn update_ctrl_timer_when_released(keyboard_input_time: &mut Timer, fixed_time_sec: f32) {
    // 키보드 입력 시간을 갱신합니다.
    keyboard_input_time.0 = (keyboard_input_time.0 - fixed_time_sec).max(0.0);
}

/// 입력 방향에 따라 플레이어의 방향을 갱신합니다.
///
/// # Note
/// 플레이어의 방향은 캐릭터가 바라보는 방향과 다를 수 있습니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 카메라 엔터티는 삼인칭 카메라 요소(`ThirdPersonCamera`)를 갖고 있어야 합니다.
/// 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_player_direction(
    world: &mut World,
    camera_entity: Entity,
    direction: &mut Direction,
    movement_state: &MovementState,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    const FUNC_TABLE: [fn(glam::Vec4, glam::Vec4, &mut Direction, &mut Timer, f32); 9] = [
        update_player_direction_when_idle_state,
        update_player_direction_when_moving_left_state,
        update_player_direction_when_moving_right_state,
        update_player_direction_when_moving_forward_state,
        update_player_direction_when_moving_backward_state,
        update_player_direction_when_moving_left_forward_state,
        update_player_direction_when_moving_right_forward_state,
        update_player_direction_when_moving_left_backward_state,
        update_player_direction_when_moving_right_backward_state,
    ];

    // 카메라가 바라보는 방향을 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");
    let view_right = third_person_camera.view_matrix_xz.x_axis.normalize();
    let view_forward = third_person_camera.view_matrix_xz.z_axis.normalize();

    let index = *movement_state as usize;
    FUNC_TABLE[index](
        view_right,
        view_forward,
        direction,
        keyboard_input_time,
        fixed_time_sec,
    );
}

/// `MovementState::Idle`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_idle_state(
    _view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    _direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_released(keyboard_input_time, fixed_time_sec);
}

/// `MovementState::MovingLeft`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_left_state(
    view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = -view_right;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    // let offset = get_direction_offset(direction.0, dir);
    // direction.0 = (direction.0 + dir * offset).normalize_or(dir);
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingRight`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_right_state(
    view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = view_right;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_forward_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_backward_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = -view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingLeftForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_left_forward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right + SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingRightForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_right_forward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right + SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingLeftBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_left_backward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right - SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `MovementState::MovingRightBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_right_backward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    direction: &mut Direction,
    keyboard_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right - SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_ctrl_timer_when_pressed(keyboard_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// 플레이어 뷰 상태를 갱신합니다.
pub fn update_player_view_state(
    focus_state: FocusState,
    view_state: &mut ViewState,
    view_state_timer: &mut Timer,
    fixed_time_sec: f32,
) {
    const FUNC_TABLE: [fn(FocusState, &mut ViewState, &mut Timer, f32); 4] = [
        update_player_view_state_when_idle_state,
        update_player_view_state_when_zoom_in_state,
        update_player_view_state_when_zoom_out_state,
        update_player_view_state_when_aimming_state,
    ];
    let index = *view_state as usize;
    FUNC_TABLE[index](focus_state, view_state, view_state_timer, fixed_time_sec);
}

/// `ViewState::Idle`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_idle_state(
    focus_state: FocusState,
    view_state: &mut ViewState,
    view_state_timer: &mut Timer,
    fixed_time_sec: f32,
) {
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => (ViewState::Idle, 0.0),
        FocusState::Aimming => (ViewState::ZoomIn, fixed_time_sec),
    }
}

/// `ViewState::ZoomIn`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_zoom_in_state(
    focus_state: FocusState,
    view_state: &mut ViewState,
    view_state_timer: &mut Timer,
    fixed_time_sec: f32,
) {
    // 함수 내에서 `view_state_timer`는 항상 `MAX_IN_OUT_TIME`보다 작다고 가정함.
    //
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => {
            let time = (MAX_IN_OUT_TIME - view_state_timer.0) + fixed_time_sec;
            if time >= MAX_IN_OUT_TIME {
                (ViewState::Idle, 0.0)
            } else {
                (ViewState::ZoomOut, time)
            }
        }
        FocusState::Aimming => {
            let time = view_state_timer.0 + fixed_time_sec;
            if time >= MAX_IN_OUT_TIME {
                (ViewState::Aimming, 0.0)
            } else {
                (ViewState::ZoomIn, time)
            }
        }
    }
}

/// `ViewState::ZoomOut`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_zoom_out_state(
    focus_state: FocusState,
    view_state: &mut ViewState,
    view_state_timer: &mut Timer,
    fixed_time_sec: f32,
) {
    // 함수 내에서 `view_state_timer`는 항상 `IN_OUT_TIME`보다 작다고 가정함.
    //
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => {
            let time = view_state_timer.0 + fixed_time_sec;
            if time >= MAX_IN_OUT_TIME {
                (ViewState::Idle, 0.0)
            } else {
                (ViewState::ZoomOut, time)
            }
        }
        FocusState::Aimming => {
            let time = (MAX_IN_OUT_TIME - view_state_timer.0) + fixed_time_sec;
            if time >= MAX_IN_OUT_TIME {
                (ViewState::Aimming, 0.0)
            } else {
                (ViewState::ZoomIn, time)
            }
        }
    }
}

/// `ViewState::Aimming`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_aimming_state(
    focus_state: FocusState,
    view_state: &mut ViewState,
    view_state_timer: &mut Timer,
    fixed_time_sec: f32,
) {
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => (ViewState::ZoomOut, fixed_time_sec),
        FocusState::Aimming => (ViewState::Aimming, 0.0),
    }
}
