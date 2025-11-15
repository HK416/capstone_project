//! `Aris_Original` 모델과 관련된 코드를 관리합니다.
//!

mod animation;
mod camera;
mod spawn;

use hecs::{Component, ViewBorrow};
use lazy_static::lazy_static;
use mod_network::components::{ActionState, CharacterAttributes};

use crate::component::{
    Child, MODEL_BONE_R_HAND, Sibling, SkinningAnimation, ToParentTrans, WorldTransform,
    update_entity_hierarchy_with_archetype,
};

pub use self::{animation::*, camera::*, spawn::*};

use super::look_to_camera_direction;

/// 캐릭터 모델의 이름입니다.
pub const MODEL_NAME: &'static str = "Aris_Original";
/// 캐릭터 모델의 무기 뼈 노드 이름입니다.
const MODEL_BONE_WEAPON: &'static str = "Bip001_Weapon";

lazy_static! {
    pub static ref CHARACTER_ATTRIBUTE: CharacterAttributes = {
        let json = include_str!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/characters/aris_original/attribute.json"
        ));
        serde_json::from_str(json).unwrap()
    };
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
                let bone_entity = skinning_animation
                    .entity_list
                    .get(MODEL_BONE_R_HAND)
                    .cloned()
                    .expect("the bone entity must be exists!");
                let (_, world_transform) = world_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component!");
                let w_hand = world_transform.0;

                let bone_entity = skinning_animation
                    .entity_list
                    .get(MODEL_BONE_WEAPON)
                    .cloned()
                    .expect("the bone entity must be exists!");
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
        }
        _ => {}
    }
}
