//! 캐릭터 모델 애니메이션과 관련된 코드를 관리합니다.
//!

mod aim;
mod aim_jumping;
mod aim_landing;
mod aim_move;
mod aim_move_to_move;
mod aim_to_idle;
mod attack_jumping;
mod attack_landing;
mod attack_move;
mod attacking;
mod callsign;
mod death;
mod idle;
mod idle_to_aim;
mod jumping;
mod landing;
mod move_to_aim_move;
mod move_to_end;
mod moving;
mod reload;
mod reload_jumping;
mod reload_landing;
mod reload_move;
mod skill;
mod skill_jumping;
mod skill_landing;
mod skill_move;
mod victory_end;
mod victory_start;

use ahash::HashMap;
use hecs::{Component, ViewBorrow};
use mod_network::components::{
    ActionState, ActionStateTimer, LatLon, MovementState, MovementStateTimer, MAX_JUMP_DURATION,
};

use crate::{
    asset::Motion,
    component::{
        character::yuuka_original::animation::{
            reload_jumping::animate_character_when_reload_jumping,
            reload_landing::animate_character_when_reload_landing,
        },
        BoneCollection, SkinningAnimation, ToParentTrans, ATTACK_END_ANIMATION_SUFFIX,
        ATTACK_ING_ANIMATION_SUFFIX, ATTACK_START_ANIMATION_SUFFIX, CAFE_WALK_ANIMATION_SUFFIX,
        EXS_ANIMATION_SUFFIX, IDLE_ANIMATION_SUFFIX, MOVE_TO_END_ANIMATION_SUFFIX,
        MOVING_ANIMATION_SUFFIX, NORMAL_CALLSIGN_SUFFIX, RELOAD_ANIMATION_SUFFIX,
        VICTORY_END_SUFFIX, VICTORY_START_SUFFIX, VITAL_DEATH_ANIMATION_SUFFIX,
    },
};

use self::{
    aim::*, aim_jumping::*, aim_landing::*, aim_move::*, aim_move_to_move::*, aim_to_idle::*,
    attack_jumping::*, attack_landing::*, attack_move::*, attacking::*, callsign::*, death::*,
    idle::*, idle_to_aim::*, jumping::*, landing::*, move_to_aim_move::*, move_to_end::*,
    moving::*, reload::*, reload_move::*, skill::*, skill_jumping::*, skill_landing::*,
    skill_move::*, victory_end::*, victory_start::*,
};

use super::*;

/// 캐릭터의 Idle 애니메이션 이름입니다.
const IDLE_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, IDLE_ANIMATION_SUFFIX);
/// 캐릭터의 Moving 애니메이션 이름입니다.
const MOVING_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, MOVING_ANIMATION_SUFFIX);
/// 캐릭터의 MoveToEnd 애니메이션 이름입니다.
const MOVE_TO_END_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, MOVE_TO_END_ANIMATION_SUFFIX);
/// 캐릭터의 CafeWalk 애니메이션 이름입니다.
const CAFE_WALK_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, CAFE_WALK_ANIMATION_SUFFIX);
/// 캐릭터의 AttackStart 애니메이션 이름입니다.
const ATTACK_START_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, ATTACK_START_ANIMATION_SUFFIX);
/// 캐릭터의 Attacking 애니메이션 이름입니다.
const ATTACK_ING_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, ATTACK_ING_ANIMATION_SUFFIX);
/// 캐릭터의 AttackEnd 애니메이션 이름입니다.
const ATTACK_END_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, ATTACK_END_ANIMATION_SUFFIX);
/// 캐릭터의 VitalDeath 애니메이션 이름입니다.
const VITAL_DEATH_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, VITAL_DEATH_ANIMATION_SUFFIX);
/// 캐릭터의 Reload 애니메이션 이름입니다.
const NORMAL_RELOAD_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, RELOAD_ANIMATION_SUFFIX);
/// 캐릭터의 Skill 애니메이션 이름입니다.
const SKILL_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, EXS_ANIMATION_SUFFIX);
/// 캐릭터의 Callsign 애니메이션 이름입니다.
const NORMAL_CALLSIGN_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, NORMAL_CALLSIGN_SUFFIX);
/// 캐릭터의 *_Victory_Start 애니메이션 이름입니다.
const VICTORY_START_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, VICTORY_START_SUFFIX);
/// 캐릭터의 *_Victory_End 애니메이션 이름입니다.
const VICTORY_END_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, VICTORY_END_SUFFIX);

/// `Bip001_Head`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
pub const HEAD_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 =
    glam::vec3(-0.050692074, 0.60760593, -0.792619);
