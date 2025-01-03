use std::sync::Arc;

use glam::FloatExt;
use hecs::{Entity, World};
use mod_render::{CameraDataLayout, CameraResource};

use crate::component::{
    Projection, ThirdPersonCamera, Timer, ToParentTrans, ViewState, WorldTransform, MAX_IN_OUT_TIME,
};

use super::update_entity_hierarchy;

/// 3인칭 카메라를 회전시킵니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 삼인칭 카메라(`ThirdPersonCamera`)를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn rotate_third_person_camera(
    world: &mut World,
    camera_entity: Entity,
    dx: f32,
    dy: f32,
    offset: f32,
) {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&mut ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 삼인칭 카메라를 갱신합니다.
    third_person_camera.update_direction(dx, dy, offset);
}

/// 3인칭 카메라의 거리와 오프셋을 갱신합니다.
///
/// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 삼인칭 카메라 요소(`ThirdPersonCamera`)를 갖고 있어야 합니다.
/// 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_third_person_camera(
    world: &mut World,
    camera_entity: Entity,
    view_state: ViewState,
    view_state_timer: Timer,
    idle_offset: glam::Vec4,
    idle_distance: f32,
    aiming_offset: glam::Vec4,
    aiming_distance: f32,
) {
    const FUNC_TABLE: [fn(&mut ThirdPersonCamera, Timer, glam::Vec4, f32, glam::Vec4, f32); 4] = [
        update_third_person_camera_when_idle_state,
        update_third_person_camera_when_zoom_in_state,
        update_third_person_camera_when_zoom_out_state,
        update_third_person_camera_when_aiming_state,
    ];

    // 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&mut ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    let index = view_state as usize;
    FUNC_TABLE[index](
        third_person_camera,
        view_state_timer,
        idle_offset,
        idle_distance,
        aiming_offset,
        aiming_distance,
    );
}

/// `ViewState::Idle`일 때 삼인칭 카메라를 갱신합니다.
fn update_third_person_camera_when_idle_state(
    third_person_camera: &mut ThirdPersonCamera,
    _view_state_timer: Timer,
    idle_offset: glam::Vec4,
    idle_distance: f32,
    _aiming_offset: glam::Vec4,
    _aiming_distance: f32,
) {
    third_person_camera.position_offset = idle_offset;
    third_person_camera.distance = idle_distance;
}

/// `ViewState::ZoomIn`일 때 삼인칭 카메라를 갱신합니다.
fn update_third_person_camera_when_zoom_in_state(
    third_person_camera: &mut ThirdPersonCamera,
    view_state_timer: Timer,
    idle_offset: glam::Vec4,
    idle_distance: f32,
    aiming_offset: glam::Vec4,
    aiming_distance: f32,
) {
    let t = view_state_timer.0 / MAX_IN_OUT_TIME;
    let position_offset = idle_offset.lerp(aiming_offset, t);
    let distance = idle_distance.lerp(aiming_distance, t);
    third_person_camera.position_offset = position_offset;
    third_person_camera.distance = distance;
}

/// `ViewState::ZoomOut`일 때 삼인칭 카메라를 갱신합니다.
fn update_third_person_camera_when_zoom_out_state(
    third_person_camera: &mut ThirdPersonCamera,
    view_state_timer: Timer,
    idle_offset: glam::Vec4,
    idle_distance: f32,
    aiming_offset: glam::Vec4,
    aiming_distance: f32,
) {
    let t = view_state_timer.0 / MAX_IN_OUT_TIME;
    let position_offset = aiming_offset.lerp(idle_offset, t);
    let distance = aiming_distance.lerp(idle_distance, t);
    third_person_camera.position_offset = position_offset;
    third_person_camera.distance = distance;
}

/// `ViewState::Aiming`일 때 삼인칭 카메라를 갱신합니다.
fn update_third_person_camera_when_aiming_state(
    third_person_camera: &mut ThirdPersonCamera,
    _view_state_timer: Timer,
    _idle_offset: glam::Vec4,
    _idle_distance: f32,
    aiming_offset: glam::Vec4,
    aiming_distance: f32,
) {
    third_person_camera.position_offset = aiming_offset;
    third_person_camera.distance = aiming_distance;
}

/// 3인칭 카메라의 월드 변환 행렬을 계산합니다.
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
/// 삼인칭 카메라(`ThirdPersonCamera`)를 갖고 있어야 합니다. 그렇지 않은 경우 [`panic!`]을 호출합니다.
///
pub fn update_third_person_camera_hierarchy(
    world: &mut World,
    target_entity: Entity,
    camera_entity: Entity,
) {
    // `target` 엔터티의 월드 변환 행렬로부터 `target`의 위치를 가져옵니다.
    let world_transform = world
        .query_one_mut::<&WorldTransform>(target_entity)
        .expect("invalid entity or invalid entity component");
    let translation = world_transform.get_translation();

    // 부모 변환 행렬을 생성합니다.
    let parent_transform = glam::Mat4::from_translation(translation);

    // 카메라 엔터티의 로컬 변환 행렬을 갱신합니다.
    let (local_transform, third_person_camera) = world
        .query_one_mut::<(&mut ToParentTrans, &ThirdPersonCamera)>(camera_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = third_person_camera.to_matrix();

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
