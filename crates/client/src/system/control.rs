use hecs::{Entity, World};

use crate::component::{
    ControllerState, Direction, FocusState, ThirdPersonCamera, Timer, ViewState, ViewStateTimer,
    ZoomLength, MAX_CONTROL_INPUT_TIME,
};

/// 컨트롤러가 눌려있을 때 입력 지연 시간을 갱신하는 함수입니다.
fn update_controller_timer_when_pressed(controller_input_time: &mut Timer, fixed_time_sec: f32) {
    controller_input_time.0 =
        (controller_input_time.0 + fixed_time_sec).min(MAX_CONTROL_INPUT_TIME);
}

/// 컨트롤러가 눌려있지 않을 때 입력 지연 시간을 갱신하는 함수입니다.
fn update_controller_timer_when_released(controller_input_time: &mut Timer, fixed_time_sec: f32) {
    controller_input_time.0 = (controller_input_time.0 - fixed_time_sec).max(0.0);
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
    player_entity: Entity,
    camera_entity: Entity,
    direction: &mut Direction,
    controller_state: ControllerState,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    const FUNC_TABLE: [fn(glam::Vec4, glam::Vec4, ViewState, &mut Direction, &mut Timer, f32); 9] = [
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

    // 카메라 엔터티에서 카메라가 바라보는 방향을 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");
    let view_right = third_person_camera
        .view_matrix_xz
        .x_axis
        .normalize_or(glam::Vec4::X);
    let view_forward = third_person_camera
        .view_matrix_xz
        .z_axis
        .normalize_or(glam::Vec4::Z);

    // 플레이어 엔터티에서 뷰 상태를 가져옵니다.
    let &view_state = world
        .query_one_mut::<&ViewState>(player_entity)
        .expect("invalid entity or invalid entity component");

    let index = controller_state as usize;
    FUNC_TABLE[index](
        view_right,
        view_forward,
        view_state,
        direction,
        controller_input_time,
        fixed_time_sec,
    );
}

/// `ControllerState::Idle`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_idle_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_released(controller_input_time, fixed_time_sec);

    // 뷰 상태가 `Idle`이 아닌 경우 플레이어 방향과 카메라 방향을 일치시킵니다.
    if view_state == ViewState::ZoomIn
        || view_state == ViewState::ZoomOut
        || view_state == ViewState::Aiming
    {
        direction.0 = view_forward;
    }
}

/// `ControllerState::MovingLeft`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_left_state(
    view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = -view_right;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    // let offset = get_direction_offset(direction.0, dir);
    // direction.0 = (direction.0 + dir * offset).normalize_or(dir);
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingRight`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_right_state(
    view_right: glam::Vec4,
    _view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = view_right;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_forward_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_backward_state(
    _view_right: glam::Vec4,
    view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    // 이동 방향 벡터를 계산합니다.
    let dir = -view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingLeftForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_left_forward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right + SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingRightForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_right_forward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right + SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingLeftBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_left_backward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right - SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// `ControllerState::MovingRightBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_player_direction_when_moving_right_backward_state(
    view_right: glam::Vec4,
    view_forward: glam::Vec4,
    _view_state: ViewState,
    direction: &mut Direction,
    controller_input_time: &mut Timer,
    fixed_time_sec: f32,
) {
    use core::f32::consts::SQRT_2;

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right - SQRT_2 * view_forward;

    // 키보드 입력 시간을 갱신합니다.
    update_controller_timer_when_pressed(controller_input_time, fixed_time_sec);

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, 0.1);
}

/// 플레이어 뷰 상태를 갱신합니다.
pub fn update_player_view_state(
    world: &mut World,
    player_entity: Entity,
    focus_state: FocusState,
    fixed_time_sec: f32,
) {
    type Func = for<'a, 'b, 'c> fn(
        FocusState,
        &'a ZoomLength,
        &'b mut ViewState,
        &'c mut ViewStateTimer,
        f32,
    );
    const FUNC_TABLE: [Func; 4] = [
        update_player_view_state_when_idle_state,
        update_player_view_state_when_zoom_in_state,
        update_player_view_state_when_zoom_out_state,
        update_player_view_state_when_aiming_state,
    ];

    // 플레이어 엔터티에서 뷰 상태와 뷰 상태 타이머를 가져옵니다.
    let (length, view_state, view_state_timer) = world
        .query_one_mut::<(&ZoomLength, &mut ViewState, &mut ViewStateTimer)>(player_entity)
        .expect("invalid entity or invalid entity component");

    let index = *view_state as usize;
    FUNC_TABLE[index](
        focus_state,
        length,
        view_state,
        view_state_timer,
        fixed_time_sec,
    );
}

/// `ViewState::Idle`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_idle_state(
    focus_state: FocusState,
    _length: &ZoomLength,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => (ViewState::Idle, 0.0),
        FocusState::Aiming => (ViewState::ZoomIn, fixed_time_sec),
    }
}

/// `ViewState::ZoomIn`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_zoom_in_state(
    focus_state: FocusState,
    length: &ZoomLength,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    // 함수 내에서 `view_state_timer`는 항상 `MAX_IN_OUT_TIME`보다 작다고 가정함.
    //
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => {
            let time = (length.in_time - view_state_timer.0) + fixed_time_sec;
            if time >= length.in_time {
                (ViewState::Idle, 0.0)
            } else {
                (ViewState::ZoomOut, time)
            }
        }
        FocusState::Aiming => {
            let time = view_state_timer.0 + fixed_time_sec;
            if time >= length.in_time {
                (ViewState::Aiming, 0.0)
            } else {
                (ViewState::ZoomIn, time)
            }
        }
    }
}

/// `ViewState::ZoomOut`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_zoom_out_state(
    focus_state: FocusState,
    length: &ZoomLength,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    // 함수 내에서 `view_state_timer`는 항상 `IN_OUT_TIME`보다 작다고 가정함.
    //
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => {
            let time = view_state_timer.0 + fixed_time_sec;
            if time >= length.out_time {
                (ViewState::Idle, 0.0)
            } else {
                (ViewState::ZoomOut, time)
            }
        }
        FocusState::Aiming => {
            let time = (length.out_time - view_state_timer.0) + fixed_time_sec;
            if time >= length.out_time {
                (ViewState::Aiming, 0.0)
            } else {
                (ViewState::ZoomIn, time)
            }
        }
    }
}

/// `ViewState::Aiming`일 때 플레이어 뷰 상태를 갱신합니다.
fn update_player_view_state_when_aiming_state(
    focus_state: FocusState,
    _length: &ZoomLength,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    fixed_time_sec: f32,
) {
    (*view_state, view_state_timer.0) = match focus_state {
        FocusState::Idle => (ViewState::ZoomOut, fixed_time_sec),
        FocusState::Aiming => (ViewState::Aiming, 0.0),
    }
}
