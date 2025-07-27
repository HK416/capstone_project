//! `Yuuka_Original` 모델과 관련된 코드를 관리합니다.
//!

mod animation;
mod camera;
mod spawn;

use hecs::{Component, EntityBuilder, ViewBorrow};
use lazy_static::lazy_static;
use mod_network::components::{ActionState, CharacterAttributes};

use crate::{
    asset::{MeshPool, FX_TEX_MUZZLE_00},
    component::{
        update_entity_hierarchy_with_archetype, Child, FxMuzzle00, FxMuzzle01, FxMuzzleTintColor,
        LifeTime, Parent, PlayerArchetype, Sibling, SkinningAnimation, ToParentTrans,
        WorldTransform, MODEL_BONE_R_HAND,
    },
};

pub use self::{animation::*, camera::*, spawn::*};

use super::look_to_camera_direction;

/// 캐릭터 모델의 이름입니다.
pub const MODEL_NAME: &'static str = "Yuuka_Original";
/// 캐릭터 모델의 무기 뼈 노드 이름입니다.
const MODEL_BONE_WEAPON: &'static str = "Bip001_Weapon_R";

lazy_static! {
    pub static ref CHARACTER_ATTRIBUTE: CharacterAttributes = {
        let json = include_str!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/characters/yuuka_original/attribute.json"
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

/// 총구 화염 이팩트 파티클을 생성합니다.
pub fn spawn_fx_muzzle_effect(
    archetype: PlayerArchetype,
    skinning_animation: &SkinningAnimation,
    mesh_pool: &MeshPool,
) -> Vec<EntityBuilder> {
    // 사각형 메쉬를 가져옵니다.
    let (mesh, _) = mesh_pool
        .get(FX_TEX_MUZZLE_00)
        .expect("the mesh must be preloaded!");

    // 총구 엔터티를 가져옵니다.
    let muzzle = skinning_animation
        .entity_list
        .get("fire_01")
        .cloned()
        .expect("the muzzle entity must be exists!");

    let parent = Parent(muzzle);
    let life_time = LifeTime::new(32);
    let tint_color = FxMuzzleTintColor([220.0 / 255.0, 36.0 / 255.0, 0.0 / 255.0]);
    let transform = ToParentTrans(glam::Mat4::from_scale(glam::vec3(0.35, 0.35, 0.35)));
    let mut builder_0 = EntityBuilder::new();
    builder_0.add_bundle((
        mesh.clone(),
        parent,
        archetype,
        transform,
        tint_color,
        life_time,
        FxMuzzle00(rand::random_range(0..4)),
    ));

    let transform = ToParentTrans(glam::Mat4::from_scale_rotation_translation(
        glam::Vec3::splat(0.35),
        glam::Quat::from_rotation_y(-90f32.to_radians()),
        glam::vec3(0.0, 0.0, 0.22),
    ));
    let mut builder_1 = EntityBuilder::new();
    builder_1.add_bundle((
        mesh.clone(),
        parent,
        archetype,
        transform,
        tint_color,
        life_time,
        FxMuzzle01(rand::random_range(0..4)),
    ));

    vec![builder_0, builder_1]
}
