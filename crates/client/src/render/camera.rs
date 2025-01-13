use std::sync::Arc;

use glam::Vec4Swizzles;
use hecs::{Entity, World};
use mod_render::{CameraDataLayout, CameraResource};

use crate::component::{Projection, WorldTransform};

/// 카메라 쉐이더 리소스를 준비합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 카메라의 월드 변환 행렬이 먼저 계산되어야합니다.
///
/// # Panics
/// - 주어진 카메라 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 카메라 엔터티는 카메라 쉐이더 리소스(`Arc<CameraResource>`), 월드 변환 행렬(`WorldTransform`),
/// 투영 변환 행렬(`Projection`)을 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn prepare_camera_resource(
    world: &World,
    camera_entities: &[Entity],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    rayon::in_place_scope(|scope| {
        for &entity in camera_entities {
            scope.spawn(move |_| {
                // 엔터티에서 카메라 리소스와 월드 변환 행렬, 투영 변환 행렬을 가져옵니다.
                let mut query = world
                    .query_one::<(&Arc<CameraResource>, &WorldTransform, &Projection)>(entity)
                    .expect("invalid entity");
                let (camera_resource, world_transform, projection) =
                    query.get().expect("invalid entity component");

                // 카메라 리소스를 갱신합니다.
                camera_resource.camera_uniform.update(
                    device,
                    queue,
                    CameraDataLayout {
                        proj_view: (projection.0 * world_transform.to_view_trans()).to_cols_array(),
                        position_w: world_transform.get_translation().xyz().to_array(),
                        direction_w: world_transform.get_look_vector().xyz().to_array(),
                        ..Default::default()
                    },
                );
            });
        }
    });
}
