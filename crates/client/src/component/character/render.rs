use ahash::HashMap;
use hecs::{Component, Entity, ViewBorrow, World};

use crate::component::{
    Child, MeshFilter, MeshRenderer, OpaqueMap, Player0, Player1, Player2, Player3, Player4,
    Player5, Player6, Player7, Player8, Player9, PlayerArchetype, ShadowMap, Sibling,
    SkinnedMeshRenderer, ToParentTrans, TransformDataLayout, TransparentMap, WorldTransform,
    MAX_BONES,
};

/// 캐릭터 엔터티의 계층 구조를 갱신합니다.
pub fn update_character_hierarchy(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
) {
    let parent = glam::Mat4::IDENTITY;
    match archetype {
        PlayerArchetype::Player0 => {
            type Tag = Player0;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player1 => {
            type Tag = Player1;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player2 => {
            type Tag = Player2;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player3 => {
            type Tag = Player3;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player4 => {
            type Tag = Player4;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player5 => {
            type Tag = Player5;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player6 => {
            type Tag = Player6;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player7 => {
            type Tag = Player7;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player8 => {
            type Tag = Player8;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
        PlayerArchetype::Player9 => {
            type Tag = Player9;
            type Q<'a> = (&'a (Tag, ToParentTrans), &'a mut (Tag, WorldTransform));
            let mut transform_view = world.view::<Q>();
            update_entity_hierarchy_with_archetype(
                entity,
                parent,
                child_view,
                sibling_view,
                &mut transform_view,
            );
        }
    }
}

/// 엔터티 계층 구조를 갱신합니다.
fn update_entity_hierarchy_with_archetype<Tag: Copy + Component>(
    entity: Entity,
    parent: glam::Mat4,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &mut ViewBorrow<'_, (&(Tag, ToParentTrans), &mut (Tag, WorldTransform))>,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티 계층 구조를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        update_entity_hierarchy_with_archetype(
            entity,
            parent,
            child_view,
            sibling_view,
            transform_view,
        );
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
        update_entity_hierarchy_with_archetype(
            entity,
            parent,
            child_view,
            sibling_view,
            transform_view,
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
) -> (ShadowMap, OpaqueMap, TransparentMap) {
    let mut shadow_resources = ShadowMap::default();
    let mut opaque_resources = OpaqueMap::default();
    let mut transparent_resources = TransparentMap::default();

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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
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
                &mut shadow_resources,
                &mut opaque_resources,
                &mut transparent_resources,
            );
        }
    }

    (shadow_resources, opaque_resources, transparent_resources)
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
    shadow_resources: &mut ShadowMap,
    opaque_resources: &mut OpaqueMap,
    transparent_resources: &mut TransparentMap,
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
            shadow_resources,
            opaque_resources,
            transparent_resources,
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
            shadow_resources,
            opaque_resources,
            transparent_resources,
        );
    }

    let result = mesh_filter_view.get(entity);
    match result {
        Some((mesh, mesh_resource, mesh_uniform, _material_uniforms, material_resources)) => {
            // 메쉬 유니폼 버퍼를 갱신합니다.
            let (_, transform) = transform_view
                .get(entity)
                .expect("invalid entity component");
            let data = TransformDataLayout {
                trans: transform.0.to_cols_array(),
            };
            mesh_uniform.update(device, encoder, staging_buffers, data);

            for (index, material_resource) in material_resources.iter().enumerate() {
                // 렌더 집합에 추가합니다.
                let material_kind = material_resource.kind();
                let key = (mesh.clone(), material_kind);
                let sub_key = (index, material_resource.clone());
                let val = MeshFilter::Mesh(mesh_resource.clone());
                match opaque_resources.get_mut(&key) {
                    Some(resource_map) => match resource_map.get_mut(&sub_key) {
                        Some(list) => {
                            list.push(val);
                        }
                        None => {
                            resource_map.insert(sub_key, vec![val]);
                        }
                    },
                    None => {
                        opaque_resources.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }
                }

                // 그림자 집합에 추가합니다.
                if material_kind.is_opaque() {
                    let key = (mesh.clone(), material_kind);
                    let val = MeshFilter::Mesh(mesh_resource.clone());
                    match shadow_resources.get_mut(&key) {
                        Some(resource_map) => match resource_map.get_mut(&index) {
                            Some(list) => {
                                list.push(val);
                            }
                            None => {
                                resource_map.insert(index, vec![val]);
                            }
                        },
                        None => {
                            shadow_resources.insert(key, HashMap::from_iter([(index, vec![val])]));
                        }
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
            bone_collection,
            bone_transform_uniform,
            _material_uniforms,
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
                // 렌더 집합에 추가합니다.
                let material_kind = material_resource.kind();
                let key = (mesh.clone(), material_kind);
                let sub_key = (index, material_resource.clone());
                let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
                match opaque_resources.get_mut(&key) {
                    Some(resource_map) => match resource_map.get_mut(&sub_key) {
                        Some(list) => {
                            list.push(val);
                        }
                        None => {
                            resource_map.insert(sub_key, vec![val]);
                        }
                    },
                    None => {
                        opaque_resources.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }
                }

                // 그림자 집합에 추가합니다.
                if material_kind.is_opaque() {
                    let key = (mesh.clone(), material_kind);
                    let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
                    match shadow_resources.get_mut(&key) {
                        Some(resource_map) => match resource_map.get_mut(&index) {
                            Some(list) => {
                                list.push(val);
                            }
                            None => {
                                resource_map.insert(index, vec![val]);
                            }
                        },
                        None => {
                            shadow_resources.insert(key, HashMap::from_iter([(index, vec![val])]));
                        }
                    }
                }
            }

            return;
        }
        None => {}
    }
}