/// `Bip001_Spine`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
pub const SPINE_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 =
    glam::vec3(0.018127501, 0.88046324, -0.47376776);
/// `Bip001_Spine1`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
pub const SPINE1_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 =
    glam::vec3(-0.16704176, 0.9152928, -0.3665189);
/// `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 `Bip001_L_Hand`에서 `Bip001_Weapon`까지의 변환 행렬입니다.
pub const WEAPON_OFFSET: glam::Mat4 = glam::Mat4::from_cols(
    glam::Vec4::new(-0.25398198, 0.96574277, 0.05343934, 0.0),
    glam::Vec4::new(-0.22625628, -0.00560271, -0.9740808, 0.0),
    glam::Vec4::new(-0.9403992, -0.25946116, 0.21992218, 0.0),
    glam::Vec4::new(-0.053849846, 0.01216054, 0.017126352, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.7308136, -0.4484693, -0.5145746, 0.0),
    glam::vec4(-0.4265977, 0.8885932, -0.168573, 0.0),
    glam::vec4(0.5328473, 0.0963209, -0.840712, 0.0),
    glam::vec4(0.00000009536743, 0.0000001215935, 0.07688279, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.8947756, -0.4049033, 0.1882291, 0.0),
    glam::vec4(-0.3156573, 0.8717507, 0.3747147, 0.0),
    glam::vec4(-0.3158121, 0.2758697, -0.9078319, 0.0),
    glam::vec4(-0.0000001144409, -0.0000001049042, -0.07688277, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.6872864, 0.7263864, 0.0000001192093, 0.0),
    glam::vec4(-0.7263865, 0.6872865, -0.000000007450585, 0.0),
    glam::vec4(-0.00000008734294, -0.00000008147134, 1.0, 0.0),
    glam::vec4(-0.1674679, 0.0, -0.00000001907349, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.52034, 0.8539593, 0.0000000000000008881785, 0.0),
    glam::vec4(-0.8539593, 0.52034, -0.00000002980232, 0.0),
    glam::vec4(-0.00000002544997, 0.00000001550734, 1.0, 0.0),
    glam::vec4(-0.1674679, 0.000000009536743, 0.0, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8666437, -0.1439797, -0.4777017, 0.0),
    glam::vec4(0.134106, 0.9894437, -0.0549249, 0.0),
    glam::vec4(0.4805669, -0.01646233, 0.8768035, 0.0),
    glam::vec4(-0.1537036, 0.00000001907349, 0.0, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.7690588, -0.5080033, 0.3879192, 0.0),
    glam::vec4(0.5303443, 0.8459078, 0.0563467, 0.0),
    glam::vec4(-0.3567682, 0.1623968, 0.9199694, 0.0),
    glam::vec4(-0.1537036, -0.000000002384186, 0.0, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9263942, -0.08319063, -0.3672509, 0.0),
    glam::vec4(-0.1037003, 0.9939411, 0.03643499, 0.0),
    glam::vec4(0.3619947, 0.0718372, -0.9294082, 0.0),
    glam::vec4(0.00000009536743, 0.0000001239777, 0.0768828, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9286584, -0.07803348, 0.3626359, 0.0),
    glam::vec4(0.02989662, 0.9586961, 0.2828571, 0.0),
    glam::vec4(-0.3697299, 0.2735192, -0.8879682, 0.0),
    glam::vec4(-0.00000009536743, -0.00000009536743, -0.07688278, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.9296805, 0.368367, 0.00000005960465, 0.0),
    glam::vec4(-0.368367, 0.9296805, -0.00000005960469, 0.0),
    glam::vec4(-0.00000007736968, 0.00000003345693, 1.0, 0.0),
    glam::vec4(-0.1674679, 0.0, 0.0, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8413359, 0.5405127, 0.0000000298023, 0.0),
    glam::vec4(-0.5405126, 0.8413358, 0.00000001490114, 0.0),
    glam::vec4(-0.00000001701949, -0.00000002864539, 1.0, 0.0),
    glam::vec4(-0.1674679, 0.0, 0.0, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.9237275, -0.06798658, -0.3769691, 0.0),
    glam::vec4(0.04067343, 0.9959681, -0.07995695, 0.0),
    glam::vec4(0.3808852, 0.0585258, 0.9227682, 0.0),
    glam::vec4(-0.1537036, 0.000000009536743, 0.00000001907349, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8483989, -0.3687848, 0.3797592, 0.0),
    glam::vec4(0.3878545, 0.9212896, 0.02818173, 0.0),
    glam::vec4(-0.3602612, 0.123382, 0.9246559, 0.0),
    glam::vec4(-0.1537036, -0.00000001549721, -0.00000001907349, 1.0),
);

