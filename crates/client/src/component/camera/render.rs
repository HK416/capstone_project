use hecs::{Entity, ViewBorrow, World};

use crate::component::{
    Camera, CameraDataLayout, CameraUniform, Child, Projection, Sibling, Skybox, SkyboxDataLayout,
    ToParentTrans, WorldTransform,
};

/// 카메라 변환 행렬 질의 타입
type Q<'a> = (
    &'a (Camera, ToParentTrans),
    &'a mut (Camera, WorldTransform),
);

/// 카메라 엔터티의 계층 구조를 갱신합니다.
pub fn update_camera_hierarchy(world: &mut World, entity: Entity, parent: glam::Mat4) {
    let child_view = world.view::<&Child>();
    let sibling_view = world.view::<&Sibling>();
    let mut transform_view = world.view::<Q>();
    update_camera_hierarchy_recursive(
        entity,
        parent,
        &child_view,
        &sibling_view,
        &mut transform_view,
    );
}

/// 카메라 엔터티의 계층 구조를 재귀적으로 갱신합니다.
fn update_camera_hierarchy_recursive(
    entity: Entity,
    parent: glam::Mat4,
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &mut ViewBorrow<'_, Q>,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티 계층 구조를 갱신합니다.
    if let Some(sibling) = sibling_view.get(entity).cloned() {
        let entity = *sibling;
        update_camera_hierarchy_recursive(entity, parent, child_view, sibling_view, transform_view);
    }

    // 현재 엔터티의 월드 변환 행렬을 갱신합니다.
    let ((_, local_transform), (_, world_transform)) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component!");
    let transform = parent * local_transform.0;
    world_transform.0 = transform;

    // 자식 엔터티가 존재하는 경우 자식 엔터티 계층 구조를 갱신합니다.
    if let Some(child) = child_view.get(entity).cloned() {
        let parent = transform;
        let entity = *child;
        update_camera_hierarchy_recursive(entity, parent, child_view, sibling_view, transform_view);
    }
}

/// 카메라 쉐이더 리소스와 스카이박스 쉐이더 리소스를 갱신합니다.
pub fn update_camera_and_skybox_resource(
    world: &World,
    camera: Entity,
    skybox: &Skybox,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) {
    // 카메라 엔터티의 요소를 가져옵니다.
    type Q<'a> = (
        &'a CameraUniform,
        &'a (Camera, WorldTransform),
        &'a Projection,
    );

    let mut query = world.query_one::<Q>(camera).expect("invalid entity!");
    let (camera_uniform, (_, world_transform), projection) =
        query.get().expect("invalid entity component!");

    // 카메라 유니폼 버퍼를 갱신합니다.
    let position_w = world_transform.get_translation();
    let proj_view = projection.0 * world_transform.to_view_trans();
    let data = CameraDataLayout {
        position_w: position_w.to_array(),
        proj_view: proj_view.to_cols_array(),
        ..Default::default()
    };
    camera_uniform.update(device, encoder, staging_buffers, data);

    // 스카이박스 유니폼 버퍼를 갱신합니다.
    let data = SkyboxDataLayout {
        proj_view: proj_view.to_cols_array(),
        color: [1.0, 1.0, 1.0],
        ..Default::default()
    };
    skybox
        .uniform
        .update(device, encoder, staging_buffers, data);
}
