use std::sync::Arc;

use ahash::HashMap;
use hecs::{Component, Entity, ViewBorrow, World};
use mod_network::components::{ActionState, CharacterKind};
use mod_parallelism::collections::Queue;
use mod_render::{DEPTH_FORMAT, SWAPCHAIN_FORMAT};

use crate::component::{
    set_weapon_position, AttributeKind, CameraResource, CharacterBakePipeline,
    CharacterRenderPipeline, Child, EyeMouthBakePipeline, EyeMouthRenderPipeline,
    HaloRenderPipeline, LightSetResource, MaterialMap, Mesh, MeshFilter, MeshRenderer, Player0,
    Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8, Player9,
    PlayerArchetype, RenderTask, ShadowMap, ShadowResource, Sibling, SkinnedMeshRenderer,
    SkinningAnimation, ToParentTrans, TransformDataLayout, TransformMap, WorldTransform,
    CHARACTER_ATTRIBUTES, MAX_BONES, SHADOW_FORMAT,
};

/// 캐릭터 엔터티의 계층 구조를 갱신합니다.
pub fn update_character_hierarchy(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    action_state: ActionState,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    character_view: &ViewBorrow<&CharacterKind>,
    skinning_view: &ViewBorrow<&SkinningAnimation>,
) {
    // 캐릭터 종류를 가져옵니다.
    let &character_kind = character_view
        .get(entity)
        .expect("invalid entity or invalid entity component!");
    // 스키닝 애니메이션 데이터를 가져옵니다.
    let skinning_animation = skinning_view
        .get(entity)
        .expect("invalid entity or invalid entity component!");

    // 캐릭터 속성 데이터를 가져옵니다.
    let i = character_kind as usize;
    let character_attributes = CHARACTER_ATTRIBUTES[i];

    let parent = glam::Mat4::IDENTITY;
    match archetype {
        PlayerArchetype::Player0 => {
            type Tag = Player0;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player1 => {
            type Tag = Player1;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player2 => {
            type Tag = Player2;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player3 => {
            type Tag = Player3;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player4 => {
            type Tag = Player4;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player5 => {
            type Tag = Player5;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player6 => {
            type Tag = Player6;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player7 => {
            type Tag = Player7;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player8 => {
            type Tag = Player8;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
        PlayerArchetype::Player9 => {
            type Tag = Player9;
            type L<'a> = &'a (Tag, ToParentTrans);
            type W<'a> = &'a mut (Tag, WorldTransform);
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
            set_weapon_position(
                action_state,
                character_attributes,
                skinning_animation,
                child_view,
                sibling_view,
                &mut local_transform_view,
                &mut world_transform_view,
            );
        }
    }
}

/// 엔터티 계층 구조를 갱신합니다.
pub fn update_entity_hierarchy_with_archetype<Tag: Copy + Component>(
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

/// 캐릭터 쉐이더 리소스를 갱신합니다.
///
/// # Note
/// 이 함수는 캐릭터 엔터티 계층 구조가 갱신 된 후에 호출되어야 합니다.
///
pub fn update_character_resource(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    draw_tasks: &Queue<RenderTask>,
) {
    match archetype {
        PlayerArchetype::Player0 => {
            let transform_view = world.view::<&(Player0, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player1 => {
            let transform_view = world.view::<&(Player1, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player2 => {
            let transform_view = world.view::<&(Player2, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player3 => {
            let transform_view = world.view::<&(Player3, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player4 => {
            let transform_view = world.view::<&(Player4, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player5 => {
            let transform_view = world.view::<&(Player5, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player6 => {
            let transform_view = world.view::<&(Player6, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player7 => {
            let transform_view = world.view::<&(Player7, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player8 => {
            let transform_view = world.view::<&(Player8, WorldTransform)>();
            update_character_resource_recursive(
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
        PlayerArchetype::Player9 => {
            let transform_view = world.view::<&(Player9, WorldTransform)>();
            update_character_resource_recursive(
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
}

/// 캐릭터 쉐이더 리소스를 갱신합니다.
fn update_character_resource_recursive<Tag: Copy + Component>(
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
        update_character_resource_recursive(
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
        update_character_resource_recursive(
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

/// 캐릭터 쉐이더 리소스를 수집합니다.
///
/// # Note
/// 이 함수는 캐릭터 엔터티 계층 구조가 갱신 된 후에 호출되어야 합니다.
///
pub fn collect_character_resource(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    transform_resources: &mut ShadowMap,
) {
    match archetype {
        PlayerArchetype::Player0 => {
            let transform_view = world.view::<&(Player0, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player1 => {
            let transform_view = world.view::<&(Player1, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player2 => {
            let transform_view = world.view::<&(Player2, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player3 => {
            let transform_view = world.view::<&(Player3, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player4 => {
            let transform_view = world.view::<&(Player4, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player5 => {
            let transform_view = world.view::<&(Player5, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player6 => {
            let transform_view = world.view::<&(Player6, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player7 => {
            let transform_view = world.view::<&(Player7, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player8 => {
            let transform_view = world.view::<&(Player8, WorldTransform)>();
            collect_character_resource_recursive(
                entity,
                child_view,
                sibling_view,
                &transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
                transform_resources,
            );
        }
        PlayerArchetype::Player9 => {
            let transform_view = world.view::<&(Player9, WorldTransform)>();
            collect_character_resource_recursive(
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
}

/// 캐릭터 쉐이더 리소스를 갱신합니다.
fn collect_character_resource_recursive<Tag: Copy + Component>(
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
        collect_character_resource_recursive(
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
        collect_character_resource_recursive(
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

/// 캐릭터 그림자를 그립니다.
pub fn bake_character<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    shadow_resource: &'a ShadowResource,
    transform_resources: &'a TransformMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(CharacterBakePipeline::get_or_init(device, SHADOW_FORMAT));

    rpass.set_bind_group(0, &shadow_resource.bind_group, &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
    rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

    for (index, filters) in transform_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}

/// 캐릭터를 그립니다.
pub fn draw_character<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    light_resource: &'a LightSetResource,
    material_resources: &'a MaterialMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(CharacterRenderPipeline::get_or_init(
        device,
        SWAPCHAIN_FORMAT,
        DEPTH_FORMAT,
    ));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(3, light_resource.bind_group(), &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
    rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
    rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
    rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

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

/// 캐릭터 그림자를 그립니다.
pub fn bake_character_eye_mouth<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    shadow_resource: &'a ShadowResource,
    transform_resources: &'a TransformMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(EyeMouthBakePipeline::get_or_init(device, SHADOW_FORMAT));

    rpass.set_bind_group(0, &shadow_resource.bind_group, &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
    rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

    for (index, filters) in transform_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}

/// 캐릭터를 그립니다.
pub fn draw_character_eye_mouth<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    light_resource: &'a LightSetResource,
    material_resources: &'a MaterialMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(EyeMouthRenderPipeline::get_or_init(
        device,
        SWAPCHAIN_FORMAT,
        DEPTH_FORMAT,
    ));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(3, light_resource.bind_group(), &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
    rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
    rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
    rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

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

/// 캐릭터를 그립니다.
pub fn draw_character_halo<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    material_resources: &'a MaterialMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(HaloRenderPipeline::get_or_init(
        device,
        SWAPCHAIN_FORMAT,
        DEPTH_FORMAT,
    ));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

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
