mod camera;
mod student;

use std::sync::Arc;

use hecs::{Entity, QueryOneError, World};
use mod_render::{MeshResource, TransformDataLayout, MAX_BONES};

use crate::component::{BoneCollection, WorldTransform};

pub use self::{camera::*, student::*};

/// 메쉬 리소스를 준비합니다.  
///
/// # Panics
/// - `Entity`의 `Component`를 획득할 때 스레드가 안전하지 않은 상태일 경우 [`panic!`]을 호출합니다.
///
pub fn sys_prepare_mesh_resource(
    world: &World,
    _camera: &Entity, // 나중에 Camera Frustom Culling을 수행하도록 수정
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    batch_size: u32,
) {
    type Q<'a> = (&'a Arc<MeshResource>, &'a WorldTransform);
    let mut query = world.query::<Q>();
    let mut batched_iter = query.iter_batched(batch_size);
    rayon::scope(|scope| {
        while let Some(query) = batched_iter.next() {
            scope.spawn(move |_| {
                for (entity, (resource, transform)) in query {
                    let mut query = world
                        .query_one::<&BoneCollection>(entity)
                        .expect("no such entity");
                    let collection = query.get();

                    prepare_mesh_resource(world, device, queue, transform, resource, collection)
                        .expect("invalid model entity");
                }
            });
        }
    })
}

/// 메쉬의 리소스를 준비합니다.  
/// 메쉬가 `BoneCollection`을 가진 경우 스키닝된 메쉬 리소스를 준비합니다.
///
/// # Panics
/// `BoneCollection`이 주어졌을 때, `BoneCollection`에 속한 `Entity`를 찾지 못하거나,
/// `Entity`가 `WorldTransform`을 갖고있지 않는 경우 `QueryOneError`를 반환합니다.
///
pub fn prepare_mesh_resource(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    transform: &WorldTransform,
    resource: &MeshResource,
    collection: Option<&BoneCollection>,
) -> Result<(), QueryOneError> {
    // 메쉬의 뼈 변환 행렬 쉐이더 리소스를 갱신합니다.
    if let Some(collection) = collection {
        let mut transforms = vec![[0.0; 16]; MAX_BONES];
        let iter = collection.bones.iter().cloned().enumerate();
        for (index, entity) in iter {
            let mut transform = world
                .query_one::<&WorldTransform>(entity)
                .map_err(|_| QueryOneError::NoSuchEntity)?;
            let transform = transform.get().ok_or(QueryOneError::Unsatisfied)?;
            transforms[index] = transform.0.to_cols_array();
        }

        resource
            .bone_trans_uniform
            .update(device, queue, transforms);
    } else {
        // 메쉬의 월드 변환 행렬 쉐이더 리소스를 갱신합니다.
        resource.transform_uniform.update(
            device,
            queue,
            TransformDataLayout {
                trans: transform.0.to_cols_array(),
                ..Default::default()
            },
        );
    }

    Ok(())
}
