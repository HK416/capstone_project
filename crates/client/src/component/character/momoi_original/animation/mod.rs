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
        character::momoi_original::animation::{
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
pub const HEAD_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(-0.0489224, 0.6057397, -0.7941568);
/// `Bip001_Spine`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
pub const SPINE_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(0.01910567, 0.8792505, -0.4759759);
/// `Bip001_Spine1`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
pub const SPINE1_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 =
    glam::vec3(-0.16422231, 0.91393447, -0.37115225);
/// `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 `Bip001_L_Hand`에서 `Bip001_Weapon`까지의 변환 행렬입니다.
pub const WEAPON_OFFSET: glam::Mat4 = glam::Mat4::from_cols(
    glam::Vec4::new(-0.2552432, 0.96499133, 0.06034819, 0.0),
    glam::Vec4::new(-0.35031125, -0.034122545, -0.93601125, 0.0),
    glam::Vec4::new(-0.9011841, -0.26005128, 0.34675694, 0.0),
    glam::Vec4::new(-0.036278114, 0.009549916, 0.09118295, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.7308136, -0.4484682, -0.5145755, 0.0),
    glam::vec4(-0.4265967, 0.8885936, -0.1685726, 0.0),
    glam::vec4(0.532848, 0.09632102, -0.8407115, 0.0),
    glam::vec4(0.0000001144409, 0.0000001120567, 0.07303866, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.8947756, -0.404903, 0.1882297, 0.0),
    glam::vec4(-0.3156569, 0.871751, 0.3747147, 0.0),
    glam::vec4(-0.3158124, 0.2758695, -0.9078318, 0.0),
    glam::vec4(-0.00000009536743, -0.0000001001358, -0.07303863, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.6872873, 0.7263857, -0.00000002980232, 0.0),
    glam::vec4(-0.7263858, 0.6872874, -0.00000004470348, 0.0),
    glam::vec4(-0.00000001198921, 0.00000005237211, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.0, -0.00000001907349, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.5203388, 0.8539597, -0.00000002980232, 0.0),
    glam::vec4(-0.8539599, 0.520339, 0.00000004470348, 0.0),
    glam::vec4(0.0000000536823, 0.00000000218903, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.0, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8666432, -0.1439794, -0.4777021, 0.0),
    glam::vec4(0.1341059, 0.9894436, -0.0549246, 0.0),
    glam::vec4(0.4805673, -0.01646262, 0.8768032, 0.0),
    glam::vec4(-0.1460184, 0.000000009536743, 0.0, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.7690578, -0.5080044, 0.3879197, 0.0),
    glam::vec4(0.5303457, 0.8459068, 0.05634656, 0.0),
    glam::vec4(-0.3567683, 0.1623978, 0.9199692, 0.0),
    glam::vec4(-0.1460184, -0.000000002384186, 0.00000001907349, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9015524, -0.1388295, -0.4097926, 0.0),
    glam::vec4(-0.1548804, 0.9879148, 0.006054447, 0.0),
    glam::vec4(0.4039996, 0.06892724, -0.9121588, 0.0),
    glam::vec4(0.00000009536743, 0.0000001049042, 0.07303865, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9434555, -0.05121048, 0.3275205, 0.0),
    glam::vec4(0.04168735, 0.9618244, 0.2704736, 0.0),
    glam::vec4(-0.3288683, 0.2688332, -0.9053037, 0.0),
    glam::vec4(-0.00000009536743, -0.00000008106232, -0.07303865, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.9123125, 0.4094951, 0.0000000596046, 0.0),
    glam::vec4(-0.409495, 0.9123124, -0.00000003725293, 0.0),
    glam::vec4(-0.0000000696329, 0.000000009578525, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.00000001907349, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8934721, 0.4491183, -0.00000008940697, 0.0),
    glam::vec4(-0.4491183, 0.8934721, -0.00000005215403, 0.0),
    glam::vec4(0.00000005645932, 0.00000008675251, 0.9999999, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.0, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.9209527, -0.08810819, -0.3795825, 0.0),
    glam::vec4(0.0576691, 0.9941931, -0.09085254, 0.0),
    glam::vec4(0.3853832, 0.06178072, 0.9206861, 0.0),
    glam::vec4(-0.1460184, 0.000000009536743, -0.00000003814697, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8697779, -0.3247607, 0.3715064, 0.0),
    glam::vec4(0.3339412, 0.9416858, 0.04136641, 0.0),
    glam::vec4(-0.3632765, 0.0880817, 0.9275085, 0.0),
    glam::vec4(-0.1460184, 0.000000004768371, 0.0, 1.0),
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
