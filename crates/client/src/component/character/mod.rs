mod animation;
mod camera;
mod pipeline;
mod pull;
mod render;
mod snapshot;
mod spawn;
mod transform;
mod view_state;

mod aris_original;
mod midori_original;
mod momoi_original;
mod yuuka_original;

use hecs::{Component, ViewBorrow};
use lazy_static::lazy_static;
use mod_network::components::{ActionState, CharacterAttributes, LatLon, NUM_CHARACTERS};

use crate::component::{Child, Sibling, ToParentTrans, WorldTransform};

pub use self::{
    animation::*, camera::*, pipeline::*, pull::*, render::*, snapshot::*, spawn::*, transform::*,
    view_state::*,
};

lazy_static! {
    pub static ref CHARACTER_ATTRIBUTES: [&'static CharacterAttributes; NUM_CHARACTERS] = [
        &aris_original::CHARACTER_ATTRIBUTE,
        &momoi_original::CHARACTER_ATTRIBUTE,
        &midori_original::CHARACTER_ATTRIBUTE,
        &yuuka_original::CHARACTER_ATTRIBUTE,
    ];
}

/// 캐릭터가 카메라가 바라보는 방향을 바라보도록 로컬 변환 행렬을 수정합니다.
fn look_to_camera_direction<Tag: Copy + Component>(
    offset: f32,
    latlon: LatLon,
    character_attributes: &CharacterAttributes,
    skinning_animation: &SkinningAnimation,
    transform_view: &mut ViewBorrow<&mut (Tag, ToParentTrans)>,
) {
    let latitude = latlon.lat + 3f32.to_radians();

    // Head
    let angle = latitude / 7.0 * offset;
    let bone_entity = skinning_animation.bip001_head;
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let axis = character_attributes.attack_head_axis;
    local_transform.0 *= glam::Mat4::from_axis_angle(axis.into(), angle);

    // Spine1
    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation.bip001_spine1;
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let axis = character_attributes.attack_spine1_axis;
    local_transform.0 *= glam::Mat4::from_axis_angle(axis.into(), angle);

    // Spine
    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation.bip001_spine;
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let axis = character_attributes.attack_spine_axis;
    local_transform.0 *= glam::Mat4::from_axis_angle(axis.into(), angle);
}

/// 캐릭터 무기의 위치를 설정합니다.
pub fn set_weapon_position<Tag: Copy + Component>(
    action_state: ActionState,
    character_attributes: &CharacterAttributes,
    skinning_animation: &SkinningAnimation,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    local_transform_view: &mut ViewBorrow<&(Tag, ToParentTrans)>,
    world_transform_view: &mut ViewBorrow<&mut (Tag, WorldTransform)>,
) {
    match action_state {
        ActionState::Aiming
        | ActionState::AimAt
        | ActionState::AimOff
        | ActionState::Attack
        | ActionState::Skill => {
            if let Some(attributes) = &character_attributes.right_weapon {
                let bone_entity = skinning_animation.bip001_r_hand;
                let (_, world_transform) = world_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component!");
                let w_hand = world_transform.0;

                let bone_entity = skinning_animation.bip001_r_weapon;
                let (_, world_transform) = world_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component!");
                let offset: glam::Mat4 = attributes.hand_to_weapon_offset.into();
                let parent = w_hand * offset;
                world_transform.0 = parent;

                if let Some(&child) = child_view.get(bone_entity) {
                    update_entity_hierarchy_with_archetype(
                        *child,
                        parent,
                        child_view,
                        sibling_view,
                        local_transform_view,
                        world_transform_view,
                    );
                }
            }

            if let Some(attributes) = &character_attributes.left_weapon {
                let bone_entity = skinning_animation.bip001_l_hand;
                let (_, world_transform) = world_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component!");
                let transform = world_transform.0;

                let bone_entity = skinning_animation.bip001_l_weapon;
                let (_, world_transform) = world_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component!");
                let offset: glam::Mat4 = attributes.hand_to_weapon_offset.into();
                let parent = transform * offset;
                world_transform.0 = parent;

                if let Some(&child) = child_view.get(bone_entity) {
                    update_entity_hierarchy_with_archetype(
                        *child,
                        parent,
                        child_view,
                        sibling_view,
                        local_transform_view,
                        world_transform_view,
                    );
                }
            }
        }
        _ => {}
    }
}
