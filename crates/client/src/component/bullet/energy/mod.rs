//! 에너지 볼 형태의 총알 객체와 관련된 코드를 관리합니다.
//!

mod pipeline;

use mod_render::DEPTH_FORMAT;

use crate::component::{AttributeKind, CameraResource, LightSetResource, MaterialMap, Mesh};

pub use self::pipeline::*;

/// 에너지 볼 형태의 총알을 그립니다.
pub fn draw_energy_bullet<'a>(
    mesh: &'a Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    light_resource: &'a LightSetResource,
    material_resources: &'a MaterialMap,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(EnergyBulletRenderPipeline::get_or_init(
        device,
        DEPTH_FORMAT,
    ));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(3, light_resource.bind_group(), &[]);

    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());

    for ((index, material), filters) in material_resources {
        let index_buffer = mesh.submeshes().get(*index).unwrap();
        rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
        rpass.set_bind_group(2, material.bind_group(), &[]);

        for resource in filters {
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}
