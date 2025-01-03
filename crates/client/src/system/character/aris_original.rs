use std::sync::Arc;

use constcat::concat;
use hecs::{Entity, QueryOneError, World};
use mod_app::asset::AssetManager;

use crate::{
    asset::MotionPool,
    component::{AnimationState, AnimationTimer, BoneCollection, SkinningAnimation, ToParentTrans},
    system::{IDLE_ANIMATION_SUFFIX, MOVE_TO_END_ANIMATION_SUFFIX, MOVING_ANIMATION_SUFFIX},
};

const IDLE_ANIMATION: &'static str = concat!(MOTION_NAME, IDLE_ANIMATION_SUFFIX);
const MOVING_ANIMATION: &'static str = concat!(MOTION_NAME, MOVING_ANIMATION_SUFFIX);
const MOVE_TO_END_ANIMATION: &'static str = concat!(MOTION_NAME, MOVE_TO_END_ANIMATION_SUFFIX);

const WORKSPACE: &'static str = "characters/aris_original";
const MOTION_NAME: &'static str = "Aris_Original";

/// `Aris Original` 캐릭터 모델의 애니메이션 타이머를 갱신합니다.
pub fn update_aris_original_animation_timer(
    asset_manager: &AssetManager,
    timer: &mut AnimationTimer,
    state: &mut AnimationState,
    elapsed_time_sec: f32,
) {
    let update_animation_timer_fn = match state {
        AnimationState::Idle => update_animation_timer_when_idle_state,
        AnimationState::Moving => update_animation_timer_when_moving_state,
        AnimationState::MoveToEnd => update_animation_timer_when_move_to_end_state,
    };

    update_animation_timer_fn(asset_manager, timer, state, elapsed_time_sec);
}

/// `AnimationState::Idle`일 때 애니메이션 타이머를 갱신합니다.
fn update_animation_timer_when_idle_state(
    asset_manager: &AssetManager,
    timer: &mut AnimationTimer,
    state: &mut AnimationState,
    elapsed_time_sec: f32,
) {
    debug_assert_eq!(*state, AnimationState::Idle);

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MOTION_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(IDLE_ANIMATION).unwrap();

    // 타이머를 갱신합니다.
    timer.0 = (timer.0 + elapsed_time_sec) % character_motion.length;
}

/// `AnimationState::Moving`일 때 애니메이션 타이머를 갱신합니다.
fn update_animation_timer_when_moving_state(
    asset_manager: &AssetManager,
    timer: &mut AnimationTimer,
    state: &mut AnimationState,
    elapsed_time_sec: f32,
) {
    debug_assert_eq!(*state, AnimationState::Moving);

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MOTION_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(MOVING_ANIMATION).unwrap();

    // 타이머를 갱신합니다.
    timer.0 = (timer.0 + elapsed_time_sec) % character_motion.length;
}

/// `AnimationState::MoveToEnd`일 때 애니메이션 타이머를 갱신합니다.
fn update_animation_timer_when_move_to_end_state(
    asset_manager: &AssetManager,
    timer: &mut AnimationTimer,
    state: &mut AnimationState,
    elapsed_time_sec: f32,
) {
    debug_assert_eq!(*state, AnimationState::MoveToEnd);

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MOTION_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(MOVE_TO_END_ANIMATION).unwrap();

    // 타이머를 갱신합니다.
    timer.0 = timer.0 + elapsed_time_sec;
    let diff_t = timer.0 - character_motion.length;
    if diff_t >= 0.0 {
        *state = AnimationState::Idle;
        timer.0 = diff_t;
    }
}

/// `Aris Original` 캐릭터 모델의 애니메이션을 갱신합니다.
///
/// # Panics
/// - 주어진 스키닝 애니메이션(`Arc<SkinningAnimation>`)을 구성하는 엔터티는 유효해야 합니다.
/// 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 스키닝 애니메이션을 구성하는 엔터티는 뼈 모음(`BoneCollection`)과 로컬 변환 행렬(`ToParentTrans`)를
/// 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_aris_original_animation(
    asset_manager: &AssetManager,
    world: &mut World,
    entity: Entity,
) {
    let query = world.query_one_mut::<&AnimationState>(entity).cloned();
    let animation_name = match query {
        Ok(state) => match state {
            AnimationState::Idle => IDLE_ANIMATION,
            AnimationState::Moving => MOVING_ANIMATION,
            AnimationState::MoveToEnd => MOVE_TO_END_ANIMATION,
        },
        Err(e) => match e {
            QueryOneError::NoSuchEntity => panic!("invalid entity"),
            QueryOneError::Unsatisfied => panic!("invalid entity component"),
        },
    };

    let query = world.query_one_mut::<&AnimationTimer>(entity).cloned();
    let time_point = match query {
        Ok(timer) => timer.0,
        Err(e) => match e {
            QueryOneError::NoSuchEntity => panic!("invalid entity"),
            QueryOneError::Unsatisfied => panic!("invalid entity component"),
        },
    };

    let query = world
        .query_one_mut::<&Arc<SkinningAnimation>>(entity)
        .cloned();
    let skinning = match query {
        Ok(skinning) => skinning,
        Err(e) => match e {
            QueryOneError::NoSuchEntity => panic!("invalid entity"),
            QueryOneError::Unsatisfied => panic!("invalid entity component"),
        },
    };

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MOTION_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(animation_name).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let keyframe = character_motion.linear_sampling(time_point);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    {
        let query = world.query_one_mut::<&mut ToParentTrans>(skinning.root);
        let local_transform = match query {
            Ok(local_transform) => local_transform,
            Err(e) => match e {
                QueryOneError::NoSuchEntity => panic!("invalid entity"),
                QueryOneError::Unsatisfied => panic!("invalid entity component"),
            },
        };
        local_transform.0 = keyframe.root_matrix;
    }

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let query = world.query_one_mut::<&Arc<BoneCollection>>(entity).cloned();
        let bone_collection = match query {
            Ok(bone_collection) => bone_collection,
            Err(e) => match e {
                QueryOneError::NoSuchEntity => panic!("invalid entity"),
                QueryOneError::Unsatisfied => panic!("invalid entity component"),
            },
        };

        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let query = world.query_one_mut::<&mut ToParentTrans>(bone_entity);
            let local_transform = match query {
                Ok(local_transform) => local_transform,
                Err(e) => match e {
                    QueryOneError::NoSuchEntity => panic!("invalid entity"),
                    QueryOneError::Unsatisfied => panic!("invalid entity component"),
                },
            };
            local_transform.0 = bone_transform;
        }
    }
}
