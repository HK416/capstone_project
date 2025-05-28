//! 총알 객체와 관련된 코드를 관리합니다.
//!

mod common;
mod energy;

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_network::components::{Bullet, NUM_BULLETS};

use crate::{
    asset::{ModelPool, ModelRoot, TextureDataPool, BULLET_URIS},
    component::{Child, ToParentTrans, WorldTransform},
};

pub use self::{common::*, energy::*};

use super::{
    AttributeKind, CameraResource, LightSetResource, MaterialKind, MaterialResource, Mesh,
    MeshFilter, MeshRenderer, OpaqueMap, ShadowMap, Sibling, SkinnedMeshRenderer,
    TransformDataLayout, TransparentMap,
};

/// 총알을 구성하는 엔터티를 생성합니다.
///
/// 생성된 최상위 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 총알 종류(`BulletKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spawn_bullet(
    world: &World,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    bullet: &Bullet,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    type Func = fn(
        Option<&str>,
        &TextureDataPool,
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
        texture_data_pool,
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

/// 총알의 쉐이더 리소스를 갱신합니다.
pub fn update_bullet_resource(
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
        update_bullet_resource(
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
        update_bullet_resource(
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

        // 렌더 집합에 추가합니다.
        for (index, material) in materials.iter().enumerate() {
            let key = (mesh.clone(), material.kind());
            let sub_key = (index, material.clone());
            let val = MeshFilter::Mesh(mesh_resource.clone());
            match material.kind() {
                MaterialKind::Bullet => {
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
                }
                MaterialKind::EnergyBullet => {
                    if let Some(res_map) = transparent_map.get_mut(&key) {
                        match res_map.get_mut(&sub_key) {
                            Some(filters) => {
                                filters.push(val);
                            }
                            None => {
                                res_map.insert(sub_key, vec![val]);
                            }
                        }
                    } else {
                        transparent_map.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
                    }
                }
                _ => {}
            };
        }

        // 그림자 집합에 추가합니다.
        for (index, material) in materials.iter().enumerate() {
            if material.kind() == MaterialKind::Bullet {
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

        // 렌더 집합에 추가합니다.
        for (index, material) in materials.iter().enumerate() {
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
        }

        // 그림자 집합에 추가합니다.
        for (index, material) in materials.iter().enumerate() {
            if material.kind() == MaterialKind::Bullet {
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
        }

        return;
    }
}

/// 총알을 그립니다.
pub fn draw_bullet<'a>(
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

/// 에너지 볼 형태의 총알을 그립니다.
pub fn draw_energy_bullet<'a>(
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
