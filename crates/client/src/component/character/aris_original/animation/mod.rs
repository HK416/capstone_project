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
mod reload_move;
mod skill;
mod skill_jumping;
mod skill_landing;
mod skill_move;
mod victory_end;
mod victory_start;

use ahash::HashMap;
use hecs::{Component, ViewBorrow};
use lazy_static::lazy_static;
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, LatLon, MovementState, MovementStateTimer,
    PlayerStateData, MAX_JUMP_DURATION,
};

use crate::{
    asset::Motion,
    component::{
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
const HEAD_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(-0.06608068, 0.6346726, -0.7699505);
/// `Bip001_Spine`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
const SPINE_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(0.34206244, 0.9269642, -0.15404683);
/// `Bip001_Spine1`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
const SPINE1_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(0.10889296, 0.92688817, -0.35919425);

/// `Bip001_L_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.914507, -0.009351098, -0.4044626, 0.0),
    glam::vec4(0.1610851, 0.9086536, -0.3852281, 0.0),
    glam::vec4(0.3711186, -0.4174466, -0.8294636, 0.0),
    glam::vec4(0.00000007629394, 0.0000001049042, 0.07303865, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9209496, -0.2404988, 0.306615, 0.0),
    glam::vec4(-0.1418219, 0.9397302, 0.3111172, 0.0),
    glam::vec4(-0.3629586, 0.2430385, -0.899552, 0.0),
    glam::vec4(-0.00000009536743, -0.0000001001358, -0.07303863, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.9008837, 0.4340602, -0.00000002980233, 0.0),
    glam::vec4(-0.4340602, 0.9008837, 0.00000001490116, 0.0),
    glam::vec4(0.00000003331644, -0.0000000004882125, 0.9999999, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.00000001907349, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.6638219, 0.7478905, -0.0000001192093, 0.0),
    glam::vec4(-0.7478906, 0.663822, 0.00000002980231, 0.0),
    glam::vec4(0.0000001014226, 0.00000006937208, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.000000009536743, 0.0, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8172209, -0.3939509, -0.4206576, 0.0),
    glam::vec4(0.3484892, 0.9191245, -0.1837534, 0.0),
    glam::vec4(0.4590265, 0.003572434, 0.8884154, 0.0),
    glam::vec4(-0.1460184, 0.000000009536743, 0.0, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.7886113, -0.4748127, 0.3906981, 0.0),
    glam::vec4(0.5236893, 0.8516232, -0.02207781, 0.0),
    glam::vec4(-0.3222447, 0.2220152, 0.9202539, 0.0),
    glam::vec4(-0.1460184, 0.00000001907349, -0.00000001907349, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9689184, -0.1076179, -0.2227459, 0.0),
    glam::vec4(0.008400703, 0.8855835, -0.4644049, 0.0),
    glam::vec4(0.2472383, -0.4518415, -0.8571539, 0.0),
    glam::vec4(0.00000009536743, 0.00000009536743, 0.07303865, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.8693725, -0.1518831, 0.4702372, 0.0),
    glam::vec4(-0.009138854, 0.9563732, 0.2920055, 0.0),
    glam::vec4(-0.4940729, 0.2495641, -0.8328325, 0.0),
    glam::vec4(-0.0000001144409, -0.0000001049042, -0.07303862, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8196427, 0.5728754, 0.00000005419786, 0.0),
    glam::vec4(-0.5728753, 0.8196425, 0.000000001892902, 0.0),
    glam::vec4(-0.00000004333847, -0.00000003260012, 1.0, 0.0),
    glam::vec4(-0.1590945, -0.000000007152557, 0.0, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.5968635, 0.8023427, -0.0000001490116, 0.0),
    glam::vec4(-0.8023427, 0.5968635, 0.00000002980232, 0.0),
    glam::vec4(0.0000001128513, 0.0000001017705, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.00000001907349, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8093521, -0.4241125, -0.4062982, 0.0),
    glam::vec4(0.4180262, 0.9019042, -0.1087341, 0.0),
    glam::vec4(0.4125574, -0.08183914, 0.9072479, 0.0),
    glam::vec4(-0.1460184, 0.000000004768371, 0.00000003814697, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.6853706, -0.5776544, 0.4433765, 0.0),
    glam::vec4(0.616642, 0.7842523, 0.06856146, 0.0),
    glam::vec4(-0.3873239, 0.2264145, 0.8937094, 0.0),
    glam::vec4(-0.1460184, 0.00000001192093, -0.00000001907349, 1.0),
);

lazy_static! {
    static ref CHARACTER_ATTRIBUTE: CharacterAttributes = {
        let json = include_str!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/characters/aris_original/attribute.json"
        ));
        serde_json::from_str(json).unwrap()
    };
}

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
    state_data: PlayerStateData,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    latlon: LatLon,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let action_state = state_data.action_state();
    let movement_state = state_data.movement_state();
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
            _ => unreachable!("invalid game logic!"),
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
