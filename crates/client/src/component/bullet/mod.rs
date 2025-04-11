//! 총알 객체와 관련된 코드를 관리합니다.
//!

mod common;
mod energy;

use hecs::{Entity, EntityBuilder, World};
use mod_network::components::{Bullet, NUM_BULLETS};

use crate::{
    asset::{ModelPool, ModelRoot, SamplerPool, TextureViewPool, BULLET_URIS},
    component::{Child, ToParentTrans, WorldTransform},
};

pub use self::{common::*, energy::*};

/// 총알을 구성하는 엔터티를 생성합니다.
///
/// 생성된 최상위 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 총알 종류(`BulletKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spwan_bullet(
    world: &World,
    model_pool: &ModelPool,
    texture_view_pool: &TextureViewPool,
    sampler_pool: &SamplerPool,
    bullet: &Bullet,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    type Func = fn(
        Option<&str>,
        &TextureViewPool,
        &SamplerPool,
        &wgpu::Device,
        &mut wgpu::CommandEncoder,
        &mut Vec<wgpu::Buffer>,
        &World,
        Entity,
        &ModelRoot,
    ) -> (Entity, Vec<(Entity, EntityBuilder)>);
    const FUNC_TABLE: [Func; NUM_BULLETS] = [spawn_common_bullet_model, spawn_energy_bullet_model];

    // 모델 풀 객체에서 총알 모델 노드를 가져옵니다.
    let i = bullet.bullet_kind as usize;
    let root = model_pool
        .get(BULLET_URIS[i])
        .expect("the bullet model must exist!");

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
    builder.add_bundle((bullet_kind, local_transform, world_transform));

    // 총알 종류에 따른 총알 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = FUNC_TABLE[i](
        Some(&format!("Bullet({})", bullet.object_id)),
        texture_view_pool,
        sampler_pool,
        device,
        encoder,
        staging_buffers,
        world,
        entity,
        &root,
    );

    // 총알 모델 루트 노드를 추가합니다.
    builder.add(Child(child));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    (entity, batch_commands)
}
