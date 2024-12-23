use std::sync::Arc;

use hecs::{Entity, World};
use mod_render::{CameraDataLayout, CameraResource};

use crate::component::{Projection, WorldTransform};

use super::update_entity_hierarchy;

/// 3인칭 카메라를 갱신합니다.
///
/// 주어진 `target` 엔터티의 위치를 기준으로 카메라의 월드 변환 행렬이 계산됩니다.
///
/// # Note
/// - 주어진 `target` 엔터티의 월드 변환 행렬이 먼저 계산되어야 합니다.
///
/// # Panics
/// - 주어진 `target` 엔터티는 월드 변환 행렬(`WorldTransform`)을 갖고 있어야 합니다.
/// 그렇지 않은 경우 [`panic!`]을 호출합니다.
/// - 주어진 `camera` 엔터티는 로컬 변환 행렬(`ToParentTrans`), 월드 변환 행렬(`WorldTransform`),
/// 식별자(`CameraTag`)를 갖고 있어야 합니다. 그렇지 않은 경우 [`panic!`]을 호출합니다.
///
pub fn update_third_person_camera(world: &mut World, target_entity: Entity, camera_entity: Entity) {
    // `target` 엔터티의 월드 변환 행렬로부터 `target`의 위치를 가져옵니다.
    let world_transform = world
        .query_one_mut::<&WorldTransform>(target_entity)
        .expect("invalid entity component");
    let translation = world_transform.get_translation();

    // 부모 변환 행렬을 생성합니다.
    let parent_transform = glam::Mat4::from_translation(translation);

    // 카메라의 월드 변환 행렬을 갱신합니다.
    update_entity_hierarchy(world, camera_entity, parent_transform);
}

/// 주어진 엔터티의 카메라 리소스를 준비합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 카메라 리소스(`Arc<CameraResource>`), 월드 변환 행렬(`WorldTransform`),
/// 투영 변환 행렬(`Projection`)을 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn prepare_camera_resource(
    world: &World,
    entities: &[Entity],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    rayon::in_place_scope(|scope| {
        for &entity in entities {
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
                        position_w: world_transform.get_translation().to_array(),
                        direction_w: world_transform.get_look_vector().to_array(),
                        ..Default::default()
                    },
                );
            });
        }
    });
}
