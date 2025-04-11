//! 카메라와 관련된 코드를 관리합니다.
//!

mod resource;
mod third_person;
mod uniform;

use hecs::{Entity, World};
use mod_physics::object3d::Frustum;

use crate::component::{Projection, WorldTransform};

pub use self::{resource::*, third_person::*, uniform::*};

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
    entity: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) {
    type Query<'a> = (
        &'a CameraUniform,
        &'a WorldTransform,
        &'a Projection,
        &'a mut Frustum,
    );
    let mut query = world.query_one::<Query>(entity).expect("invalid entity");

    let (camera_uniform, world_transform, projection, frustum) =
        query.get().expect("invalid entity component");
    let position_w = world_transform.get_translation();
    let proj_view = projection.0 * world_transform.to_view_trans();

    // 카메라 유니폼 버퍼를 갱신합니다.
    camera_uniform.update(
        device,
        encoder,
        staging_buffers,
        CameraDataLayout {
            proj_view: proj_view.to_cols_array(),
            position_w: position_w.to_array(),
            ..Default::default()
        },
    );

    // 카메라 절두체를 갱신합니다.
    *frustum = Frustum::from_mat4(proj_view);
}
