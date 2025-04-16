use std::sync::{Arc, OnceLock};

use super::WeightedBlendedOITResource;

/// [wgpu::ShaderModule]을 반환합니다.
fn create_shader_module(device: &wgpu::Device) -> &'static wgpu::ShaderModule {
    static MODULE: OnceLock<wgpu::ShaderModule> = OnceLock::new();
    MODULE.get_or_init(|| unsafe {
        let desc = wgpu::include_wgsl!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/shaders/weighted_blended_oit.wgsl"
        ));

        if cfg!(feature = "enable-shader-validation") {
            device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
        } else {
            device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
        }
    })
}

/// Weighted Blended Order-Independent Transparency를 수행하는 렌더링 파이프라인입니다.
pub struct WeightedBlendedOITRenderPipeline;

/// Weighted Blended Order-Independent Transparency를 수행하는 그래픽스 파이프라인 인스턴스입니다.
static PIPELINE: OnceLock<Arc<wgpu::RenderPipeline>> = OnceLock::new();

impl WeightedBlendedOITRenderPipeline {
    /// [wgpu::PipelineLayout]을 반환합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(WeightedBlendedOIT)"),
            bind_group_layouts: &[WeightedBlendedOITResource::bind_group_layout(device)],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.  
    /// 렌더링 파이프라인이 초기화 되지 않은 경우 `None`을 반환합니다.
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
                let module = create_shader_module(device);
                let layout = Self::create_pipeline_layout(device);
                Arc::new(
                    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("RenderPipeline(WeightedBlendedOIT)"),
                        layout: Some(&layout),
                        vertex: wgpu::VertexState {
                            module,
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
                    }),
                )
            })
            .clone()
    }
}
