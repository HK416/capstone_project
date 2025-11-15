//! 캐릭터 모델의 `ReloadJumping` 애니메이션과 관련된 코드를 관리합니다.
//!

use ahash::HashMap;
use hecs::{Component, ViewBorrow};
use mod_network::components::{ActionStateTimer, CharacterAttributes, LatLon, MovementStateTimer};

use crate::{
    asset::Motion,
    component::{BoneCollection, MODEL_BONE_ROOT, SkinningAnimation, ToParentTrans},
};

use super::*;

/// "*_Normal_Reload"와 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티가 유효하지 않은 경우 [`panic!`]을 호출합니다.
/// - 엔터티 컴포넌트 데이터가 스레드에 안전하지 않은 경우 [`panic!`]을 호출합니다.
///
pub fn animate_character_when_reload_jumping<Tag: Copy + Component>(
    motions: &HashMap<String, Motion>,
    skinning_animation: &SkinningAnimation,
    character_attributes: &CharacterAttributes,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    latlon: LatLon,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    // "*_Normal_Reload" 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such animation data!");

    // 애니메이션 키 프레임을 샘플링합니다.
    let time_point_0 = action_state_timer
        .0
        .min(character_attributes.normal_reload_duration);
    let keyframe = motion.linear_sampling(time_point_0);

    // 최상위 엔터티의 로컬 변환 행렬을 갱신합니다.
    let root_entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_ROOT)
        .cloned()
        .expect("the bone entity must exists!");
    let (_, local_transform) = transform_view
        .get_mut(root_entity)
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

    jump_animation(skinning_animation, movement_state_timer, transform_view);

    let offset = 0.0;
    look_to_camera_direction(
        offset,
        latlon,
        character_attributes,
        skinning_animation,
        transform_view,
    );
}
