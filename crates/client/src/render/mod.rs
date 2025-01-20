pub mod camera;
pub mod character;
pub mod skybox;
pub mod terrain;

use std::sync::Arc;

use hecs::{Entity, ViewBorrow, World};
use mod_render::{MeshResource, TransformDataLayout, MAX_BONES};

use crate::component::{BoneCollection, Child, Sibling, WorldTransform};

pub use self::{camera::*, character::*, skybox::*, terrain::*};

/// 주어진 엔터티의 메쉬 리소스를 준비합니다.
///
/// 주어진 엔터티가 쉐이더 리소스(`Arc<MeshResource>`), 월드 변환 행렬(`WorldTransform`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// 엔터티가 뼈 모음(`BoneCollection`)을 갖고 있지 않는 경우 메쉬의 쉐이더 리소스를 갱신합니다.  
/// 엔터티가 뼈 모음(`BoneCollection`)을 갖고 있는 경우 스키닝된 메쉬의 쉐이더 리소스를 갱신합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn prepare_mesh_resource(
    world: &World,
    entities: &[Entity], // Frustum Culling된 엔터티들
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    let child_view = &world.view::<&Child>();
    let sibling_view = &world.view::<&Sibling>();
    let transform_view = &world.view::<&WorldTransform>();
    let resource_view = &world.view::<&Arc<MeshResource>>();
    let bone_collection_view = &world.view::<&BoneCollection>();
    rayon::in_place_scope(|scope| {
        for &entity in entities {
            scope.spawn(move |_| {
                prepare_mesh_resource_recursion(
                    child_view,
                    sibling_view,
                    transform_view,
                    resource_view,
                    bone_collection_view,
                    entity,
                    device,
                    queue,
                )
            });
        }
    });
}

/// 주어진 엔터티의 메쉬 리소스를 준비하는 재귀함수입니다.
///
/// 주어진 엔터티가 쉐이더 리소스(`Arc<MeshResource>`), 월드 변환 행렬(`WorldTransform`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// 엔터티가 뼈 모음(`BoneCollection`)을 갖고 있지 않는 경우 메쉬의 쉐이더 리소스를 갱신합니다.  
/// 엔터티가 뼈 모음(`BoneCollection`)을 갖고 있는 경우 스키닝된 메쉬의 쉐이더 리소스를 갱신합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn prepare_mesh_resource_recursion(
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &ViewBorrow<'_, &WorldTransform>,
    resource_view: &ViewBorrow<'_, &Arc<MeshResource>>,
    bone_collection_view: &ViewBorrow<'_, &BoneCollection>,
    entity: Entity,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 탐색합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        prepare_mesh_resource_recursion(
            child_view,
            sibling_view,
            transform_view,
            resource_view,
            bone_collection_view,
            *sibling_entity,
            device,
            queue,
        );
    }

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 탐색합니다.
    if let Some(child_entity) = child_view.get(entity).cloned() {
        prepare_mesh_resource_recursion(
            child_view,
            sibling_view,
            transform_view,
            resource_view,
            bone_collection_view,
            *child_entity,
            device,
            queue,
        );
    }

    // 현재 엔터티가 조건에 맞는지 확인합니다.
    let results = transform_view.get(entity).zip(resource_view.get(entity));
    if let Some((world_transform, mesh_resource)) = results {
        // 쉐이더 리소스를 갱신합니다.
        if let Some(bone_collection) = bone_collection_view.get(entity) {
            let mut bone_transforms = vec![[0.0; 16]; MAX_BONES];
            for (index, bone_entity) in bone_collection.bones.iter().cloned().enumerate() {
                // 뼈 엔터티에 월드 변환 행렬 컴포넌트를 가져옵니다.
                let world_transform = transform_view
                    .get(bone_entity)
                    .expect("invalid entity or invalid entity component");

                // 뼈 변환 행렬 모음에 뼈 엔터티의 월드 변환 행렬을 저장합니다.
                bone_transforms[index] = world_transform.0.to_cols_array();
            }

            // 쉐이더 리소스를 갱신합니다.
            mesh_resource
                .bone_trans_uniform
                .update(device, queue, bone_transforms);
        } else {
            // 쉐이더 리소스를 갱신합니다.
            mesh_resource.transform_uniform.update(
                device,
                queue,
                TransformDataLayout {
                    trans: world_transform.0.to_cols_array(),
                    ..Default::default()
                },
            );
        }
    }
}
