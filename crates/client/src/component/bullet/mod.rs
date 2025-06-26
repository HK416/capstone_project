//! 총알 객체와 관련된 코드를 관리합니다.
//!

mod common;
mod energy;

use std::{ops::Deref, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_network::components::InGameBulletPullData;
use parking_lot::Mutex;

use crate::{
    asset::{ModelNode, ModelPool, ModelRoot, TextureDataPool, BULLET_URIS},
    component::{
        BoneCollection, BoneTransformUniform, Bullet, BulletMaterialResource,
        BulletMaterialUniform, Child, EnergyBulletMaterialResource, EnergyBulletMaterialUniform,
        MaterialData, MaterialUniform, MeshResource, Parent, SkinnedMeshResource, ToParentTrans,
        TransformUniform, WorldTransform, MAX_BONES,
    },
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
/// - 로컬 변환 행렬(`(Bullet, ToParentTrans)`)
/// - 월드 변환 행렬(`(Bullet, WorldTransform)`)
///
pub fn spawn_bullet(
    world: &World,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    bullet: &InGameBulletPullData,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 모델 풀 객체에서 총알 모델 노드를 가져옵니다.
    let i = bullet.bullet_kind as usize;
    let root = model_pool
        .get(BULLET_URIS[i])
        .expect("the bullet model must exist!");

    // 엔터티를 하나 할당 받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트를 추가합니다.
    builder.add(bullet.bullet_kind);
    builder.add((
        Bullet,
        ToParentTrans(glam::Mat4::from_rotation_translation(
            glam::Quat::from_array(bullet.rotation),
            glam::Vec3::from_array(bullet.translation),
        )),
    ));
    builder.add((Bullet, WorldTransform::default()));

    // 총알 종류에 따른 총알 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = spawn_bullet_model(
        Some(&format!("Bullet({})", bullet.object_id)),
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
