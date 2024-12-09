use hecs::{Entity, QueryOneError, World};
use mod_app::asset::AssetManager;
use mod_render::{CameraResource, GraphicsPipelinePool};

use crate::component::{create_student_render_pipeline, update_hierarchy, StudentTag};

/// 학생의 모델 계층 구조를 갱신합니다.
pub fn sys_student_hierarchy(world: &mut World) -> Result<(), QueryOneError> {
    // 학생 태그를 가진 `Entity`를 수집합니다.
    let entities: Vec<Entity> = world
        .query_mut::<&StudentTag>()
        .into_iter()
        .map(|(entity, _)| entity)
        .collect();

    // 최상위 `Entity`부터 최하위 `Entity`까지 월드 변환 행렬을 갱신합니다.
    for entity in entities {
        update_hierarchy(world, entity, glam::Mat4::IDENTITY)?;
    }

    Ok(())
}

/// 학생 모델을 애니메이션합니다.
pub fn sys_student_animation(
    world: &mut World,
    asset_manager: &AssetManager,
    elapsed_time_sec: f32,
) {
    aris_original::sys_aris_original_animation(world, asset_manager, elapsed_time_sec);
}

/// 학생 모델을 그립니다.
pub fn sys_student_draw<'a>(
    world: &'a World,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    camera: &'a CameraResource,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    let pipeline = GraphicsPipelinePool::get_or_init("student", || {
        create_student_render_pipeline(device, depth_stencil_format, render_target_format)
    });

    // 렌더링 파이프라인을 바인드합니다.
    rpass.set_pipeline(&pipeline);

    sys_aris_original_draw(world, camera, rpass);
}

mod aris_original;

use self::aris_original::*;
