use std::sync::Arc;

use ahash::HashMap;
use hecs::{Component, Entity, ViewBorrow, World};
use mod_parallelism::collections::Queue;
use mod_render::{DEPTH_FORMAT, SWAPCHAIN_FORMAT};

use crate::component::{
    AttributeKind, CameraResource, Child, LightSetResource, MaterialMap, Mesh, MeshFilter,
    MeshRenderer, RenderTask, ShadowMap, ShadowResource, Sibling, SkinnedMeshRenderer, Stage,
    StageBakePipeline, StageRenderPipeline, ToParentTrans, TransformDataLayout, TransformMap,
    TreeRenderPipeline, WorldTransform, MAX_BONES, SHADOW_FORMAT,
};

/// 지형 엔터티의 계층 구조를 갱신합니다.
pub fn update_stage_hierarchy(
    world: &World,
    entities: &[Entity],
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
) {
    type Tag = Stage;
    type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
    let mut transform_view = world.view::<Q>();
    let parent = glam::Mat4::IDENTITY;
    for &entity in entities {
        update_entity_hierarchy(
            entity,
            parent,
            child_view,
            sibling_view,
            &mut transform_view,
        );
    }
}

/// 엔터티 계층 구조를 갱신합니다.
fn update_entity_hierarchy<Tag: Copy + Component>(
    entity: Entity,
    parent: glam::Mat4,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &mut ViewBorrow<'_, (&(Tag, ToParentTrans), &mut (Tag, WorldTransform))>,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티 계층 구조를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        update_entity_hierarchy(entity, parent, child_view, sibling_view, transform_view);
    }

    // 현재 엔터티의 월드 변환 행렬을 갱신합니다.
    let ((_, local_transform), (_, world_transform)) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component!");
    let transform = parent * local_transform.0;
    world_transform.0 = transform;

    // 자식 엔터티가 존재하는 경우 자식 엔터티 계층 구조를 갱신합니다.
    if let Some(child) = child_view.get(entity).cloned() {
        let parent = transform;
        let entity = *child;
        update_entity_hierarchy(entity, parent, child_view, sibling_view, transform_view);
    }
}

/// 스테이지 쉐이더 리소스를 갱신합니다.
///
/// # Note
/// 이 함수는 스테이지 엔터티 계층 구조가 갱신 된 후에 호출되어야 합니다.
///
pub fn update_stage_resource(
    world: &World,
    entities: &[Entity],
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    draw_tasks: &Arc<Queue<RenderTask>>,
) {
    let transform_view = world.view::<&(Stage, WorldTransform)>();
    for &entity in entities {
        update_stage_resource_recursive(
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
}

/// 스테이지 쉐이더 리소스를 갱신합니다.
fn update_stage_resource_recursive<Tag: Copy + Component>(
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
        update_stage_resource_recursive(
            entity,
            device,
            encoder,
            staging_buffers,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            &draw_tasks,
        );
    }

    // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        update_stage_resource_recursive(
            entity,
            device,
            encoder,
            staging_buffers,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            &draw_tasks,
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

/// 스테이지 쉐이더 리소스를 수집합니다.
///
/// # Note
/// 이 함수는 스테이지 엔터티 계층 구조가 갱신 된 후에 호출되어야 합니다.
///
pub fn collect_stage_resource(
    world: &World,
    entities: &[Entity],
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    transform_resources: &mut ShadowMap,
) {
    let transform_view = world.view::<&(Stage, WorldTransform)>();
    for &entity in entities {
        collect_stage_resource_recursive(
            entity,
            child_view,
            sibling_view,
            &transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            transform_resources,
        );
    }
}

/// 스테이지 쉐이더 리소스를 수집합니다.
fn collect_stage_resource_recursive<Tag: Copy + Component>(
    entity: Entity,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &ViewBorrow<'_, &(Tag, WorldTransform)>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    transform_resources: &mut ShadowMap,
) {
    // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
    if let Some(child) = child_view.get(entity).cloned() {
        let entity = *child;
        collect_stage_resource_recursive(
            entity,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            transform_resources,
        );
    }

    // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        collect_stage_resource_recursive(
            entity,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            transform_resources,
        );
    }

    let result = mesh_filter_view.get(entity);
    match result {
        Some((mesh, mesh_resource, _mesh_uniform, material_resources)) => {
            for (material_index, material_resource) in material_resources.iter().enumerate() {
                // 그림자 작업 목록에 추가합니다.
                let material_kind = material_resource.kind();
                let key = (mesh.clone(), material_kind);
                let sub_key = material_index;
                let val = MeshFilter::Mesh(mesh_resource.clone());
                match transform_resources.get_mut(&key) {
                    Some(resource_map) => match resource_map.get_mut(&sub_key) {
                        Some(list) => {
                            list.push(val);
                        }
                        None => {
                            resource_map.insert(sub_key, vec![val]);
                        }
                    },
                    None => {
                        transform_resources.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }
                }
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
            _bone_collection,
            _bone_transform_uniform,
            material_resources,
        )) => {
            for (material_index, material_resource) in material_resources.iter().enumerate() {
                // 그림자 작업 목록에 추가합니다.
                let material_kind = material_resource.kind();
                let key = (mesh.clone(), material_kind);
                let sub_key = material_index;
                let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
                match transform_resources.get_mut(&key) {
                    Some(resource_map) => match resource_map.get_mut(&sub_key) {
                        Some(list) => {
                            list.push(val);
                        }
                        None => {
                            resource_map.insert(sub_key, vec![val]);
                        }
                    },
                    None => {
                        transform_resources.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }
                }
            }

            return;
        }
        None => {}
    }
}

/// 지형의 그림자를 생성합니다.
pub fn bake_stage<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    shadow_resource: &'a ShadowResource,
    transform_resources: &'a TransformMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(StageBakePipeline::get_or_init(device, SHADOW_FORMAT));

    rpass.set_bind_group(0, &shadow_resource.bind_group, &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));

    for (index, filters) in transform_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}

/// 지형을 그립니다.
pub fn draw_stage<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    light_resource: &'a LightSetResource,
    material_resources: &'a MaterialMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(StageRenderPipeline::get_or_init(
        device,
        SWAPCHAIN_FORMAT,
        DEPTH_FORMAT,
    ));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(3, light_resource.bind_group(), &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
    rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

    for ((index, material), filters) in material_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
        rpass.set_bind_group(2, material.bind_group(), &[]);

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}

/// 나무를 그립니다.
pub fn draw_tree<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    light_resource: &'a LightSetResource,
    material_resources: &'a MaterialMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(TreeRenderPipeline::get_or_init(
        device,
        SWAPCHAIN_FORMAT,
        DEPTH_FORMAT,
    ));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(3, light_resource.bind_group(), &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
    rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

    for ((index, material), filters) in material_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
        rpass.set_bind_group(2, material.bind_group(), &[]);

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}
