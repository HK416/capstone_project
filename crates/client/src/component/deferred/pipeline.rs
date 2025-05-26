//! 여러 렌더 타겟 데이터를 취합하는 렌더링 파이프라인과 관련된 코드를 관리합니다.
//!

use std::sync::OnceLock;

use wgpu::util::DeviceExt;

use super::{AccumRenderTarget, RevealRenderTarget};

/// 여러 렌더 타겟의 데이터를 취합하는 렌더링 파이프라인입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct CompositePipeline {
    vertex: wgpu::Buffer,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl CompositePipeline {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Composite)"),
                entries: &[
                    // 0번 바인딩: 누적 값(Accumuldate) 렌더 타겟
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: 노출 값(Revalage) 렌더 타겟
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2번 바인딩: 렌더 타겟 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
        })
    }

    /// [wgpu::ShaderModule]을 생성합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        let desc = wgpu::include_wgsl!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/shaders/composite.wgsl"
        ));

        unsafe {
            if cfg!(feature = "enable-shader-validation") {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
            } else {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
            }
        }
    }

    /// [wgpu::PipelineLayout]을 생성합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Composite)"),
            bind_group_layouts: &[Self::bind_group_layout(device)],
            push_constant_ranges: &[],
        })
    }

    /// [wgpu::RenderPipeline]을 생성합니다.
    fn create_render_pipeline(
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let module = Self::create_shader_module(device);
        let layout = Self::create_pipeline_layout(device);
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RenderPipeline(Composite)"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    step_mode: wgpu::VertexStepMode::Vertex,
                    array_stride: core::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Cw,
                polygon_mode: wgpu::PolygonMode::Fill,
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                bias: wgpu::DepthBiasState::default(),
                depth_compare: wgpu::CompareFunction::Always,
                depth_write_enabled: false,
                format: depth_stencil_format,
                stencil: wgpu::StencilState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    format: render_target_format,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    /// 여러 렌더 타겟을 취합하는데 사용되는 사각형 정점 버퍼를 생성합니다.
    fn create_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        const VERTICES: [[f32; 4]; 4] = [
            [-1.0, -1.0, 0.0, 1.0],
            [-1.0, 1.0, 0.0, 0.0],
            [1.0, -1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 0.0],
        ];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex(Composite)"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// 렌더 타겟 텍스처 샘플러를 생성합니다.
    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler(Composite)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        accum_render_target: &AccumRenderTarget,
        reveal_render_target: &RevealRenderTarget,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Composite)"),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(accum_render_target.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(reveal_render_target.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// 새로운 렌더 타겟 취합 파이프라인을 생성합니다.
    pub fn new(
        device: &wgpu::Device,
        accum_render_target: &AccumRenderTarget,
        reveal_render_target: &RevealRenderTarget,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> Self {
        let vertex = Self::create_vertex_buffer(device);
        let sampler = Self::create_sampler(device);
        let bind_group =
            Self::create_bind_group(device, accum_render_target, reveal_render_target, &sampler);
        let pipeline =
            Self::create_render_pipeline(device, render_target_format, depth_stencil_format);
        Self {
            vertex,
            sampler,
            bind_group,
            pipeline,
        }
    }

    /// 기존 파이프라인으로부터 새로운 파이프라인을 생성합니다.
    pub fn renew(
        self,
        device: &wgpu::Device,
        accum_render_target: &AccumRenderTarget,
        reveal_render_target: &RevealRenderTarget,
    ) -> Self {
        let vertex = self.vertex;
        let sampler = self.sampler;
        let bind_group =
            Self::create_bind_group(device, accum_render_target, reveal_render_target, &sampler);
        let pipeline = self.pipeline;
        Self {
            vertex,
            sampler,
            bind_group,
            pipeline,
        }
    }

    /// 파이프라인을 실행합니다.
    pub fn process(&self, rpass: &mut wgpu::RenderPass) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex.slice(..));
        rpass.draw(0..4, 0..1);
    }
}
