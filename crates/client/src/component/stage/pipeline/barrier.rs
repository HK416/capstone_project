#![allow(dead_code)]
//! 방어막과 관련된 그래픽스, 컴퓨트 파이프라인 코드를 관리합니다.
//!

use std::sync::OnceLock;

use crate::component::{
    AccumRenderTarget, BrightRenderTarget, CameraResource, MeshResource, RevealRenderTarget,
    StageBarrierMaterialResource,
};

/// 방어막을 그리는 그래픽스 파이프라인입니다.
pub struct StageBarrierRenderPipeline;

/// 그래픽스 파이프라인의 인스턴스입니다.
static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();

impl StageBarrierRenderPipeline {
    /// 쉐이더 모듈을 생성합니다.
    fn create_shader_module(device: &wgpu::Device) -> &'static wgpu::ShaderModule {
        static MODULE: OnceLock<wgpu::ShaderModule> = OnceLock::new();
        MODULE.get_or_init(|| unsafe {
            let desc = wgpu::include_wgsl!(concat!(
                env!("CARGO_WORKSPACE_DIR"),
                "/assets/shaders/stage_barrier.wgsl"
            ));

            if cfg!(feature = "enable-shader-validation") {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
            } else {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
            }
        })
    }

    /// 파이프라인 레이아웃을 생성합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Tree)"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                MeshResource::bind_group_layout(device),
                StageBarrierMaterialResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
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
                label: Some("RenderPipeline(StageBarrier)"),
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
                        // 1번 입력 속성: 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
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
                    cull_mode: None,
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
