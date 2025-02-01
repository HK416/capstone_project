use std::sync::Arc;

use mod_render::{GraphicsPipelinePool, SkyboxResource};

/// Skybox 텍스처 상대경로입니다.
pub const WORKSPACE: &'static str = "stage";
/// Skybox 텍스처 이름입니다.
pub const TEXTURE_NAME: &'static str = "Sky";
/// Skybox 그래픽스 파이프라인의 이름입니다.
pub const SKYBOX_PIPELINE_NAME: &'static str = "Skybox";

/// 쉐이더 모듈을 생성합니다.
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/skybox.wgsl"
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
        label: Some("PipelineLayout(Terrain)"),
        bind_group_layouts: &[SkyboxResource::bind_group_layout(device)],
        push_constant_ranges: &[],
    })
}

/// Skybox 렌더링 파이프라인을 생성합니다.
pub fn create_skybox_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Skybox)"),
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
            depth_compare: wgpu::CompareFunction::LessEqual,
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

/// Skybox 큐브맵 이미지로 렌더타겟을 초기화합니다.
///
/// # Note
/// 이 함수는 모든 오브젝트를 그린 후 호출하는 것이 가장 성능이 좋습니다.
///
pub fn clear_render_target_with_skybox<'a>(
    skybox_resource: &'a SkyboxResource,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    // Skybox 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init(SKYBOX_PIPELINE_NAME, || {
        create_skybox_render_pipeline(device, depth_stencil_format, render_target_format)
    });
    rpass.set_pipeline(&pipeline);

    // Skybox 쉐이더 리소스를 랜더 패스에 바인드합니다.
    rpass.set_bind_group(0, &skybox_resource.bind_group, &[]);

    // 큐브의 정점 위치를 랜더 패스에 바인드합니다.
    rpass.set_vertex_buffer(0, skybox_resource.vertex_buffer.slice(..));

    // 큐브를 그립니다.
    rpass.draw(0..36, 0..1);
}
