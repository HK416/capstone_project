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
        BoneCollection, SkinningAnimation, ToParentTrans, ATTACK_END_ANIMATION_SUFFIX,
        ATTACK_ING_ANIMATION_SUFFIX, ATTACK_START_ANIMATION_SUFFIX, CAFE_WALK_ANIMATION_SUFFIX,
        EXS_ANIMATION_SUFFIX, IDLE_ANIMATION_SUFFIX, MODEL_BONE_L_CALF, MODEL_BONE_L_FOOT,
        MODEL_BONE_L_THIGH, MODEL_BONE_R_CALF, MODEL_BONE_R_FOOT, MODEL_BONE_R_THIGH,
        MOVE_TO_END_ANIMATION_SUFFIX, MOVING_ANIMATION_SUFFIX, NORMAL_CALLSIGN_SUFFIX,
        PUBLIC01_SUFFIX, RELOAD_ANIMATION_SUFFIX, VICTORY_END_SUFFIX, VICTORY_START_SUFFIX,
        VITAL_DEATH_ANIMATION_SUFFIX,
    },
};

use self::{
    aim::*, aim_jumping::*, aim_landing::*, aim_move::*, aim_move_to_move::*, aim_to_idle::*,
    attack_jumping::*, attack_landing::*, attack_move::*, attacking::*, callsign::*, death::*,
    idle::*, idle_to_aim::*, jumping::*, landing::*, move_to_aim_move::*, move_to_end::*,
    moving::*, reload::*, reload_jumping::*, reload_landing::*, reload_move::*, skill::*,
    skill_jumping::*, skill_landing::*, skill_move::*, victory_end::*, victory_start::*,
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
const SKILL_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, PUBLIC01_SUFFIX);
/// 캐릭터의 Callsign 애니메이션 이름입니다.
const NORMAL_CALLSIGN_ANIMATION: &'static str =
    constcat::concat!(MODEL_NAME, NORMAL_CALLSIGN_SUFFIX);
/// 캐릭터의 *_Victory_Start 애니메이션 이름입니다.
const VICTORY_START_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, VICTORY_START_SUFFIX);
/// 캐릭터의 *_Victory_End 애니메이션 이름입니다.
const VICTORY_END_ANIMATION: &'static str = constcat::concat!(MODEL_NAME, VICTORY_END_SUFFIX);

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

/// 점프 애니메이션을 적용합니다.
fn jump_animation<Tag: Copy + Component>(
    skinning_animation: &SkinningAnimation,
    movement_state_timer: MovementStateTimer,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let t = movement_state_timer.0.min(MAX_JUMP_DURATION) as f32 / MAX_JUMP_DURATION as f32;

    let angle = -25f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_L_THIGH)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let angle = 10f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_R_THIGH)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_L_CALF)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_ATTACKING_IDENTITY;

    let angle = 60f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_R_CALF)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_L_FOOT)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_ATTACKING_IDENTITY;

    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_R_FOOT)
        .cloned()
        .expect("the bone entity must be exists!");
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
    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_L_THIGH)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let angle = 10f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_R_THIGH)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_L_CALF)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_ATTACKING_IDENTITY;

    let angle = 60f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_R_CALF)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_L_FOOT)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_ATTACKING_IDENTITY;

    let entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_R_FOOT)
        .cloned()
        .expect("the bone entity must be exists!");
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
        ActionState::Retreat => {
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
