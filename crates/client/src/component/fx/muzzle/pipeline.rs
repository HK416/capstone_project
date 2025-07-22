#![allow(dead_code)]
//! 총구 화염 파티클 렌더링 파이프라인을 관리합니다.
//!

use std::{mem::offset_of, sync::OnceLock};

use mod_render::DEPTH_FORMAT;

use crate::component::{
    AccumRenderTarget, AttributeKind, BrightRenderTarget, CameraResource, FxMuzzleInstance,
    FxMuzzleInstanceDataLayout, FxMuzzleResource, Mesh, ParticleResource, RevealRenderTarget,
};

/// 총구 화염 파티클을 그리는 렌더링 파이프라인입니다.
pub struct FxMuzzleRenderPipeline;

static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();

impl FxMuzzleRenderPipeline {
    /// [wgpu::ShaderModule]을 반환합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        unsafe {
            let desc = wgpu::include_wgsl!(concat!(
                env!("CARGO_WORKSPACE_DIR"),
                "/assets/shaders/fx_muzzle.wgsl",
            ));

            if cfg!(feature = "enable-shader-validation") {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
            } else {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
            }
        }
    }

    /// [wgpu::PipelineLayout]을 반환합니다.
    fn get_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Fx(Muzzle))"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                FxMuzzleResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.
    /// 렌더링 파이프라인이 초기화되지 않은 경우 `None`을 반환합니다.
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
            let layout = Self::get_pipeline_layout(device);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(Fx(Muzzle))"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        // 0번 입력 속성: 정점 위치
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            }],
                            step_mode: wgpu::VertexStepMode::Vertex,
                        },
                        // 1번 입력 속성: 정점 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            }],
                            step_mode: wgpu::VertexStepMode::Vertex,
                        },
                        // 2번 입력 속성: 인스턴스 데이터
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<FxMuzzleInstanceDataLayout>()
                                as wgpu::BufferAddress,
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: offset_of!(FxMuzzleInstanceDataLayout, x_axis)
                                        as wgpu::BufferAddress,
                                    shader_location: 2,
                                    format: wgpu::VertexFormat::Float32x4,
                                },
                                wgpu::VertexAttribute {
                                    offset: offset_of!(FxMuzzleInstanceDataLayout, y_axis)
                                        as wgpu::BufferAddress,
                                    shader_location: 3,
                                    format: wgpu::VertexFormat::Float32x4,
                                },
                                wgpu::VertexAttribute {
                                    offset: offset_of!(FxMuzzleInstanceDataLayout, z_axis)
                                        as wgpu::BufferAddress,
                                    shader_location: 4,
                                    format: wgpu::VertexFormat::Float32x4,
                                },
                                wgpu::VertexAttribute {
                                    offset: offset_of!(FxMuzzleInstanceDataLayout, w_axis)
                                        as wgpu::BufferAddress,
                                    shader_location: 5,
                                    format: wgpu::VertexFormat::Float32x4,
                                },
                                wgpu::VertexAttribute {
                                    offset: offset_of!(FxMuzzleInstanceDataLayout, tint)
                                        as wgpu::BufferAddress,
                                    shader_location: 6,
                                    format: wgpu::VertexFormat::Float32x3,
                                },
                                wgpu::VertexAttribute {
                                    offset: offset_of!(FxMuzzleInstanceDataLayout, index)
                                        as wgpu::BufferAddress,
                                    shader_location: 7,
                                    format: wgpu::VertexFormat::Uint32,
                                },
                            ],
                            step_mode: wgpu::VertexStepMode::Instance,
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    front_face: wgpu::FrontFace::Cw,
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    strip_index_format: None,
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

/// 총구 화염 이펙트를 그립니다.
pub fn draw_fx_muzzle_effect<'a>(
    mesh: &Mesh,
    device: &wgpu::Device,
    camera_resource: &'a CameraResource,
    particle_resource: &'a ParticleResource,
    instance_buffer: &'a FxMuzzleInstance,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    if instance_buffer.num_instance() == 0 {
        return;
    }

    rpass.set_pipeline(FxMuzzleRenderPipeline::get_or_init(device, DEPTH_FORMAT));

    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
    rpass.set_bind_group(1, particle_resource.bind_group(), &[]);
    rpass.set_vertex_buffer(0, mesh.vertex(..));
    rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
    rpass.set_vertex_buffer(2, instance_buffer.slice());
    rpass.draw(0..mesh.num_vertices(), 0..instance_buffer.num_instance());
}
