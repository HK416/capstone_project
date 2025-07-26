//! 총알 객체와 관련된 코드를 관리합니다.
//!

mod common;
mod energy;

use std::{num::NonZeroU32, ops::Deref, sync::Arc};

use ahash::HashMap;
use hecs::{Component, Entity, EntityBuilder, ViewBorrow, World};
use mod_network::components::InGameBulletPullData;
use mod_parallelism::collections::Queue;
use parking_lot::Mutex;

use crate::{
    asset::{ModelNode, ModelPool, ModelRoot, TextureDataPool, BULLET_URIS},
    component::{
        BoneCollection, BoneTransformUniform, Bullet, BulletMaterialResource,
        BulletMaterialUniform, Child, EnergyBulletMaterialResource, EnergyBulletMaterialUniform,
        MaterialData, MaterialResource, MaterialUniform, MeshFilter, MeshRenderer, MeshResource,
        Parent, RenderTask, Sibling, SkinnedMeshRenderer, SkinnedMeshResource, ToParentTrans,
        TransformDataLayout, TransformUniform, WorldTransform, MAX_BONES,
    },
};

pub use self::{common::*, energy::*};

/// 총알을 구성하는 엔터티를 생성합니다.
///
/// 생성된 최상위 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 총알 종류(`BulletKind`)
/// - 로컬 변환 행렬(`(Bullet, ToParentTrans)`)
/// - 월드 변환 행렬(`(Bullet, WorldTransform)`)
///
pub fn spawn_bullet(
    world: &World,
    half_size_x: NonZeroU32,
    half_size_y: NonZeroU32,
    half_size_z: NonZeroU32,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    bullet: &InGameBulletPullData,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 모델 풀 객체에서 총알 모델 노드를 가져옵니다.
    let i = bullet.kind as usize;
    let root = model_pool
        .get(BULLET_URIS[i])
        .expect("the bullet model must exist!");

    // 엔터티를 하나 할당 받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트를 추가합니다.
    builder.add((
        Bullet,
        ToParentTrans(glam::Mat4::from_rotation_translation(
            bullet.rotation().into(),
            bullet
                .translation(half_size_x, half_size_y, half_size_z)
                .into(),
        )),
    ));
    builder.add((Bullet, WorldTransform::default()));

    // 총알 종류에 따른 총알 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = spawn_bullet_model(
        Some(&format!("Bullet({})", bullet.id)),
        world,
        entity,
        &root,
        device,
        encoder,
        staging_buffers,
        texture_data_pool,
    );

    // 총알 모델 루트 노드를 추가합니다.
    builder.add(Child(child));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    (entity, batch_commands)
}

/// 일반 총알 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`(Bullet, ToParentTrans)`)
/// - 월드 변환 행렬(`(Bullet, WorldTransform)`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 유니폼 버퍼(`Vec<MaterialUniform>`)
///
pub fn spawn_bullet_model(
    label: Option<&str>,
    world: &World,
    parent: Entity,
    root: &ModelRoot,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    texture_data_pool: &TextureDataPool,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    let mut entity_list = HashMap::default();
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_bullet_model_recursive(
        label,
        world,
        parent,
        &root.node,
        &[],
        device,
        encoder,
        staging_buffers,
        &mut batch_commands,
        &mut entity_list,
        texture_data_pool,
    );

    (entity, batch_commands)
}

