//! 데미지 폰트를 그리는 렌더링, 컴퓨트 파이프라인과 관련된 코드를 관리합니다.
//!

use std::sync::{Arc, OnceLock};

use crate::component::CameraResource;

use super::DamageFontResource;

/// 데미지 폰트를 그리는 그래픽스 파이프라인입니다.
pub struct DamageFontRenderPipeline;

/// 데미지 폰트를 그리는 그래픽스 파이프라인 인스턴스입니다.
static PIPELINE: OnceLock<Arc<wgpu::RenderPipeline>> = OnceLock::new();

impl DamageFontRenderPipeline {
    /// [wgpu::ShaderModule]을 반환합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        unsafe {
            let desc = wgpu::include_wgsl!(concat!(
                env!("CARGO_WORKSPACE_DIR"),
                "/assets/shaders/damage_font.wgsl"
            ));

            if cfg!(feature = "enable-shader-validation") {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
            } else {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
            }
        }
    }

    /// 파이프라인 레이아웃을 생성합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(DamageFont)"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                DamageFontResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.  
    /// 렌더링 파이프라인이 초기화 되지 않은 경우 `None`을 반환합니다.
    pub fn get() -> Option<Arc<wgpu::RenderPipeline>> {
        PIPELINE.get().cloned()
    }

    /// 렌더링 파이프라인을 가져오거나 초기화합니다.
    pub fn get_or_init(
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> Arc<wgpu::RenderPipeline> {
        PIPELINE
            .get_or_init(|| {
                let module = Self::create_shader_module(device);
                let layout = Self::create_pipeline_layout(device);
                Arc::new(
                    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("RenderPipeline(DamageFont)"),
                        layout: Some(&layout),
                        vertex: wgpu::VertexState {
                            module: &module,
                            entry_point: Some("vs_main"),
                            buffers: &[
                                wgpu::VertexBufferLayout {
                                    array_stride: core::mem::size_of::<[f32; 3]>()
                                        as wgpu::BufferAddress,
                                    attributes: &[wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 0,
                                        format: wgpu::VertexFormat::Float32x3,
                                    }],
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                },
                                wgpu::VertexBufferLayout {
                                    array_stride: core::mem::size_of::<[f32; 2]>()
                                        as wgpu::BufferAddress,
                                    attributes: &[wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 1,
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
                    }),
                )
            })
            .clone()
    }
}
