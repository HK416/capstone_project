mod aris_original;
mod common;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_network::components::{Bullet, BulletKind};

use crate::{
    asset::{
        AssetError, MeshPool, ModelHierarchyPool, SamplerPool, TextureDataPool, TexturePool,
        TextureViewPool,
    },
    component::{Child, ToParentTrans, WorldTransform},
};

use self::{aris_original::*, common::*};

const NUM_BULLETS: usize = 2;

/// ## Bullet Tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BulletTag;

/// 총알 모델을 풀 객체에 로드합니다.
pub fn load_bullet_model(
    texture_data_pool: &TextureDataPool,
    texture_pool: &TexturePool,
    texture_view_pool: &TextureViewPool,
    sampler_pool: &SamplerPool,
    mesh_pool: &MeshPool,
    asset_manager: &AssetManager,
    bullet_kind: BulletKind,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> Result<(), AssetError> {
    const MODELS: [(&'static str, &'static str); NUM_BULLETS] = [
        (common::WORKSPACE, common::MODEL_NAME),
        (aris_original::WORKSPACE, aris_original::MODEL_NAME),
    ];

    let i = bullet_kind as usize;
    let (workspace, model_name) = MODELS[i];

    // 총알 모델을 로드합니다.
    ModelHierarchyPool::get_or_init(
        texture_data_pool,
        texture_pool,
        texture_view_pool,
        sampler_pool,
        mesh_pool,
        model_name,
        workspace,
        asset_manager,
        device,
        queue,
        encoder,
        staging_buffers,
    )?;

    Ok(())
}

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
    texture_data_pool: &TextureDataPool,
    texture_pool: &TexturePool,
    texture_view_pool: &TextureViewPool,
    sampler_pool: &SamplerPool,
    mesh_pool: &MeshPool,
    bullet: &Bullet,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError> {
    type SpawnFn = fn(
        &TextureDataPool,
        &TexturePool,
        &TextureViewPool,
        &SamplerPool,
        &MeshPool,
        &AssetManager,
        &wgpu::Device,
        &wgpu::Queue,
        &mut wgpu::CommandEncoder,
        &mut Vec<wgpu::Buffer>,
        &World,
        Entity,
    ) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError>;
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
    let (model_root_entity, mut batch_commands) = FUNC_TABLE[i](
        texture_data_pool,
        texture_pool,
        texture_view_pool,
        sampler_pool,
        mesh_pool,
        asset_manager,
        device,
        queue,
        encoder,
        staging_buffers,
        world,
        parent,
    )?;

    // 총알 모델 루트 노드를 추가합니다.
    builder.add(Child(model_root_entity));

    // 엔터티 생성 명령어를 추가하빈다.
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
}
