//! 스카이박스와 관련된 코드를 관리합니다.
//!

mod cube;
mod pipeline;
mod resource;
mod uniform;

use wgpu::util::DeviceExt;

pub use self::{cube::*, pipeline::*, resource::*, uniform::*};

/// 큐브 형태의 스카이박스입니다.
#[derive(Debug)]
pub struct Skybox {
    pub vertex: wgpu::Buffer,
    pub uniform: SkyboxUniform,
    pub resource: SkyboxResource,
}

impl Skybox {
    /// 새로운 스카이박스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) -> Self {
        // 스테이징(업로드) 버퍼를 생성합니다.
        let contents = bytemuck::cast_slice(&CUBE_POSITIONS);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Staging(Vertex({}))", label.unwrap_or("Unknown"))),
            contents,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 정점 버퍼를 생성합니다.
        let vertex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Vertex({})", label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: copy_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // 버퍼 데이터를 복사합니다.
        encoder.copy_buffer_to_buffer(&staging, 0, &vertex, 0, copy_size);
        staging_buffers.push(staging);

        // 유니폼 버퍼를 생성합니다.
        let uniform = SkyboxUniform::uninit(label, device);
        // 쉐이더 리소스를 생성합니다.
        let resource = SkyboxResource::new(label, device, &uniform, texture, sampler);

        Self {
            vertex,
            uniform,
            resource,
        }
    }
}

/// 스카이박스로 렌더 타겟을 초기화합니다.
///
/// # Note
/// 이 함수는 그리기 마지막에 호출하는 것이 가장 성능이 좋습니다.
///
pub fn clear_render_target_with_skybox<'a>(
    skybox: &'a Skybox,
    pipeline: &'a wgpu::RenderPipeline,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    rpass.set_pipeline(&pipeline);
    rpass.set_vertex_buffer(0, skybox.vertex.slice(..));
    rpass.set_bind_group(0, skybox.resource.bind_group(), &[]);
    rpass.draw(0..NUM_CUBE_VERTICES as u32, 0..1);
}
