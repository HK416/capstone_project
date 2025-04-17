//! 스테이지와 관련된 그래픽스, 컴퓨트 파이프라인 코드를 관리합니디.
//!

use std::sync::{Arc, OnceLock};

use crate::component::{CameraResource, MeshResource, StageMaterialResource};

/// 쉐이더 모듈을 생성합니다.
fn create_shadoer_module(device: &wgpu::Device) -> &'static wgpu::ShaderModule {
    static MODULE: OnceLock<wgpu::ShaderModule> = OnceLock::new();
    MODULE.get_or_init(|| unsafe {
        let desc = wgpu::include_wgsl!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/shaders/stage.wgsl",
        ));

        if cfg!(feature = "enable-shader-validation") {
            device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
        } else {
            device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
        }
    })
}

/// 스테이지를 그리는 그래픽스 파이프라인입니다.
pub struct StageRenderPipeline;

/// 스테이지를 그리는 그래픽스 파이프라인 인스턴스입니다.
static PIPELINE: OnceLock<Arc<wgpu::RenderPipeline>> = OnceLock::new();

impl StageRenderPipeline {
    /// 파이프라인 레이아웃을 생성합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Stage)"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                MeshResource::bind_group_layout(device),
                StageMaterialResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.  
    /// 렌더링 파이프라인이 초기화 되지 않은 상태일 경우 `None`을 반환합니다.
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
                let module = create_shadoer_module(device);
                let layout = Self::create_pipeline_layout(device);
                Arc::new(
                    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("RenderPipeline(Stage)"),
                        layout: Some(&layout),
                        vertex: wgpu::VertexState {
                            module,
                            entry_point: Some("vs_main"),
                            buffers: &[
                                // 0번 입력 속성: 위치
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
                                // 1번 입력 속성: 노멀
                                wgpu::VertexBufferLayout {
                                    array_stride: core::mem::size_of::<[f32; 3]>()
                                        as wgpu::BufferAddress,
                                    attributes: &[wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 1,
                                        format: wgpu::VertexFormat::Float32x3,
                                    }],
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                },
                                // 2번 입력 속성: 0번 텍스처 좌표
                                wgpu::VertexBufferLayout {
                                    array_stride: core::mem::size_of::<[f32; 2]>()
                                        as wgpu::BufferAddress,
                                    attributes: &[wgpu::VertexAttribute {
                                        offset: 0,
                                        shader_location: 2,
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
