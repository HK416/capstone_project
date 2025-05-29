//! 스테이지 객체와 관련된 코드를 관리합니다.
//!

mod city;
mod pipeline;
mod spawn;

use ahash::HashMap;
use hecs::{Entity, ViewBorrow};
use mod_network::components::{MAX_IN_GAME_TEAM_PLAYERS, NUM_STAGES};

pub use self::{pipeline::*, spawn::*};

use super::{
    AttributeKind, CameraResource, Child, LightSetResource, MaterialKind, MaterialResource, Mesh,
    MeshFilter, MeshRenderer, OpaqueMap, ShadowMap, ShadowResource, Sibling, SkinnedMeshRenderer,
    TransformDataLayout, TransparentMap, WorldTransform,
};

/// 승리 팀의 회전방향입니다.
pub const RESET_ROTATION: [[glam::Quat; 2]; NUM_STAGES] = [city::RESET_ROTATION];

/// 승리 팀의 위치입니다.
pub const RESET_POSITIONS: [[glam::Vec3; MAX_IN_GAME_TEAM_PLAYERS]; NUM_STAGES] =
    [city::RESET_POSITIONS];

/// 스테이지 객체 태그
pub struct StageTag;

/// 지형의 쉐이더 리소스를 갱신합니다.
pub fn update_stage_resource(
    entity: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    shadow_map: &mut ShadowMap,
    opaque_map: &mut OpaqueMap,
    transparent_map: &mut TransparentMap,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &ViewBorrow<'_, &WorldTransform>,
    mesh_filter_view: &mut ViewBorrow<'_, MeshRenderer>,
    skinned_mesh_filter_view: &mut ViewBorrow<'_, SkinnedMeshRenderer>,
) {
    // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
    if let Some(child_entity) = child_view.get(entity).cloned() {
        update_stage_resource(
            *child_entity,
            device,
            encoder,
            staging_buffers,
            shadow_map,
            opaque_map,
            transparent_map,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
        );
    }

    // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        update_stage_resource(
            *sibling_entity,
            device,
            encoder,
            staging_buffers,
            shadow_map,
            opaque_map,
            transparent_map,
            child_view,
            sibling_view,
            transform_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
        );
    }

    let result = mesh_filter_view.get_mut(entity);
    if let Some((mesh, mesh_resource, uniform, _, materials)) = result {
        // 유니폼 버퍼를 갱신합니다.
        let transform = transform_view
            .get(entity)
            .expect("invalid entity component");
        uniform.update(
            device,
            encoder,
            staging_buffers,
            TransformDataLayout {
                trans: transform.0.to_cols_array(),
            },
        );

        for (index, material) in materials.iter().enumerate() {
            match material.kind() {
                MaterialKind::Stage | MaterialKind::Tree => {
                    // 불투명 렌더 집합에 추가합니다.
                    let key = (mesh.clone(), material.kind());
                    let sub_key = (index, material.clone());
                    let val = MeshFilter::Mesh(mesh_resource.clone());
                    if let Some(res_map) = opaque_map.get_mut(&key) {
                        match res_map.get_mut(&sub_key) {
                            Some(filters) => {
                                filters.push(val);
                            }
                            None => {
                                res_map.insert(sub_key, vec![val]);
                            }
                        }
                    } else {
                        opaque_map.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }

                    // 그림자 집합에 추가합니다.
                    let key = (mesh.clone(), material.kind());
                    let val = MeshFilter::Mesh(mesh_resource.clone());
                    if let Some(res_map) = shadow_map.get_mut(&key) {
                        match res_map.get_mut(&index) {
                            Some(filters) => {
                                filters.push(val);
                            }
                            None => {
                                res_map.insert(index, vec![val]);
                            }
                        }
                    } else {
                        shadow_map.insert(key, HashMap::from_iter([(index, vec![val])]));
                    }
                }
                _ => {}
            }
        }

        return;
    }

    let result = skinned_mesh_filter_view.get_mut(entity);
    if let Some((mesh, mesh_resource, collection, uniform, _, materials)) = result {
        // 유니폼 버퍼를 갱신합니다.
        let data = collection
            .bones
            .iter()
            .map(|&entity| {
                transform_view
                    .get(entity)
                    .expect("invalid entity or invalid entity component")
            })
            .map(|transform| transform.0.to_cols_array())
            .collect();
        uniform.update(device, encoder, staging_buffers, data);

        for (index, material) in materials.iter().enumerate() {
            match material.kind() {
                MaterialKind::Stage | MaterialKind::Tree => {
                    // 불투명 렌더 집합에 추가합니다.
                    let key = (mesh.clone(), material.kind());
                    let sub_key = (index, material.clone());
                    let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
                    if let Some(res_map) = opaque_map.get_mut(&key) {
                        match res_map.get_mut(&sub_key) {
                            Some(filters) => {
                                filters.push(val);
                            }
                            None => {
                                res_map.insert(sub_key, vec![val]);
                            }
                        }
                    } else {
                        opaque_map.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }

                    // 그림자 집합에 추가합니다.
                    let key = (mesh.clone(), material.kind());
                    let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
                    if let Some(res_map) = shadow_map.get_mut(&key) {
                        match res_map.get_mut(&index) {
                            Some(filters) => {
                                filters.push(val);
                            }
                            None => {
                                res_map.insert(index, vec![val]);
                            }
                        }
                    } else {
                        shadow_map.insert(key, HashMap::from_iter([(index, vec![val])]));
                    }
                }
                _ => {}
            }
        }

        return;
    }
}

/// 지형을 그립니다.
pub fn draw_stage<'a>(
    mesh: &'a Mesh,
    pipeline: &'a wgpu::RenderPipeline,
    camera_resource: &'a CameraResource,
    light_set_resource: &'a LightSetResource,
    material_resources: &'a HashMap<(usize, MaterialResource), Vec<MeshFilter>>,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(&pipeline);

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(3, light_set_resource.bind_group(), &[]);

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

/// 지형의 그림자를 생성합니다.
pub fn bake_stage<'a>(
    mesh: &'a Mesh,
    pipeline: &'a wgpu::RenderPipeline,
    shadow_resource: &'a ShadowResource,
    submesh_resources: &'a HashMap<usize, Vec<MeshFilter>>,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(&pipeline);

    rpass.set_bind_group(0, &shadow_resource.bind_group, &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));

    for (index, filters) in submesh_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}
