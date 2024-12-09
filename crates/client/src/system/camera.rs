use std::sync::Arc;

use hecs::{Entity, World};
use mod_render::{CameraDataLayout, CameraResource};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::component::{CameraTag, Projection, WorldTransform};

/// 카메라 리소스를 준비합니다.  
/// 이 함수는 다른 읽기 작업과 동시에 실행 가능합니다.
///
/// # Panics
/// - 주어진 `Entity`는 `Arc<CameraResource>`, `WorldTransform`, `Projection`, `CameraTag`를
/// 갖고 있어야 합니다. 그렇지 않은 경우 [`panic!`]을 호출합니다.
/// - `Entity`의 `Component`를 획득할 때 스레드가 안전하지 않은 상태일 경우 [`panic!`]을 호출합니다.
///
pub fn sys_prepare_camera_resource(
    world: &World,
    entities: &[Entity],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    type Q<'a> = (&'a Arc<CameraResource>, &'a WorldTransform, &'a Projection);
    type R<'a> = &'a CameraTag;

    entities.par_iter().for_each(|&entity| {
        let mut query = world
            .query_one::<Q>(entity)
            .expect("the entity does not match the condition")
            .with::<R>();

        let (resource, transform, projection) = query
            .get()
            .expect("the entity does not match the condition or thread not safety");

        prepare_camera_resource(device, queue, resource, transform, projection);
    });
}

/// 카메라 리소스를 준비합니다.
pub fn prepare_camera_resource(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    resource: &CameraResource,
    transform: &WorldTransform,
    projection: &Projection,
) {
    resource.camera_uniform.update(
        device,
        queue,
        CameraDataLayout {
            proj_view: (projection.0 * transform.to_view_trans()).to_cols_array(),
            position_w: transform.get_translation().to_array(),
            direction_w: transform.get_look_vector().to_array(),
            ..Default::default()
        },
    );
}
