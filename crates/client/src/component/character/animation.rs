#![allow(dead_code)]
use ahash::{HashMap, HashSet};
use hecs::{Entity, ViewBorrow, World};
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, LatLon, MovementState, MovementStateTimer,
};

use crate::{
    asset::{MotionPool, CHARACTER_URIS},
    component::{
        Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8, Player9,
        PlayerArchetype, ToParentTrans,
    },
};

use super::*;

/// 모든 캐릭터 모델의 최상위 뼈 노드 이름입니다.
pub const MODEL_BONE_ROOT: &'static str = "Bip001";
/// 모든 캐릭터 모델의 머리 뼈 노드 이름입니다.
pub const MODEL_BONE_HEAD: &'static str = "Bip001_Head";
/// 모든 캐릭터 모델의 골반 뼈 노드 이름입니다.
pub const MODEL_BONE_PELVIS: &'static str = "Bip001_Pelvis";
/// 모든 캐릭터 모델의 아래 척추 뼈 노드 이름입니다.
pub const MODEL_BONE_SPINE: &'static str = "Bip001_Spine";
/// 모든 캐릭터 모델의 윗 척추 뼈 노드 이름입니다.
pub const MODEL_BONE_SPINE_1: &'static str = "Bip001_Spine1";

// /// 모든 캐릭터 모델의 왼쪽 쇄골 뼈 노드 이름입니다.
// pub const MODEL_BONE_L_CLAVICLE: &'static str = "Bip001_L_Clavicle";
// /// 모든 캐릭터 모델의 왼쪽 윗팔 뼈 노드 이름입니다.
// pub const MODEL_BONE_L_UPPERARM: &'static str = "Bip001_L_UpperArm";
// /// 모든 캐릭터 모델의 왼쪽 아래팔 뼈 노드 이름입니다.
// pub const MODEL_BONE_L_FOREARM: &'static str = "Bip001_L_Forearm";
// /// 모든 캐릭터 모델의 왼쪽 손 뼈 노드 이름입니다.
// pub const MODEL_BONE_L_HAND: &'static str = "Bip001_L_Hand";

/// 모든 캐릭터 모델의 오른쪽 쇄골 뼈 노드 이름입니다.
pub const MODEL_BONE_R_CLAVICLE: &'static str = "Bip001_R_Clavicle";
/// 모든 캐릭터 모델의 오른쪽 윗팔 뼈 노드 이름입니다.
pub const MODEL_BONE_R_UPPERARM: &'static str = "Bip001_R_UpperArm";
/// 모든 캐릭터 모델의 오른쪽 아래팔 뼈 노드 이름입니다.
pub const MODEL_BONE_R_FOREARM: &'static str = "Bip001_R_Forearm";
/// 모든 캐릭터 모델의 오른쪽 손 뼈 노드 이름입니다.
pub const MODEL_BONE_R_HAND: &'static str = "Bip001_R_Hand";

/// 모든 캐릭터 모델의 왼쪽 허벅지 안쪽 뼈 노드 이름입니다.
pub const MODEL_BONE_L_THIGH: &'static str = "Bip001_L_Thigh";
/// 모든 캐릭터 모델의 오른쪽 허벅지 안쪽 뼈 노드 이름입니다.
pub const MODEL_BONE_R_THIGH: &'static str = "Bip001_R_Thigh";
/// 모든 캐릭터 모델의 왼쪽 종아리 뼈 노드 이름입니다.
pub const MODEL_BONE_L_CALF: &'static str = "Bip001_L_Calf";
/// 모든 캐릭터 모델의 오른쪽 종아리 뼈 노드 이름입니다.
pub const MODEL_BONE_R_CALF: &'static str = "Bip001_R_Calf";
/// 모든 캐릭터 모델의 왼쪽 발 뼈 노드 이름입니다.
pub const MODEL_BONE_L_FOOT: &'static str = "Bip001_L_Foot";
/// 모든 캐릭터 모델의 오른쪽 발 뼈 노드 이름입니다.
pub const MODEL_BONE_R_FOOT: &'static str = "Bip001_R_Foot";