/// 캐릭터가 카메라가 바라보는 방향을 바라보도록 로컬 변환 행렬을 수정합니다.
fn look_to_camera_direction<Tag: Copy + Component>(
    offset: f32,
    latlon: LatLon,
    skinning_animation: &SkinningAnimation,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let latitude = latlon.lat.to_f32_const() + 3f32.to_radians();

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation.lower_spine;
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 *= glam::Mat4::from_axis_angle(SPINE_W2L_X_NORMAL_ATTACK_ING, angle);

    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation.uppper_spine;
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 *= glam::Mat4::from_axis_angle(SPINE1_W2L_X_NORMAL_ATTACK_ING, angle);

    let angle = latitude / 7.0 * offset;
    let bone_entity = skinning_animation.head;
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 *= glam::Mat4::from_axis_angle(HEAD_W2L_X_NORMAL_ATTACK_ING, angle);
}

/// 점프 애니메이션을 적용합니다.
fn jump_animation<Tag: Copy + Component>(
    skinning_animation: &SkinningAnimation,
    movement_state_timer: MovementStateTimer,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let t = movement_state_timer.0.min(MAX_JUMP_DURATION) as f32 / MAX_JUMP_DURATION as f32;

    let angle = -25f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_thigh;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let angle = 10f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_thigh;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_calf;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_ATTACKING_IDENTITY;

    let angle = 60f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_calf;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_foot;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_ATTACKING_IDENTITY;

    let entity = skinning_animation.right_foot;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_FOOT_NORMAL_ATTACKING_IDENTITY;
}

/// 착지 애니메이션을 적용합니다.
fn landing_animation<Tag: Copy + Component>(
    skinning_animation: &SkinningAnimation,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let angle = -25f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_thigh;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let angle = 10f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_thigh;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_calf;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_ATTACKING_IDENTITY;

    let angle = 60f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_calf;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_foot;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_ATTACKING_IDENTITY;

    let entity = skinning_animation.right_foot;
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_FOOT_NORMAL_ATTACKING_IDENTITY;
}

/// 캐릭터 애니메이션을 재생합니다.
pub fn animate_character<Tag: Copy + Component>(
    motions: &HashMap<String, Motion>,
    skinning_animation: &SkinningAnimation,
    action_state: ActionState,
    movement_state: MovementState,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    latlon: LatLon,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let character_attribute = &CHARACTER_ATTRIBUTE;
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => {
                animate_character_when_idle(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_moving(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::MoveToEnd => {
                animate_character_when_move_to_end(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::Aiming => match movement_state {
            MovementState::Idle | MovementState::MoveToEnd => {
                animate_character_when_aiming(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_aim_move(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_aim_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_aim_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::AimAt => match movement_state {
            MovementState::Idle | MovementState::MoveToEnd => {
                animate_character_when_idle_to_aim(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_move_to_aim_move(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_aim_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_aim_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::AimOff => match movement_state {
            MovementState::Idle | MovementState::MoveToEnd => {
                animate_character_when_aim_to_idle(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_aim_move_to_move(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_aim_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_aim_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::Attack => match movement_state {
            MovementState::Idle | MovementState::MoveToEnd => {
                animate_character_when_attacking(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_attack_move(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_attack_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_attack_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::Death => {
            animate_character_when_death(
                motions,
                skinning_animation,
                character_attribute,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                transform_view,
            );
        }
        ActionState::Reload => match movement_state {
            MovementState::Idle | MovementState::MoveToEnd => {
                animate_character_when_reload(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_reload_move(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_reload_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_reload_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::Skill => match movement_state {
            MovementState::Idle | MovementState::MoveToEnd => {
                animate_character_when_skill(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Moving => {
                animate_character_when_skill_move(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Jumping => {
                animate_character_when_skill_jumping(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
            MovementState::Landing => {
                animate_character_when_skill_landing(
                    motions,
                    skinning_animation,
                    character_attribute,
                    action_state_timer,
                    movement_state_timer,
                    latlon,
                    collection_view,
                    transform_view,
                );
            }
        },
        ActionState::Callsign => {
            animate_character_when_callsign(
                motions,
                skinning_animation,
                character_attribute,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                transform_view,
            );
        }
        ActionState::VictoryStart => {
            animate_character_when_victory_start(
                motions,
                skinning_animation,
                character_attribute,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                transform_view,
            );
        }
        ActionState::VictoryEnd => {
            animate_character_when_victory_end(
                motions,
                skinning_animation,
                character_attribute,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                transform_view,
            );
        }
    }
}
