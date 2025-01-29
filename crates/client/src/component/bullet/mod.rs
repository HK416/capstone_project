mod aris_original;
mod common;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_network::components::Bullet;

use crate::{
    asset::ModelAssetError,
    component::{Child, ToParentTrans, WorldTransform},
};

use self::{aris_original::*, common::*};

const NUM_BULLETS: usize = 2;

/// ## Bullet Tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BulletTag;

/// 총알을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 총알 종류(`BulletKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - 총알 태그(`BulletTag`)
///
pub fn spwan_bullet(
    bullet: &Bullet,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    type SpawnFn = fn(
        &AssetManager,
        &wgpu::Device,
        &wgpu::Queue,
        &World,
        Entity,
    ) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError>;
    const FUNC_TABLE: [SpawnFn; NUM_BULLETS] =
        [spawn_common_bullet_model, spawn_aris_original_bullet_model];

    // 엔터티를 하나 할당 받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let bullet_kind = bullet.bullet_kind;
    let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
        glam::Quat::from_array(bullet.rotation),
        glam::Vec3::from_array(bullet.translation),
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add(bullet_kind);
    builder.add(local_transform);
    builder.add(world_transform);

    // 총알 종류에 따른 총알 모델을 구성하는 엔터티를 생성합니다.
    let i = bullet_kind as usize;
    let parent = entity;
    let (model_root_entity, mut batch_commands) =
        FUNC_TABLE[i](asset_manager, device, queue, world, parent)?;

    // 총알 모델 루트 노드를 추가합니다.
    builder.add(Child(model_root_entity));

    // 엔터티 생성 명령어를 추가하빈다.
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
}