/// 모든 캐릭터 모델의 Idle 애니메이션 접미사입니다.
pub const IDLE_ANIMATION_SUFFIX: &'static str = "_Normal_Idle";
/// 모든 캐릭터 모델의 Moving 애니메이션 접미사입니다.
pub const MOVING_ANIMATION_SUFFIX: &'static str = "_Move_Ing";
/// 모든 캐릭터 모델의 MoveToEnd 애니메이션 접미사입니다.
pub const MOVE_TO_END_ANIMATION_SUFFIX: &'static str = "_Move_End_Normal";
/// 모든 캐릭터 모델의 CafeWalk 애니메이션 접미사입니다.
pub const CAFE_WALK_ANIMATION_SUFFIX: &'static str = "_Cafe_Walk";
/// 모든 캐릭터 모델의 AttackStart 애니메이션 접미사입니다.
pub const ATTACK_START_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Start";
/// 모든 캐릭터 모델의 Attacking 애니메이션 접미사입니다.
pub const ATTACK_ING_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Ing";
/// 모든 캐릭터 모델의 AttackEnd 애니메이션 접미사입니다.
pub const ATTACK_END_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_End";
/// 모든 캐릭터 모델의 Reload 애니메이션 접미사입니다.
pub const RELOAD_ANIMATION_SUFFIX: &'static str = "_Normal_Reload";
/// 모든 캐릭터 모델의 Vital_Death 애니메이션 접미사입니다.
pub const VITAL_DEATH_ANIMATION_SUFFIX: &'static str = "_Vital_Death";
/// 모든 캐릭터 모델의 Normal_Callsign 애니메이션 접미사입니다.
pub const NORMAL_CALLSIGN_SUFFIX: &'static str = "_Normal_Callsign";
/// 모든 캐릭터 모델의 Victory_Start 애니메이션 접미사입니다.
pub const VICTORY_START_SUFFIX: &'static str = "_Victory_Start";
/// 모든 캐릭터 모델의 Victory_End 애니메이션 접미사입니다.
pub const VICTORY_END_SUFFIX: &'static str = "_Victory_End";
/// 모든 캐릭터 모델의 Public01 애니메이션 접미사입니다.
pub const PUBLIC01_SUFFIX: &'static str = "_Public01";
// /// 모든 캐릭터 모델의 Ex스킬 애니메이션 접미사입니다.
pub const EXS_ANIMATION_SUFFIX: &'static str = "_Exs";
/// 모든 캐릭터 모델의 Formation_Idle 애니메이션 접미사입니다.
pub const FORMATION_IDLE: &'static str = "_Formation_Idle";
/// 모든 캐릭터 모델의 Formation_Pickup 애니메이션 접미사입니다.
pub const FORMATION_PICKUP: &'static str = "_Formation_Pickup";

/// ## Skinning Animation
/// 스키닝 애니메이션에 사용되는 스키닝 메쉬 엔터티와 최상위 뼈 노드 엔터티의 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinningAnimation {
    /// NOTE: `BoneCollection`의 `root`와 다름!
    pub entity_list: HashMap<String, Entity>,
    pub mesh_entity_list: HashMap<String, Entity>,
    pub mixing_bone_list: HashSet<Entity>,
}

impl Default for SkinningAnimation {
    fn default() -> Self {
        Self {
            entity_list: HashMap::default(),
            mesh_entity_list: HashMap::default(),
            mixing_bone_list: HashSet::default(),
        }
    }
}

/// ## Bone Collection
/// 스키닝된 메쉬를 구성하는 뼈의 엔터티 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneCollection {
    /// NOTE: 스키닝된 메쉬의 최상위 뼈를 나타냅니다.
    pub root: Entity,
    pub bones: Vec<Entity>,
}

/// 캐릭터 애니메이션을 재생합니다.
pub fn animate_character(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    motion_pool: &MotionPool,
    action_state: ActionState,
    movement_state: MovementState,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    latlon: LatLon,
    character_view: &ViewBorrow<&CharacterKind>,
    skinning_view: &ViewBorrow<&SkinningAnimation>,
    collection_view: &ViewBorrow<&BoneCollection>,
) {
    // 캐릭터 종류를 가져옵니다.
    let &character_kind = character_view
        .get(entity)
        .expect("invalid entity or invalid entity component!");
    // 스키닝 애니메이션 데이터를 가져옵니다.
    let skinning_animation = skinning_view
        .get(entity)
        .expect("invalid entity or invalid entity component!");

    // 캐릭터 애니메이션 데이터를 가져옵니다.
    let i = character_kind as usize;
    let motions = motion_pool
        .get(CHARACTER_URIS[i])
        .expect("no such animation data!");

    match archetype {
        PlayerArchetype::Player0 => {
            type Tag = Player0;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player1 => {
            type Tag = Player1;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player2 => {
            type Tag = Player2;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player3 => {
            type Tag = Player3;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player4 => {
            type Tag = Player4;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player5 => {
            type Tag = Player5;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player6 => {
            type Tag = Player6;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player7 => {
            type Tag = Player7;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player8 => {
            type Tag = Player8;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player9 => {
            type Tag = Player9;
            let mut transform_view = world.view::<&mut (Tag, ToParentTrans)>();
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::animate_character::<Tag>,
                CharacterKind::MomoiOriginal => momoi_original::animate_character::<Tag>,
                CharacterKind::MidoriOriginal => midori_original::animate_character::<Tag>,
                CharacterKind::YuukaOriginal => yuuka_original::animate_character::<Tag>,
            };
            func(
                &motions,
                skinning_animation,
                action_state,
                movement_state,
                action_state_timer,
                movement_state_timer,
                latlon,
                collection_view,
                &mut transform_view,
            );
        }
    }
}
