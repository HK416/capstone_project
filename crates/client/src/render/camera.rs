use std::sync::Arc;

use hecs::{Entity, World};
use mod_physics::object3d::Frustum;
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
                    .query_one::<(&Arc<CameraResource>, &WorldTransform, &Projection, &mut Frustum)>(entity)
                    .expect("invalid entity");
                let (camera_resource, world_transform, projection, frustum) =
                    query.get().expect("invalid entity component");

                // 카메라 리소스를 갱신합니다.
                let proj_view = projection.0 * world_transform.to_view_trans();
                camera_resource.camera_uniform.update(
                    device,
                    queue,
                    CameraDataLayout {
                        proj_view: proj_view.to_cols_array(),
                        position_w: world_transform.get_translation().to_array(),
                        direction_w: world_transform.get_look_vector().to_array(),
                        ..Default::default()
                    },
                );

                // 카메라 절두체를 갱신합니다.
                *frustum = Frustum::from_mat4(proj_view);
            });
        }
    });
}
