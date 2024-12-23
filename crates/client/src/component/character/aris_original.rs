use std::sync::Arc;

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, NoSuchEntity, World};
use mod_app::asset::AssetManager;
use mod_render::{MeshResource, SkinningDataLayout};

use crate::{
    asset::{ModelAssetError, ModelHierarchyPool, Node},
    component::{
        BoneCollection, Child, Parent, Sibling, SkinningAnimation, ToParentTrans, WorldTransform,
    },
};

use super::{Character, CharacterHalo, MODEL_BONE_ROOT};

const WORKSPACE: &'static str = "characters/aris_original";
const MODEL_NAME: &'static str = "Aris_Original";
const MODEL_HALO_NAME: &'static str = "Aris_Original_Halo";

/// `aris_original` 캐릭터 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 다음 컴포넌트를 기본으로 가집니다.
/// - `Parent`
/// - `ToParentTrans`
/// - `WorldTransform`
///
/// 생성된 엔터티는 선택적으로 다음 컴포넌트를 가집니다.
/// - `Arc<Mesh>`
/// - `Arc<MeshResource>`
/// - `Arc<BoneCollection>`
/// - `Child`
/// - `Sibling`
/// - `Vec<Arc<MaterialResource>>`
///
pub(super) fn spawn_aris_original_model(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    parent: Entity,
) -> Result<(Entity, Arc<SkinningAnimation>, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let root =
        ModelHierarchyPool::get_or_init(&MODEL_NAME, &WORKSPACE, asset_manager, device, queue)?;

    let mut meshes = HashMap::default();
    let mut entities = HashMap::default();
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_model_recursive(
        world,
        device,
        queue,
        &mut meshes,
        &mut entities,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    )
    .map_err(|_| ModelAssetError::NoSuchEntity)?;

    let collection = Arc::new(SkinningAnimation {
        root: entities
            .get(MODEL_BONE_ROOT)
            .cloned()
            .ok_or(ModelAssetError::NoSuchEntity)?,
        meshes,
    });

    Ok((entity, collection, batch_commands))
}

/// `aris_original` 모델을 구성하는 `Entity`를 생성하는 재귀함수입니다.
fn spawn_model_recursive(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    meshes: &mut HashMap<String, Entity>,
    entities: &mut HashMap<String, Entity>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Result<Entity, NoSuchEntity> {
    let name = current.name.clone();
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform.into_mat4()));
    builder.add(WorldTransform::default());

    if let Some(child) = current.children.first() {
        let entity = spawn_model_recursive(
            world,
            device,
            queue,
            meshes,
            entities,
            batch_commands,
            entity,
            child,
            &current.children[1..],
        )?;
        builder.add(Child(entity));
    }

    if let Some(sibling) = siblings.first() {
        let entity = spawn_model_recursive(
            world,
            device,
            queue,
            meshes,
            entities,
            batch_commands,
            parent,
            sibling,
            &siblings[1..],
        )?;
        builder.add(Sibling(entity));
    }

    if let Some(mesh) = &current.mesh {
        let mesh = mesh.clone();
        let mesh_name = mesh.name().to_string();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh.name()), device));

        if let Some(skinning) = &current.skinning {
            mesh_resource.skinning_uniform.update(
                device,
                queue,
                SkinningDataLayout {
                    quality: skinning.quality,
                    num_bones: skinning.num_bones,
                    ..Default::default()
                },
            );
            mesh_resource
                .bindpose_uniform
                .update(device, queue, skinning.bindposes.clone());

            let root = entities
                .get(&skinning.root_bone)
                .cloned()
                .ok_or(NoSuchEntity)?;
            let mut bones = Vec::with_capacity(skinning.bones.len());
            for name in skinning.bones.iter() {
                bones.push(entities.get(name).cloned().ok_or(NoSuchEntity)?);
            }
            builder.add(Arc::new(BoneCollection { root, bones }));
        }

        builder.add(mesh);
        builder.add(mesh_resource);
        builder.add(Character::ArisOriginal);

        meshes.insert(mesh_name, entity);
    }

    if !current.materials.is_empty() {
        let mut materials = Vec::with_capacity(current.materials.len());
        for resource in current.materials.iter() {
            materials.push(resource.clone());
        }
        builder.add(materials);
    }

    entities.insert(name, entity);
    batch_commands.push((entity, builder));
    Ok(entity)
}

/// `aris_original_halo` 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 다음 컴포넌트를 기본으로 가집니다.
/// - `Parent`
/// - `ToParentTrans`
/// - `WorldTransform`
///
/// 생성된 엔터티는 다음 컴포넌트를 선택적으로 가집니다.
/// - `Arc<Mesh>`
/// - `Arc<MeshResource>`
/// - `Child`
/// - `Sibling`
/// - `Vec<Arc<MaterialResource>>`
///
pub(super) fn spawn_aris_original_halo_model(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    parent: Entity,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let root = ModelHierarchyPool::get_or_init(
        &MODEL_HALO_NAME,
        &WORKSPACE,
        asset_manager,
        device,
        queue,
    )?;

    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_model_halo_recursive(
        world,
        device,
        queue,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    )
    .map_err(|_| ModelAssetError::NoSuchEntity)?;

    Ok((entity, batch_commands))
}

/// `aris_original_halo` 모델을 구성하는 `Entity`를 생성하는 재귀함수입니다.
fn spawn_model_halo_recursive(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Result<Entity, NoSuchEntity> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform.into_mat4()));
    builder.add(WorldTransform::default());

    if let Some(child) = current.children.first() {
        let entity = spawn_model_halo_recursive(
            world,
            device,
            queue,
            batch_commands,
            entity,
            child,
            &current.children[1..],
        )?;
        builder.add(Child(entity));
    }

    if let Some(sibling) = siblings.first() {
        let entity = spawn_model_halo_recursive(
            world,
            device,
            queue,
            batch_commands,
            parent,
            sibling,
            &siblings[1..],
        )?;
        builder.add(Sibling(entity));
    }

    if let Some(mesh) = &current.mesh {
        let mesh = mesh.clone();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh.name()), device));

        builder.add(mesh);
        builder.add(mesh_resource);
        builder.add(CharacterHalo::ArisOriginalHalo);
    }

    if !current.materials.is_empty() {
        let mut materials = Vec::with_capacity(current.materials.len());
        for resource in current.materials.iter() {
            materials.push(resource.clone());
        }
        builder.add(materials);
    }

    batch_commands.push((entity, builder));
    Ok(entity)
}
