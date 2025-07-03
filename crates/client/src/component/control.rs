use mod_network::components::{ControllerState, LatLon, NUM_CONTROLLER_STATES};

/// 플레이어가 이동하고자 하는 방향을 나타냅니다. (캐릭터가 바라보는 방향과 다를 수 있습니다)
///
/// ## Note
/// SIMD 지원을 위해 4차원 벡터를 사용함.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveDirection(pub glam::Vec3A);

impl MoveDirection {
    /// 삼인칭 카메라 시점을 기준으로 컨트롤러 상태에 따라 플레이어 이동 방향을 갱신합니다.
    pub fn update_from_third_person_camera(
        &mut self,
        controller: ControllerState,
        latlon: &LatLon,
    ) {
        type Func = fn(glam::Vec3A, glam::Vec3A, &mut MoveDirection);
        const FUNC_TABLE: [Func; NUM_CONTROLLER_STATES] = [
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
        let mat = glam::Mat4::from_rotation_y(latlon.lon);
        let view_right = glam::Vec3A::from_vec4(mat.x_axis).normalize_or(glam::Vec3A::X);
        let view_forward = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

        let index = controller as usize;
        FUNC_TABLE[index](view_right, view_forward, self);
    }
}

impl Default for MoveDirection {
    fn default() -> Self {
        Self(glam::Vec3A::Z)
    }
}

/// `ControllerState::Idle`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_idle_state(
    _view_right: glam::Vec3A,
    _view_forward: glam::Vec3A,
    _direction: &mut MoveDirection,
) {
    /* empty */
}

/// `ControllerState::MovingLeft`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_left_state(
    view_right: glam::Vec3A,
    _view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = -view_right;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingRight`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_right_state(
    view_right: glam::Vec3A,
    _view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = view_right;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_forward_state(
    _view_right: glam::Vec3A,
    view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = view_forward;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_backward_state(
    _view_right: glam::Vec3A,
    view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::PI;

    // 이동 방향 벡터를 계산합니다.
    let dir = -view_forward;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingLeftForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_left_forward_state(
    view_right: glam::Vec3A,
    view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right + SQRT_2 * view_forward;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingRightForward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_right_forward_state(
    view_right: glam::Vec3A,
    view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right + SQRT_2 * view_forward;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingLeftBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_left_backward_state(
    view_right: glam::Vec3A,
    view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = -SQRT_2 * view_right - SQRT_2 * view_forward;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}

/// `ControllerState::MovingRightBackward`상태에서 플레이어의 방향을 갱신합니다.
fn update_move_direction_when_moving_right_backward_state(
    view_right: glam::Vec3A,
    view_forward: glam::Vec3A,
    direction: &mut MoveDirection,
) {
    use core::f32::consts::{PI, SQRT_2};

    // 이동 방향 벡터를 계산합니다.
    let dir = SQRT_2 * view_right - SQRT_2 * view_forward;

    // 두 벡터의 각도로 부터 보정 값을 계산합니다.
    let dst_v = dir;
    let src_v = direction.0;
    let s = dst_v.angle_between(src_v) / PI * 0.5 + 0.5;

    // 플레이어 방향을 갱신합니다.
    direction.0 = direction.0.lerp(dir, s).normalize_or(dir);
}
