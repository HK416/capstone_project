mod terrain;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;

use crate::asset::ModelAssetError;

use super::{Child, ToParentTrans, WorldTransform};

use self::terrain::*;

/// ## Terrain Tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainTag;

/// 지형을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 지형 태그(`Terrain`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spawn_terrain(
    name: &str,
    workspace: &str,
    scale: glam::Vec3,
    rotation: glam::Quat,
    translation: glam::Vec3,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let tag = TerrainTag;
    let local_transform = ToParentTrans(glam::Mat4::from_scale_rotation_translation(
        scale,
        rotation,
        translation,
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add(tag);
    builder.add(local_transform);
    builder.add(world_transform);

    // 지형 모델을 구성하는 엔터티를 생성합니다.
    let (model_root_entity, mut batch_commands) =
        spawn_terrain_model(name, workspace, asset_manager, device, queue, world, entity)?;

    // 자식 엔터티를 추가합니다.
    builder.add(Child(model_root_entity));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
}
