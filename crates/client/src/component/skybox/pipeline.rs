//! 스카이박스를 그리는 렌더링, 컴퓨트 파이프라인과 관련된 코드를 관리합니다.
//!

use std::sync::OnceLock;

use crate::component::BrightRenderTarget;

use super::SkyboxResource;

/// 스카이박스를 그리는 렌더링 파이프라인입니다.
pub struct SkyboxRenderPipeline;

/// 스카이박스를 그리는 그래픽스 파이프라인 인스턴스입니다.
static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();

impl SkyboxRenderPipeline {
    /// [wgpu::ShaderModule]을 반환합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        unsafe {
            let desc = wgpu::include_wgsl!(concat!(
                env!("CARGO_WORKSPACE_DIR"),
                "/assets/shaders/skybox.wgsl",
            ));

            if cfg!(feature = "enable-shader-validation") {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
            } else {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
            }
        }
    }

    /// [wgpu::PipelineLayout]을 반환합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Skybox)"),
            bind_group_layouts: &[SkyboxResource::bind_group_layout(device)],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.  
    /// 렌더링 파이프라인이 초기화 되지 않은 경우 `None`을 반환합니다.
    pub fn get() -> Option<&'static wgpu::RenderPipeline> {
        PIPELINE.get()
    }

    /// 렌더링 파이프라인을 가져오거나 초기화합니다.
    pub fn get_or_init(
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> &'static wgpu::RenderPipeline {
        PIPELINE.get_or_init(|| {
            let module = Self::create_shader_module(device);
            let layout = Self::create_pipeline_layout(device);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    targets: &[
                        // 0번 렌더 타겟: 색상
                        Some(wgpu::ColorTargetState {
                            blend: None,
                            format: render_target_format,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        // 1번 렌더 타겟: bloom
                        Some(wgpu::ColorTargetState {
                            blend: None,
                            format: BrightRenderTarget::FORMAT,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                multiview: None,
                cache: None,
            })
        })
    }
}
