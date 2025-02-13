use std::sync::Arc;

use hecs::{With, World};
use mod_network::components::BulletKind;
use mod_render::{
    AttributeKind, CameraResource, GraphicsPipelinePool, MaterialResource, Mesh, MeshResource,
};

pub const BULLET_PIPELINE_NAME: &'static str = "Bullet";

/// 쉐이더 모듈을 생성합니다.
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/bullet.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }
    }
}

/// 파이프라인 레이아웃을 생성합니다.
fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PipelineLayout(Bullet)"),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            MeshResource::bind_group_layout(device),
            MaterialResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 총알 모델 렌더링 파이프라인을 생성합니다.
pub fn create_bullet_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Bullet)"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[
                // 0번 입력 속성: 위치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 1번 입력 속성: 노멀
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 2번 입력 속성: 탄젠트 공간 노멀
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 3번 입력 속성: 0번 텍스처 좌표
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::Less,
            depth_write_enabled: true,
            format: depth_stencil_format,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                blend: None,
                format: render_target_format,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}

pub fn draw_bullet<'a>(
    world: &'a World,
    camera_resource: &'a CameraResource,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    type Query<'a> = (
        &'a Arc<Mesh>,
        &'a Arc<MeshResource>,
        &'a Vec<Arc<MaterialResource>>,
    );
    let mut query = world.query::<With<Query, &BulletKind>>();
    for (_, (mesh, mesh_resource, materials)) in query.iter() {
        // 총알 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
        let pipeline = GraphicsPipelinePool::get_or_init(BULLET_PIPELINE_NAME, || {
            create_bullet_render_pipeline(device, depth_stencil_format, render_target_format)
        });
        rpass.set_pipeline(&pipeline);

        // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
        rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

        // 메쉬 쉐이더 리소스를 렌더 패스에 바인드합니다.
        rpass.set_bind_group(1, &mesh_resource.bind_group, &[]);

        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Tangent, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

        for (index, submesh) in mesh.submeshes().iter().enumerate() {
            // 메쉬의 인덱스 버퍼를 바인드합니다.
            rpass.set_index_buffer(submesh.slice(..), submesh.format());

            // 재질의 쉐이더 리소스를 바인드합니다.
            rpass.set_bind_group(2, &materials[index].bind_group, &[]);

            // 인덱스 버퍼를 사용하여 그립니다.
            rpass.draw_indexed(0..submesh.count(), 0, 0..1);
        }
    }
}
