#![allow(dead_code)]
//! 에너지 볼 형태의 총알과 관련된 그래픽스, 컴퓨트 파이프라인 코드를 관리합니다.
//!

use std::sync::OnceLock;

use crate::component::{
    AccumRenderTarget, BrightRenderTarget, CameraResource, EnergyBulletMaterialResource,
    LightSetResource, MeshResource, RevealRenderTarget,
};

/// 에너지 볼 형태의 총알을 그리는 그래픽스 파이프라인입니다.
pub struct EnergyBulletRenderPipeline;

/// 에너지 볼 형태의 총알을 그리는 그래픽스 파이프라인 인스턴스입니다.
static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();

impl EnergyBulletRenderPipeline {
    /// [wgpu::ShaderModule]을 반환합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        unsafe {
            let desc = wgpu::include_wgsl!(concat!(
                env!("CARGO_WORKSPACE_DIR"),
                "/assets/shaders/energy_bullet.wgsl",
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
            label: Some("PipelineLayout(EnergyBullet)"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                MeshResource::bind_group_layout(device),
                EnergyBulletMaterialResource::bind_group_layout(device),
                LightSetResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.  
    /// 렌더링 파이프라인이 초기화되지 않은 상태일 경우 `None`을 반환합니다.
    pub fn get() -> Option<&'static wgpu::RenderPipeline> {
        PIPELINE.get()
    }

    /// 렌더링 파이프라인을 가져오거나 초기화합니다.
    pub fn get_or_init(
        device: &wgpu::Device,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> &'static wgpu::RenderPipeline {
        PIPELINE.get_or_init(|| {
            let module = Self::create_shader_module(device);
            let layout = Self::create_pipeline_layout(device);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(EnergyBullet)"),
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
                    depth_write_enabled: false,
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
                        // 0번 렌더 타겟: 누적 값 렌더 타겟
                        Some(wgpu::ColorTargetState {
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            format: AccumRenderTarget::FORMAT,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        // 1번 렌더 타겟: 노출 값 렌더 타겟
                        Some(wgpu::ColorTargetState {
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::Zero,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::Zero,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            format: RevealRenderTarget::FORMAT,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        // 2번 렌더 타겟: 발광체 색상 렌더 타겟
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
