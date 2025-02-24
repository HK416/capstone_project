//! # 렌더링
//! 렌더링은 다음 순서로 진행됩니다.
//! 1. Opaque Pass
//! 불투명한 객체가 스왑체인에 연결된 렌터 타겟 텍스처에 그려집니다.
//!
//! 2. Transparent Pass
//! 투명한 객체가 누적값(Accumulate) 렌더 타겟 텍스처와 노출값(Revealage) 렌더 타겟 텍스처에 그려집니다.
//!
//! 3. Composite Pass
//! 아래 렌더 타겟 텍스처를 이용하여 스왑체인에 연결된 렌더 타겟 텍스처에 그립니다.
//! - 누적값(Accumulate)
//! - 노출값(Revealage)
//!

use std::sync::{Arc, OnceLock};

use winit::window::Window;

/// Composite Pass에 사용되는 쉐이더 리소스입니다.
///
/// 렌더링 해상도가 변경될 경우 해당 쉐이더 리소스를 다시 생성해야합니다.
///
#[derive(Debug)]
pub struct CompositeResource {
    pub accum_render_target: Arc<wgpu::TextureView>,
    pub reveal_render_target: Arc<wgpu::TextureView>,
    pub bind_group: wgpu::BindGroup,
}

impl CompositeResource {
    /// 누적 값 렌더 타겟 텍스처의 포맷입니다.
    pub const ACCUM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    /// 노출 값 렌더 타겟 텍스처의 포맷입니다.
    pub const REVEAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
}

impl CompositeResource {
    /// Composite Pass 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CompositePass)"),
                entries: &[
                    // 0번 바인딩: 누적 값 렌더 타겟 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: 노출 값 렌더 타겟 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        })
    }
}

impl CompositeResource {
    /// 초기화되지 않은 새로운 Composite Pass 쉐이더 리소스를 생성합니다.
    pub fn uninit(window: &Window, device: &wgpu::Device) -> Self {
        let (width, height): (u32, u32) = window.inner_size().into();

        let accum_render_target = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("RenderTarget(Accumulate)"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    format: Self::ACCUM_FORMAT,
                    dimension: wgpu::TextureDimension::D2,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let reveal_render_target = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("RenderTarget(Revealage)"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    format: Self::REVEAL_FORMAT,
                    dimension: wgpu::TextureDimension::D2,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(CompositePass)"),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&accum_render_target),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&reveal_render_target),
                },
            ],
        });

        Self {
            accum_render_target,
            reveal_render_target,
            bind_group,
        }
    }

    /// Composite Pass를 실행합니다.
    pub fn process<'a>(
        &self,
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        let pipeline =
            get_composite_pass_pipeline(device, render_target_format, depth_stencil_format);
        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0..4, 0..1);
    }
}

/// 쉐이더 모듈을 생성합니다.
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/composite.wgsl"
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
        label: Some("PipelineLayout(CompositePass)"),
        bind_group_layouts: &[CompositeResource::bind_group_layout(device)],
        push_constant_ranges: &[],
    })
}

/// Composite Pass 렌더링 파이프라인을 가져옵니다.
pub fn get_composite_pass_pipeline(
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
) -> &'static wgpu::RenderPipeline {
    static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
    PIPELINE.get_or_init(|| {
        let module = create_shader_module(device);
        let layout = create_pipeline_layout(device);
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RenderPipeline(CompositePass)"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Cw,
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_stencil_format,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
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
    })
}
