use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};
use mod_render::CameraResource;

/// 배경을 그리는 렌더링 파이프라인의 이름입니다.
pub const BACKGROUND_PIPELINE_NAME: &'static str = "Background";
/// `Login_Pad_BG` 텍스처 이름입니다.
pub const LOGIN_PAD_BG: &'static str = "Login_Pad_BG";
/// 임베딩된 `Login_Pad_BG.dds`파일 데이터입니다.
pub const LOGIN_PAD_BG_DATA: &'static [u8; 2227108] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/Login_Pad_BG.dds"
));

/// ## Background Data Layout
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BackgroundDataLayout {
    pub ratio: f32,
    pub _padding0: [u8; 12],
}

impl Default for BackgroundDataLayout {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            _padding0: [0; 12],
        }
    }
}

/// ## Background Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundUniform(Arc<wgpu::Buffer>);

impl BackgroundUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<BackgroundDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

#[allow(dead_code)]
impl BackgroundUniform {
    /// 초기화되지 않은 새로운 `BackgroundUniform`를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// `BackgroundUniform`의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, _device: &wgpu::Device, _queue: &wgpu::Queue, data: BackgroundDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut BackgroundDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });
    }

    /// `BackgroundUniform`의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        data: Vec<u8>,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    view.copy_from_slice(&data);

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });
    }

    /// 범위에 해당하는 슬라이스된 유니폼 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }

    /// 유니폼 버퍼의 [`wgpu::BindingResource`]를 반환합니다.
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.0.as_entire_binding()
    }

    /// 유니폼 버퍼의 [`wgpu::BufferBinding`]을 반환합니다.
    pub fn as_entire_buffer_binding(&self) -> wgpu::BufferBinding<'_> {
        self.0.as_entire_buffer_binding()
    }
}

static_assertions::const_assert_ne!(BackgroundUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    BackgroundUniform::SIZE as usize,
    core::mem::size_of::<BackgroundDataLayout>()
);

/// ## Background Shader Resource
#[derive(Debug)]
pub struct BackgroundResource {
    pub pipeline: Arc<wgpu::RenderPipeline>,
    pub uniform_buffer: BackgroundUniform,
    pub bind_group: wgpu::BindGroup,
}

impl BackgroundResource {
    /// `BackgroundResource`의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&format!(
                    "BindGroupLayout({})",
                    stringify!(BackgroundResource)
                )),
                entries: &[
                    // 0번 바인딩: 배경 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(BackgroundUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 1번 바인딩: 배경 텍스처
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
                    // 2번 바인딩: 배경 텍스처 샘플러
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

    /// 초기화되지 않은 새로운 `BackgroundResource`를 생성합니다.
    pub fn uninit(
        label: Option<&str>,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> Self {
        let pipeline =
            create_background_render_pipeline(device, depth_stencil_format, render_target_format);
        let tag = &format!("Uniform({})", stringify!(BackgroundUniform));
        let uniform_buffer = BackgroundUniform::uninit(Some(tag), device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    /// 배경을 그립니다.
    pub fn draw<'a>(&'a self, camera: &CameraResource, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &camera.bind_group, &[]);
        rpass.set_bind_group(1, &self.bind_group, &[]);
        rpass.draw(0..4, 0..1);
    }
}

/// 쉐이더 모듈을 생성합니다.
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "assets/shaders/background.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }
    }
}

/// 파이프라인 레이아웃을 생성합니다.
fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("PipelineLayout({})", BACKGROUND_PIPELINE_NAME)),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            BackgroundResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 배경을 그리는 렌더링 파이프라인을 생성합니다.
pub fn create_background_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("RenderPipeline({})", BACKGROUND_PIPELINE_NAME)),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            entry_point: Some("vs_main"),
            module: &module,
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::LessEqual,
            depth_write_enabled: true,
            format: depth_stencil_format,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            entry_point: Some("fs_main"),
            module: &module,
            targets: &[Some(wgpu::ColorTargetState {
                blend: None,
                format: render_target_format,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}