/// 일반 총알 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`(Bullet, ToParentTrans)`)
/// - 월드 변환 행렬(`(Bullet, WorldTransform)`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 쉐이더 유니폼 버퍼(`Vec<MaterialUniform>`)
///
fn spawn_bullet_model_recursive(
    label: Option<&str>,
    world: &World,
    parent: Entity,
    current: &ModelNode,
    siblings: &[ModelNode],
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    entity_list: &mut HashMap<String, Entity>,
    texture_data_pool: &TextureDataPool,
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add(Parent(parent));
    builder.add((Bullet, ToParentTrans(current.transform)));
    builder.add((Bullet, WorldTransform::default()));

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(node) = current.children.first() {
        // 자식 엔터티를 생성합니다.
        let child = spawn_bullet_model_recursive(
            label,
            world,
            entity,
            node,
            &node.children[1..],
            device,
            encoder,
            staging_buffers,
            batch_commands,
            entity_list,
            texture_data_pool,
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(child));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(node) = siblings.first() {
        // 형제 엔터티를 생성합니다.
        let sibling = spawn_bullet_model_recursive(
            label,
            world,
            parent,
            node,
            &siblings[1..],
            device,
            encoder,
            staging_buffers,
            batch_commands,
            entity_list,
            texture_data_pool,
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(sibling));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = current.mesh.clone() {
        match current.skinning.clone() {
            Some(skinning) => {
                // 바인드 포즈(기본 자세 뼈 변환 행렬) 유니폼 버퍼를 복사합니다.
                let bindpose_uniform = skinning.bindpose_uniform.clone();

                // 뼈 변환 행렬 유니폼 버퍼를 생성합니다.
                let bone_trans_uniform = BoneTransformUniform::uninit(
                    Some(&format!("BoneTransform({})", label.unwrap_or("Unknown"))),
                    device,
                );

                // 스키닝 메쉬 쉐이더 리소스를 생성합니다.
                let resource =
                    SkinnedMeshResource::new(label, device, &bindpose_uniform, &bone_trans_uniform);

                // 스키닝 메쉬를 구성하는 뼈 엔터티 집합을 생성합니다.
                let root = entity_list
                    .get(&skinning.root_bone)
                    .cloned()
                    .expect("no such entity!");
                let mut bones = Vec::with_capacity(MAX_BONES);
                for entity_name in skinning.bones.iter() {
                    let entity = entity_list
                        .get(entity_name)
                        .cloned()
                        .expect("no such entity!");
                    bones.push(entity);
                }
                let collection = BoneCollection { root, bones };

                // 엔터티에 컴포넌트를 추가합니다.
                builder.add_bundle((mesh, collection, bone_trans_uniform, resource));
            }
            None => {
                // 월드 변환 행렬 유니폼 버퍼를 생성합니다.
                let transform_uniform = TransformUniform::uninit(
                    Some(&format!("Transform({})", label.unwrap_or("Unknown"))),
                    device,
                );

                // 메쉬 리소스를 생성합니다.
                let resource = MeshResource::new(label, device, &transform_uniform);

                // 엔터티에 컴포넌트를 추가합니다.
                builder.add_bundle((mesh, transform_uniform, resource));
            }
        }
    }

    // 재질 데이터가 존재하는 경우 엔터티에 재질 데이터를 추가합니다.
    let result = create_material_resources(label, device, texture_data_pool, &current.materials);
    if let Some((uniforms, resources)) = result {
        builder.add_bundle((uniforms, resources));
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}

/// 재질 쉐이더 리소스를 생성합니다.
fn create_material_resources(
    label: Option<&str>,
    device: &wgpu::Device,
    _texture_data_pool: &TextureDataPool,
    materials: &[Arc<MaterialData>],
) -> Option<(Vec<MaterialUniform>, Vec<MaterialResource>)> {
    let num_materials = materials.len();
    if num_materials == 0 {
        return None;
    }

    let mut material_uniforms = Vec::with_capacity(num_materials);
    let mut material_resources = Vec::with_capacity(num_materials);
    for material in materials.iter() {
        match material.deref() {
            MaterialData::Bullet(bullet_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = bullet_material_data.as_layout();
                let material_uniform = BulletMaterialUniform::new(
                    Some(&format!(
                        "BulletMaterialUniform({})",
                        label.unwrap_or("unknown")
                    )),
                    device,
                    data,
                );

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = BulletMaterialResource::new(label, device, &material_uniform);

                material_uniforms.push(MaterialUniform::Bullet {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            MaterialData::EnergyBullet(energy_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = energy_material_data.as_layout();
                let material_uniform = EnergyBulletMaterialUniform::new(
                    Some(&format!(
                        "EnergyBulletMaterialUniform({})",
                        label.unwrap_or("unknown")
                    )),
                    device,
                    data,
                );

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = EnergyBulletMaterialResource::new(label, device, &material_uniform);

                material_uniforms.push(MaterialUniform::EnergyBullet {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            _ => panic!("invalid material data!"),
        }
    }

    Some((material_uniforms, material_resources))
}

/// 총알 엔터티의 계층 구조를 갱신합니다.
pub fn update_bullet_hierarchy(
    world: &World,
    entity: Entity,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
) {
    type L<'a> = &'a (Bullet, ToParentTrans);
    type W<'a> = &'a mut (Bullet, WorldTransform);
    let parent = glam::Mat4::IDENTITY;
    let mut local_transform_view = world.view::<L>();
    let mut world_transform_view = world.view::<W>();
    update_entity_hierarchy_with_archetype(
        entity,
        parent,
        child_view,
        sibling_view,
        &mut local_transform_view,
        &mut world_transform_view,
    );
}

/// 엔터티 계층 구조를 갱신합니다.
fn update_entity_hierarchy_with_archetype<Tag: Copy + Component>(
    entity: Entity,
    parent: glam::Mat4,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    local_transform_view: &mut ViewBorrow<'_, &(Tag, ToParentTrans)>,
    world_transform_view: &mut ViewBorrow<'_, &mut (Tag, WorldTransform)>,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티 계층 구조를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        update_entity_hierarchy_with_archetype(
            entity,
            parent,
            child_view,
            sibling_view,
            local_transform_view,
            world_transform_view,
        );
    }

    // 현재 엔터티의 월드 변환 행렬을 갱신합니다.
    let (_, local_transform) = local_transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component!");
    let (_, world_transform) = world_transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component!");
    let transform = parent * local_transform.0;
    world_transform.0 = transform;

    // 자식 엔터티가 존재하는 경우 자식 엔터티 계층 구조를 갱신합니다.
    if let Some(child) = child_view.get(entity).cloned() {
        let parent = transform;
        let entity = *child;
        update_entity_hierarchy_with_archetype(
            entity,
            parent,
            child_view,
            sibling_view,
            local_transform_view,
            world_transform_view,
        );
    }
}

/// 총알 쉐이더 리소스를 갱신합니다.
///
/// # Note
/// 이 함수는 총알 엔터티 계층 구조가 갱신 된 후에 호출되어야 합니다.
///
pub fn update_bullet_resource(
    world: &World,
    entity: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    draw_tasks: &Queue<RenderTask>,
) {
    let transform_view = world.view::<&(Bullet, WorldTransform)>();
    update_bullet_resource_recursive(
        entity,
        device,
        encoder,
        staging_buffers,
        child_view,
        sibling_view,
        &transform_view,
        mesh_filter_view,
        skinned_mesh_filter_view,
        &draw_tasks,
    );
}

/// 총알 쉐이더 리소스를 갱신합니다.
fn update_bullet_resource_recursive<Tag: Copy + Component>(
    entity: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &ViewBorrow<'_, &(Tag, WorldTransform)>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    draw_tasks: &Queue<RenderTask>,
) {
    // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
    if let Some(child) = child_view.get(entity).cloned() {
        let entity = *child;
        update_bullet_resource_recursive(
            entity,
            device,
            encoder,
            staging_buffers,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            draw_tasks,
        );
    }

    // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        update_bullet_resource_recursive(
            entity,
            device,
            encoder,
            staging_buffers,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            draw_tasks,
        );
    }

    let result = mesh_filter_view.get(entity);
    match result {
        Some((mesh, mesh_resource, mesh_uniform, material_resources)) => {
            // 메쉬 유니폼 버퍼를 갱신합니다.
            let (_, transform) = transform_view
                .get(entity)
                .expect("invalid entity component");
            let data = TransformDataLayout {
                trans: transform.0.to_cols_array(),
            };
            mesh_uniform.update(device, encoder, staging_buffers, data);

            for (index, material_resource) in material_resources.iter().enumerate() {
                // 그리기 작업 목록에 추가합니다.
                draw_tasks.push(RenderTask {
                    mesh: mesh.clone(),
                    mesh_resource: MeshFilter::Mesh(mesh_resource.clone()),
                    material_index: index,
                    material_resource: material_resource.clone(),
                });
            }

            return;
        }
        None => {}
    };

    let result = skinned_mesh_filter_view.get(entity);
    match result {
        Some((
            mesh,
            mesh_resource,
            bone_collection,
            bone_transform_uniform,
            material_resources,
        )) => {
            // 뼈 변환 행렬 유니폼 버퍼를 갱신합니다.
            let mut data = Vec::with_capacity(MAX_BONES);
            for entity in bone_collection.bones.iter().cloned() {
                let (_, transform) = transform_view
                    .get(entity)
                    .expect("invalid entity or invalid entity component!");
                data.push(transform.0.to_cols_array());
            }
            bone_transform_uniform.update(device, encoder, staging_buffers, data);

            for (index, material_resource) in material_resources.iter().enumerate() {
                // 그리기 작업 목록에 추가합니다.
                draw_tasks.push(RenderTask {
                    mesh: mesh.clone(),
                    mesh_resource: MeshFilter::SkinnedMesh(mesh_resource.clone()),
                    material_index: index,
                    material_resource: material_resource.clone(),
                });
            }

            return;
        }
        None => {}
    }
}
