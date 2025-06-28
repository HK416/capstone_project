//! `Midori_Original` 모델의 카메라 설정과 관련된 코드를 관리합니다.
//!

use glam::FloatExt;
use mod_network::components::{
    ActionState, ActionStateTimer, ViewState, ViewStateTimer, NUM_VIEW_STATES,
};

use super::*;

/// 각 캐릭터에 해당하는 카메라 기본 상대 위치 입니다.
pub const CAMERA_DEF_REL_POS: glam::Vec3A = glam::vec3a(0.25, 0.85, 2.0);
/// 각 캐릭터에 해당하는 카메라 줌인 상대 위치 입니다.
pub const CAMERA_ZOOM_REL_POS: glam::Vec3A = glam::vec3a(0.25, 0.7, 0.15);
/// 각 캐릭터에 해당하는 카메라 기본 Fov-y 각도입니다. (단위: 라디안)
pub const CAMERA_DEF_FOV_Y: f32 = 45f32.to_radians();
/// 각 캐릭터에 해당하는 카메라 줌인 Fov-y 각도입니다. (단위: 라디안)
pub const CAMERA_ZOOM_FOV_Y: f32 = 15f32.to_radians();

/// 캐릭터와 연결된 카메라의 파라미터를 갱신합니다.
pub fn update_camera_param(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    action_state: ActionState,
    view_state: ViewState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    type Func = fn(&mut glam::Vec3A, &mut f32, ActionState, ActionStateTimer, ViewStateTimer);
    const TABLE: [Func; NUM_VIEW_STATES] = [
        update_camera_when_idle,
        update_camera_when_zoom_in,
        update_camera_when_zoom_out,
        update_camera_when_aiming,
    ];

    let i = view_state as usize;
    TABLE[i](
        camera_rel_pos,
        camera_fov_y,
        action_state,
        action_state_timer,
        view_state_timer,
    );
}

/// [`ViewState::Idle`]일 떄 캐릭터와 연결된 카메라를 갱신합니다.
fn update_camera_when_idle(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    _view_state_timer: ViewStateTimer,
) {
    match action_state {
        ActionState::Attack => attack_wave_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_DEF_REL_POS,
            CAMERA_DEF_FOV_Y,
        ),
        _ => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_DEF_REL_POS,
            CAMERA_DEF_FOV_Y,
        ),
    };
}

/// [`ViewState::ZoomIn`]일 떄 캐릭터와 연결된 카메라를 갱신합니다.
fn update_camera_when_zoom_in(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    let duration = CHARACTER_ATTRIBUTE.normal_attack_start_duration;
    let s = view_state_timer.0 as f32 / duration as f32;
    let rel_pos = CAMERA_DEF_REL_POS.lerp(CAMERA_ZOOM_REL_POS, s);
    let fov_y = CAMERA_DEF_FOV_Y.lerp(CAMERA_ZOOM_FOV_Y, s);

    match action_state {
        ActionState::Attack => attack_wave_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            rel_pos,
            fov_y,
        ),
        ActionState::Death
        | ActionState::Callsign
        | ActionState::VictoryEnd
        | ActionState::VictoryStart => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_DEF_REL_POS,
            CAMERA_DEF_FOV_Y,
        ),
        _ => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            rel_pos,
            fov_y,
        ),
    };
}

/// [`ViewState::ZoomOut`]일 떄 캐릭터와 연결된 카메라를 갱신합니다.
fn update_camera_when_zoom_out(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    let duration = CHARACTER_ATTRIBUTE.normal_attack_end_duration;
    let s = view_state_timer.0 as f32 / duration as f32;
    let rel_pos = CAMERA_ZOOM_REL_POS.lerp(CAMERA_DEF_REL_POS, s);
    let fov_y = CAMERA_ZOOM_FOV_Y.lerp(CAMERA_DEF_FOV_Y, s);

    match action_state {
        ActionState::Attack => attack_wave_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            rel_pos,
            fov_y,
        ),
        ActionState::Death
        | ActionState::Callsign
        | ActionState::VictoryEnd
        | ActionState::VictoryStart => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_DEF_REL_POS,
            CAMERA_DEF_FOV_Y,
        ),
        _ => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            rel_pos,
            fov_y,
        ),
    };
}

/// [`ViewState::Aiming`]일 떄 캐릭터와 연결된 카메라를 갱신합니다.
fn update_camera_when_aiming(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    _view_state_timer: ViewStateTimer,
) {
    match action_state {
        ActionState::Attack => attack_wave_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_ZOOM_REL_POS,
            CAMERA_ZOOM_FOV_Y,
        ),
        ActionState::Death
        | ActionState::Callsign
        | ActionState::VictoryEnd
        | ActionState::VictoryStart => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_DEF_REL_POS,
            CAMERA_DEF_FOV_Y,
        ),
        _ => none_camera_effect(
            camera_rel_pos,
            camera_fov_y,
            action_state_timer,
            CAMERA_ZOOM_REL_POS,
            CAMERA_ZOOM_FOV_Y,
        ),
    };
}

/// 카메라 이펙트 효과를 적용하지 않습니다.
fn none_camera_effect(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    _action_state_timer: ActionStateTimer,
    rel_pos: glam::Vec3A,
    fov_y: f32,
) {
    *camera_rel_pos = rel_pos;
    *camera_fov_y = fov_y;
}

/// 공격 흔들림 카메라 이펙트 효과를 적용합니다.
fn attack_wave_camera_effect(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    action_state_timer: ActionStateTimer,
    rel_pos: glam::Vec3A,
    fov_y: f32,
) {
    const TIME_POINT_0: u16 = 900;
    const TIME_POINT_1: u16 = 950;
    const TIME_POINT_2: u16 = 1000;
    const WAVE_OFFSET: f32 = 7f32.to_radians();

    *camera_rel_pos = rel_pos;
    if (TIME_POINT_0..TIME_POINT_1).contains(&action_state_timer.0) {
        let t = (action_state_timer.0 - TIME_POINT_0) as f32 / (TIME_POINT_1 - TIME_POINT_0) as f32;
        let s = t * t / (t * t + (1.0 - t) * (1.0 - t));
        *camera_fov_y = fov_y + WAVE_OFFSET * s;
    } else if (TIME_POINT_1..=TIME_POINT_2).contains(&action_state_timer.0) {
        let t = (action_state_timer.0 - TIME_POINT_1) as f32 / (TIME_POINT_2 - TIME_POINT_1) as f32;
        let s = 1.0 - t * t / (t * t + (1.0 - t) * (1.0 - t));
        *camera_fov_y = fov_y + WAVE_OFFSET * s;
    } else {
        *camera_fov_y = fov_y;
    }
}
