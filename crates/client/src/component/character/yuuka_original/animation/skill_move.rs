//! 캐릭터 모델의 `SkillMove` 애니메이션과 관련된 코드를 관리합니다.
//!

use ahash::HashMap;
use hecs::{Component, ViewBorrow};
use mod_network::components::{ActionStateTimer, CharacterAttributes, LatLon, MovementStateTimer};

use crate::{
    asset::Motion,
    component::{BoneCollection, SkinningAnimation, ToParentTrans},
};

use super::*;

/// 스킬 애니메이션과 `*_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티가 유효하지 않은 경우 [`panic!`]을 호출합니다.
/// - 엔터티 컴포넌트 데이터가 스레드에 안전하지 않은 경우 [`panic!`]을 호출합니다.
///
pub fn animate_character_when_skill_move<Tag: Copy + Component>(
    motions: &HashMap<String, Motion>,
    skinning_animation: &SkinningAnimation,
    character_attribute: &CharacterAttributes,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    latlon: LatLon,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    // 스킬 애니메이션을 가져옵니다.
    let motion = motions
        .get(SKILL_ANIMATION)
        .expect("no such animation data!");

    // 애니메이션 키 프레임을 샘플링합니다.
    let time_point_0 = action_state_timer.0.min(character_attribute.skill_duration);
    let keyframe = motion.linear_sampling(time_point_0);

    // 최상위 엔터티의 로컬 변환 행렬을 갱신합니다.
    let (_, local_transform) = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component!");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬를 구성하는 엔터티의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬 엔터티를 가져옵니다.
        let entity = skinning_animation
            .mesh_entity_list
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity!");

        // 스키닝된 메쉬 엔터티를 구성하는 뼈 엔터티 집합을 가져옵니다.
        let collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component!");

        // 뼈 엔터티의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = collection.bones[bone_index];
            let (_, local_transform) = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component!");
            local_transform.0 = bone_transform;
        }
    }

    // "*_Cafe_Walk" 애니메이션을 가져옵니다.
    let motion = motions
        .get(CAFE_WALK_ANIMATION)
        .expect("no such animation data!");

    // 애니메이션 키 프레임을 샘플링합니다.
    let time_point_1 = movement_state_timer.0 % character_attribute.cafe_walk_duration;
    let keyframe = motion.linear_sampling(time_point_1);

    // 키 프레임을 구성하는 스키닝된 메쉬를 구성하는 엔터티의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬 엔터티를 가져옵니다.
        let entity = skinning_animation
            .mesh_entity_list
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity!");

        // 스키닝된 메쉬 엔터티를 구성하는 뼈 엔터티 집합을 가져옵니다.
        let collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component!");

        // 뼈 엔터티의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = collection.bones[bone_index];
            let (_, local_transform) = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component!");
            local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
        }
    }

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, latlon, skinning_animation, transform_view);
}
