use std::sync::OnceLock;

use wgpu::util::DeviceExt;

use super::{BlurTextureResource, BrightRenderTarget};

/// 컴퓨트 파이프라인에서 사용되는 작업 그룹의 크기입니다.
const WORKGROUP_SIZE: u32 = 16;

/// 가우시안 블러를 수행하는 컴퓨트 파이프라인입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct GaussianBlurPipeline {
    ping_bind_group: wgpu::BindGroup,
    pong_bind_group: wgpu::BindGroup,
    ping_pipeline: wgpu::ComputePipeline,
    pong_pipeline: wgpu::ComputePipeline,
    width: u32,
    height: u32,
}

impl GaussianBlurPipeline {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(GaussianBlur)"),
                entries: &[
                    // 0번 바인딩: 발광체 오브젝트 색상 렌더 타겟
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: 출력물 저장 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba16Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
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
            "/assets/shaders/blur.wgsl"
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
            label: Some("PipelineLayout(GaussianBlur)"),
            bind_group_layouts: &[Self::bind_group_layout(device)],
            push_constant_ranges: &[],
        })
    }

    /// [wgpu::ComputePipeline]을 생성합니다.
    fn create_horizontal_compute_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
        let module = Self::create_shader_module(device);
        let layout = Self::create_pipeline_layout(device);
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ComputePipeline(GaussianBlur)"),
            layout: Some(&layout),
            module: &module,
            entry_point: Some("cs_horizontal_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }

    /// [wgpu::ComputePipeline]을 생성합니다.
    fn create_vertical_compute_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
        let module = Self::create_shader_module(device);
        let layout = Self::create_pipeline_layout(device);
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ComputePipeline(GaussianBlur)"),
            layout: Some(&layout),
            module: &module,
            entry_point: Some("cs_vertical_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }

    /// [wgpu::BindGroup]을 생성합니다.
    fn create_bind_group(
        device: &wgpu::Device,
        input_texture: &wgpu::TextureView,
        output_texture: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(GaussianBlur)"),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(output_texture),
                },
            ],
        })
    }

    /// 새로운 가우시안 블러 파이프라인을 생성합니다.
    pub fn new(
        width: u32,
        height: u32,
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
    ) -> (Self, BrightRenderTarget, BloomPipeline) {
        let bright_render_target = BrightRenderTarget::new(width, height, device);
        let ping_result_texture = BlurTextureResource::new(width / 2, height / 2, device);
        let blur_texture_resource = BlurTextureResource::new(width / 4, height / 4, device);
        let ping_bind_group = Self::create_bind_group(
            device,
            bright_render_target.view(),
            ping_result_texture.view(),
        );
        let pong_bind_group = Self::create_bind_group(
            device,
            ping_result_texture.view(),
            blur_texture_resource.view(),
        );
        let ping_pipeline = Self::create_horizontal_compute_pipeline(device);
        let pong_pipeline = Self::create_vertical_compute_pipeline(device);
        let bloom_pipeline =
            BloomPipeline::new(device, &blur_texture_resource, render_target_format);

        (
            Self {
                ping_bind_group,
                pong_bind_group,
                ping_pipeline,
                pong_pipeline,
                width,
                height,
            },
            bright_render_target,
            bloom_pipeline,
        )
    }

    /// 기존 파이프라인으로부터 새로운 파이프라인을 생성합니다.
    pub fn renew(
        self,
        width: u32,
        height: u32,
        device: &wgpu::Device,
        bloom_pipeline: BloomPipeline,
    ) -> (Self, BrightRenderTarget, BloomPipeline) {
        let bright_render_target = BrightRenderTarget::new(width, height, device);
        let ping_result_texture = BlurTextureResource::new(width / 2, height / 2, device);
        let blur_texture_resource = BlurTextureResource::new(width / 4, height / 4, device);
        let ping_bind_group = Self::create_bind_group(
            device,
            bright_render_target.view(),
            ping_result_texture.view(),
        );
        let pong_bind_group = Self::create_bind_group(
            device,
            ping_result_texture.view(),
            blur_texture_resource.view(),
        );
        let ping_pipeline = self.ping_pipeline;
        let pong_pipeline = self.pong_pipeline;
        let bloom_pipeline = bloom_pipeline.renew(device, &blur_texture_resource);

        (
            Self {
                ping_bind_group,
                pong_bind_group,
                ping_pipeline,
                pong_pipeline,
                width,
                height,
            },
            bright_render_target,
            bloom_pipeline,
        )
    }

    /// 파이프라인을 실행합니다.
    pub fn process(&self, cpass: &mut wgpu::ComputePass) {
        cpass.set_pipeline(&self.ping_pipeline);
        cpass.set_bind_group(0, &self.ping_bind_group, &[]);
        let dispatch_x = (self.width / 2 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let dispatch_y = (self.height / 2 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        cpass.set_pipeline(&self.pong_pipeline);
        cpass.set_bind_group(0, &self.pong_bind_group, &[]);
        let dispatch_x = (self.width / 4 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let dispatch_y = (self.height / 4 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}

/// Bloom 효과를 구현하는
#[derive(Debug, PartialEq, Eq)]
pub struct BloomPipeline {
    vertex: wgpu::Buffer,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl BloomPipeline {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Bloom)"),
                entries: &[
                    // 0번 바인딩: 블러 텍스처
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
                    // 1번 바인딩: 블러 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
            "/assets/shaders/bloom.wgsl"
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
            label: Some("PipelineLayout(Bloom)"),
            bind_group_layouts: &[Self::bind_group_layout(device)],
            push_constant_ranges: &[],
        })
    }

    /// [wgpu::RenderPipeline]을 생성합니다.
    fn create_render_pipeline(
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let module = Self::create_shader_module(device);
        let layout = Self::create_pipeline_layout(device);
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RenderPipeline(Bloom)"),
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    format: render_target_format,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    /// Bloom 결과를 블렌딩할 때 사용되는 사각형 정점 버퍼를 생성합니다.
    fn create_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        const VERTICES: [[f32; 4]; 4] = [
            [-1.0, -1.0, 0.0, 1.0],
            [-1.0, 1.0, 0.0, 0.0],
            [1.0, -1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 0.0],
        ];

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex(Bloom)"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    /// 렌더 타겟 텍스처 샘플러를 생성합니다.
    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler(Bloom)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    }

    /// [wgpu::BindGroup]을 생성합니다.
    fn create_bind_group(
        device: &wgpu::Device,
        sampler: &wgpu::Sampler,
        blur_texture_resource: &BlurTextureResource,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Bloom)"),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(blur_texture_resource.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// 새로운 [BloomPipeline]을 생성합니다.
    pub fn new(
        device: &wgpu::Device,
        blur_texture_resource: &BlurTextureResource,
        render_target_format: wgpu::TextureFormat,
    ) -> Self {
        let vertex = Self::create_vertex_buffer(device);
        let sampler = Self::create_sampler(device);
        let bind_group = Self::create_bind_group(device, &sampler, blur_texture_resource);
        let pipeline = Self::create_render_pipeline(device, render_target_format);
        Self {
            vertex,
            sampler,
            bind_group,
            pipeline,
        }
    }

    /// 기존 파이프라인으로부터 새로운 파이프라인을 생성합니다.
    pub fn renew(self, device: &wgpu::Device, blur_texture_resource: &BlurTextureResource) -> Self {
        let vertex = self.vertex;
        let sampler = self.sampler;
        let bind_group = Self::create_bind_group(device, &sampler, blur_texture_resource);
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
