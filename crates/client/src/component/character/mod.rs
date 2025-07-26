mod animation;
mod camera;
mod pipeline;
mod render;
mod spawn;

mod aris_original;
mod midori_original;
mod momoi_original;
mod yuuka_original;

use hecs::{Component, Entity, ViewBorrow, World};
use lazy_static::lazy_static;
use mod_network::components::{
    ActionState, CharacterAttributes, CharacterKind, LatLon, NUM_CHARACTERS,
};

use crate::{
    asset::MeshPool,
    component::{Child, PlayerArchetype, Sibling, ToParentTrans, WorldTransform},
};

pub use self::{animation::*, camera::*, pipeline::*, render::*, spawn::*};

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
    let bone_entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_HEAD)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let axis = character_attributes.attack_head_axis;
    local_transform.0 *= glam::Mat4::from_axis_angle(axis.into(), angle);

    // Spine1
    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_SPINE_1)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let axis = character_attributes.attack_spine1_axis;
    local_transform.0 *= glam::Mat4::from_axis_angle(axis.into(), angle);

    // Spine
    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation
        .entity_list
        .get(MODEL_BONE_SPINE)
        .cloned()
        .expect("the bone entity must be exists!");
    let (_, local_transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let axis = character_attributes.attack_spine_axis;
    local_transform.0 *= glam::Mat4::from_axis_angle(axis.into(), angle);
}

/// 캐릭터 무기의 위치를 설정합니다.
pub fn set_weapon_position<Tag: Copy + Component>(
    action_state: ActionState,
    character_kind: CharacterKind,
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
            let func = match character_kind {
                CharacterKind::ArisOriginal => aris_original::set_weapon_position,
                CharacterKind::MomoiOriginal => momoi_original::set_weapon_position,
                CharacterKind::MidoriOriginal => midori_original::set_weapon_position,
                CharacterKind::YuukaOriginal => yuuka_original::set_weapon_position,
            };
            func(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                local_transform_view,
                world_transform_view,
            );
        }
        _ => {}
    }
}

/// 총구 화염 이펙트 엔터티를 생성합니다.
pub fn spawn_fx_muzzle_effect(
    world: &mut World,
    entity: Entity,
    archetype: PlayerArchetype,
    mesh_pool: &MeshPool,
) {
    // 캐릭터 종류를 가져옵니다.
    let &kind = world
        .query_one_mut::<&CharacterKind>(entity)
        .expect("invalid entity or invalid entity component!");

    // 스키닝 애니메이션 데이터를 가져옵니다.
    let skinning_animation = world
        .query_one_mut::<&SkinningAnimation>(entity)
        .expect("invalid entity or invalid entity component!");

    let builders = match kind {
        CharacterKind::ArisOriginal => {
            vec![]
        }
        CharacterKind::MomoiOriginal => {
            momoi_original::spawn_fx_muzzle_effect(archetype, skinning_animation, mesh_pool)
        }
        CharacterKind::MidoriOriginal => {
            midori_original::spawn_fx_muzzle_effect(archetype, skinning_animation, mesh_pool)
        }
        CharacterKind::YuukaOriginal => {
            yuuka_original::spawn_fx_muzzle_effect(archetype, skinning_animation, mesh_pool)
        }
    };

    // 엔터티를 생성합니다.
    for mut builder in builders {
        world.spawn(builder.build());
    }
}

/// Midori_Original의 총구 화염 이펙트 엔터티를 생성합니다.
pub fn spawn_midori_fx_muzzle_effect(
    world: &mut World,
    entity: Entity,
    archetype: PlayerArchetype,
    mesh_pool: &MeshPool,
) {
    // 스키닝 애니메이션 데이터를 가져옵니다.
    let skinning_animation = world
        .query_one_mut::<&SkinningAnimation>(entity)
        .expect("invalid entity or invalid entity component!");

    let builders =
        midori_original::spawn_skill_fx_muzzle_effect(archetype, skinning_animation, mesh_pool);

    // 엔터티를 생성합니다.
    for mut builder in builders {
        world.spawn(builder.build());
    }
}

/// Momoi_Original의 총구 화염 이펙트 엔터티를 생성합니다.
pub fn spawn_momoi_fx_muzzle_effect(
    world: &mut World,
    entity: Entity,
    archetype: PlayerArchetype,
    mesh_pool: &MeshPool,
) {
    // 스키닝 애니메이션 데이터를 가져옵니다.
    let skinning_animation = world
        .query_one_mut::<&SkinningAnimation>(entity)
        .expect("invalid entity or invalid entity component!");

    let builders =
        momoi_original::spawn_skill_fx_muzzle_effect(archetype, skinning_animation, mesh_pool);

    // 엔터티를 생성합니다.
    for mut builder in builders {
        world.spawn(builder.build());
    }
}
