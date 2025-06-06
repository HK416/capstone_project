use std::sync::{
    atomic::{AtomicUsize, Ordering as MemOrdering},
    Arc, OnceLock,
};

use wgpu::util::DeviceExt;

/// 최대 렌더 타겟 뷰의 개수 입니다.
pub const MAX_RENDER_TARGET_VIEWS: usize = 2;

/// `TriangleStrip`으로 구성된 사각형 메쉬의 [wgpu::Buffer]를 가져옵니다.
pub fn get_quad_vertex_buffer(device: &wgpu::Device) -> &'static wgpu::Buffer {
    /// 정점 버퍼의 데이터입니다.
    const VERTICES: [[f32; 4]; 4] = [
        [-1.0, -1.0, 0.0, 1.0],
        [-1.0, 1.0, 0.0, 0.0],
        [1.0, -1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0, 0.0],
    ];
    /// `TriangleStrip`으로 구성된 사각형 정점 버퍼입니다.
    static INSTANCE: OnceLock<wgpu::Buffer> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex(Quad)"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        })
    })
}

/// 선형 보간을 수행하는 [wgpu::Sampler]를 가져옵니다.
pub fn get_bilinear_sampler(device: &wgpu::Device) -> &'static wgpu::Sampler {
    static INSTANCE: OnceLock<wgpu::Sampler> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler(Bilinear)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    })
}

/// 프레임 버퍼 쉐이더 리소스입니다.
#[derive(Debug)]
pub struct FrameResource {
    /// 렌더링 파이프라인입니다.
    pipeline: wgpu::RenderPipeline,
    /// 깊이 버퍼 뷰 입니다.
    depth_buffer_view: Arc<wgpu::TextureView>,
    /// 렌더 타겟 뷰 입니다.
    render_target_views: [Arc<wgpu::TextureView>; MAX_RENDER_TARGET_VIEWS],
    /// 바인드 그룹입니다.
    bind_group: [wgpu::BindGroup; MAX_RENDER_TARGET_VIEWS],
    /// 현재 렌더 타겟 뷰의 인덱스입니다.
    index: AtomicUsize,
}

impl FrameResource {
    /// [wgpu::ShaderModule]을 생성합니다.
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        let desc = wgpu::include_wgsl!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "assets/shaders/frame.wgsl"
        ));

        unsafe {
            if cfg!(feature = "enable-shader-validation") {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
            } else {
                device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
            }
        }
    }

    /// [wgpu::BindGroupLayout]을 반환합니다.
    fn get_bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Frame)"),
                entries: &[
                    // 0번 바인딩: 렌더 타겟 텍스처 뷰
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
                    // 1번 바인딩: 텍스처 샘플러
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

    /// [wgpu::PipelineLayout]을 생성합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Frame)"),
            bind_group_layouts: &[Self::get_bind_group_layout(device)],
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
            label: Some("RenderPipeline(Frame)"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: (core::mem::size_of::<f32>() * 0) as wgpu::BufferAddress,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                            offset: (core::mem::size_of::<f32>() * 2) as wgpu::BufferAddress,
                        },
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                front_face: wgpu::FrontFace::Cw,
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: None,
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
        })
    }

    /// 새로운 프레임 쉐이더 리소스를 생성합니다.
    pub fn new(
        device: &wgpu::Device,
        content_width: u32,
        content_height: u32,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> Self {
        let pipeline = Self::create_render_pipeline(device, render_target_format);
        let size = wgpu::Extent3d {
            width: content_width,
            height: content_height,
            depth_or_array_layers: 1,
        };
        let depth_buffer_view = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Texture(Depth)"),
                    dimension: wgpu::TextureDimension::D2,
                    size,
                    format: depth_stencil_format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let render_target_view_0 = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Texture(RenderTarget0)"),
                    dimension: wgpu::TextureDimension::D2,
                    size,
                    format: render_target_format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let render_target_view_1 = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Texture(RenderTarget0)"),
                    dimension: wgpu::TextureDimension::D2,
                    size,
                    format: render_target_format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Non-Filter-RT0)"),
            layout: Self::get_bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_target_view_0),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(get_bilinear_sampler(device)),
                },
            ],
        });
        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Non-Filter-RT1)"),
            layout: Self::get_bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_target_view_1),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(get_bilinear_sampler(device)),
                },
            ],
        });

        Self {
            pipeline,
            depth_buffer_view,
            render_target_views: [render_target_view_0, render_target_view_1],
            bind_group: [bind_group_0, bind_group_1],
            index: AtomicUsize::new(0),
        }
    }

    /// 프레임 쉐이더 리소스를 재생성합니다.
    pub fn renew(
        self,
        device: &wgpu::Device,
        content_width: u32,
        content_height: u32,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> Self {
        let pipeline = self.pipeline;
        let size = wgpu::Extent3d {
            width: content_width,
            height: content_height,
            depth_or_array_layers: 1,
        };
        let depth_buffer_view = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Texture(Depth)"),
                    dimension: wgpu::TextureDimension::D2,
                    size,
                    format: depth_stencil_format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let render_target_view_0 = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Texture(RenderTarget0)"),
                    dimension: wgpu::TextureDimension::D2,
                    size,
                    format: render_target_format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let render_target_view_1 = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Texture(RenderTarget0)"),
                    dimension: wgpu::TextureDimension::D2,
                    size,
                    format: render_target_format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Non-Filter-RT0)"),
            layout: Self::get_bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_target_view_0),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(get_bilinear_sampler(device)),
                },
            ],
        });
        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Non-Filter-RT1)"),
            layout: Self::get_bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_target_view_1),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(get_bilinear_sampler(device)),
                },
            ],
        });

        Self {
            pipeline,
            depth_buffer_view,
            render_target_views: [render_target_view_0, render_target_view_1],
            bind_group: [bind_group_0, bind_group_1],
            index: AtomicUsize::new(0),
        }
    }

    /// 현재 렌더 타겟 뷰를 반환합니다.
    pub fn get_render_target_view(&self) -> &Arc<wgpu::TextureView> {
        let index = self.index.load(MemOrdering::Acquire);
        &self.render_target_views[index]
    }

    /// 현재 깊이 버퍼 뷰를 반환합니다.
    pub fn get_depth_buffer_view(&self) -> &Arc<wgpu::TextureView> {
        &self.depth_buffer_view
    }

    /// 렌더링 파이프라인을 실행합니다.
    pub fn process<'a>(&self, device: &wgpu::Device, rpass: &mut wgpu::RenderPass<'a>) {
        let index = self.index.load(MemOrdering::Acquire);
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group[index], &[]);
        rpass.set_vertex_buffer(0, get_quad_vertex_buffer(device).slice(..));
        rpass.draw(0..4, 0..1);
        self.index
            .store((index + 1) % MAX_RENDER_TARGET_VIEWS, MemOrdering::Release);
    }
}
