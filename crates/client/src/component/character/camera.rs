use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, NUM_CHARACTERS, ViewState, ViewStateTimer,
};

use super::*;

/// 각 캐릭터에 해당하는 카메라 기본 상대 위치 입니다.
pub const CAMERA_DEF_REL_POS: [glam::Vec3A; NUM_CHARACTERS] = [
    aris_original::CAMERA_DEF_REL_POS,
    momoi_original::CAMERA_DEF_REL_POS,
    midori_original::CAMERA_DEF_REL_POS,
    yuuka_original::CAMERA_DEF_REL_POS,
];
/// 각 캐릭터에 해당하는 카메라 기본 Fov-y 각도입니다. (단위: 라디안)
pub const CAMERA_DEF_FOV_Y: [f32; NUM_CHARACTERS] = [
    aris_original::CAMERA_DEF_FOV_Y,
    momoi_original::CAMERA_DEF_FOV_Y,
    midori_original::CAMERA_DEF_FOV_Y,
    yuuka_original::CAMERA_DEF_FOV_Y,
];

/// 캐릭터와 연결된 카메라의 파라미터를 갱신합니다.
pub fn update_camera_param(
    camera_rel_pos: &mut glam::Vec3A,
    camera_fov_y: &mut f32,
    character_kind: CharacterKind,
    action_state: ActionState,
    view_state: ViewState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    type Func =
        fn(&mut glam::Vec3A, &mut f32, ActionState, ViewState, ActionStateTimer, ViewStateTimer);
    const TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::update_camera_param,
        momoi_original::update_camera_param,
        midori_original::update_camera_param,
        yuuka_original::update_camera_param,
    ];

    let i = character_kind as usize;
    TABLE[i](
        camera_rel_pos,
        camera_fov_y,
        action_state,
        view_state,
        action_state_timer,
        view_state_timer,
    );
}
