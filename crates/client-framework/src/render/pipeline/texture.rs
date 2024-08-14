use std::ops;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::render::camera::CameraObject;
use crate::render::material::Material;
use crate::render::object::GameObject;
use crate::render::targets::DepthBuffer;
use crate::render::targets::SWAPCHAIN_FORMAT;



/// 3차원 메쉬에 텍스처를 매핑하여 출력하는 그래픽스 파이프라인입니다.
#[derive(Debug)]
pub struct TexturePipeline(wgpu::RenderPipeline);

impl TexturePipeline {
    /// 쉐이더 모듈을 반환합니다.
    #[must_use]
    pub fn shader(device: &wgpu::Device) -> &'static wgpu::ShaderModule {
        static SHADER: OnceLock<wgpu::ShaderModule> = OnceLock::new();
        SHADER.get_or_init(|| {
            device.create_shader_module(wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/texture.wgsl")))
        })
    }

    /// 파이프라인 레이아웃을 반환합니다.
    #[must_use]
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::PipelineLayout {
        static LAYOUT: OnceLock<wgpu::PipelineLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("PipelineLayout(TexturePipeline)"), 
                    bind_group_layouts: &[
                        // 0번 그룹: 카메라 데이터
                        CameraObject::layout(device), 
                        // 1번 그룹: 오브젝트 데이터
                        GameObject::layout(device), 
                        // 2번 그룹: 재질 데이터
                        Material::layout(device), 
                    ], 
                    push_constant_ranges: &[]
                }
            )
        })
    }
}

impl TexturePipeline {
    /// 텍스처 파이프라인을 생성합니다.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Arc<Self> {
        Self(device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(TexturePipeline)"), 
                layout: Some(&Self::layout(device)), 
                vertex: wgpu::VertexState {
                    module: &Self::shader(device), 
                    entry_point: "vs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    buffers: &[
                        // 정점의 위치
                        wgpu::VertexBufferLayout {
                            array_stride: 12, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3, 
                                    offset: 0, 
                                }, 
                            ], 
                        }, 
                        // 정점의 0번 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: 8, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    shader_location: 1, 
                                    format: wgpu::VertexFormat::Float32x2, 
                                    offset: 0, 
                                }, 
                            ], 
                        }, 
                    ],
                }, 
                primitive: wgpu::PrimitiveState {
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    front_face: wgpu::FrontFace::Ccw, 
                    cull_mode: Some(wgpu::Face::Back), 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState {
                    depth_compare: wgpu::CompareFunction::Less, 
                    format: DepthBuffer::FORMAT, 
                    depth_write_enabled: true, 
                    stencil: wgpu::StencilState::default(), 
                    bias: wgpu::DepthBiasState::default()
                }), 
                multisample: wgpu::MultisampleState::default(), 
                fragment: Some(wgpu::FragmentState {
                    module: &Self::shader(device), 
                    entry_point: "fs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            blend: None, 
                            format: SWAPCHAIN_FORMAT, 
                            write_mask: wgpu::ColorWrites::all()
                        }), 
                    ], 
                }), 
                multiview: None, 
                cache: None, 
            }, 
        )).into()
    }
}

impl ops::Deref for TexturePipeline {
    type Target = wgpu::RenderPipeline;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for TexturePipeline {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
