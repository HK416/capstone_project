//! 일반 총알과 관련된 그래픽스, 컴퓨트 파이프라인 코드를 관리합니다.
//!

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use crate::component::{BulletMaterialResource, CameraResource, MeshResource, MAX_LIGHTS};

/// 일반 총알을 그리는 그래픽스 파이프라인입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulletRenderPipeline(Arc<wgpu::RenderPipeline>);

impl BulletRenderPipeline {
    /// [wgpu::ShaderModule]을 반환합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        unsafe {
            let desc = wgpu::include_wgsl!(concat!(
                env!("CARGO_WORKSPACE_DIR"),
                "/assets/shaders/bullet.wgsl",
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
            label: Some("PipelineLayout(Bullet)"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                MeshResource::bind_group_layout(device),
                BulletMaterialResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.
    pub fn get(
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> Self {
        static PIPELINE: OnceLock<Arc<wgpu::RenderPipeline>> = OnceLock::new();
        let pipeline = PIPELINE
            .get_or_init(|| {
                let module = Self::create_shader_module(device);
                let layout = Self::create_pipeline_layout(device);
                let constants = HashMap::from_iter([("max_lights".into(), MAX_LIGHTS as f64)]);
                Arc::new(
                    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("RenderPipeline(Bullet)"),
                        layout: Some(&layout),
                        vertex: wgpu::VertexState {
                            module: &module,
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
                            ],
                            compilation_options: wgpu::PipelineCompilationOptions {
                                constants: &constants,
                                ..Default::default()
                            },
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
            .clone();

        Self(pipeline)
    }
}
