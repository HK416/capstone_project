use std::fmt;
use std::mem;
use std::sync::Arc;
use winit::window::Window;

use crate::render::bind_group::EntityBindGroup;
use crate::render::bind_group::GlobalBindGroup;
use crate::render::material::GraphicsPipeline;
use crate::render::mesh::RenderableMesh;
use crate::render::mesh::ModelMesh3D;
use crate::render::scale::RenderScale;
use crate::render::targets::SWAPCHAIN_FORMAT;



/// 텍스처를 입힌 오브젝트를 표시합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureMaterialID;



/// 텍스처를 입힌 오브젝트를 표시합니다.
pub struct TextureMaterial {
    depth_buffer: wgpu::TextureView, 
    pipeline: wgpu::RenderPipeline, 
}

impl TextureMaterial {
    /// 깊이 버퍼 텍스처 포맷입니다.
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
}

impl TextureMaterial {
    /// 깊이 버퍼를 생성합니다.
    #[must_use]
    fn create_depth_buffer(
        window: &Window, 
        device: &wgpu::Device
    ) -> wgpu::TextureView {
        // 현재 애플리케이션 창의 크기를 가져옵니다.
        let (width, height): (u32, u32) = window.inner_size().into();

        // 깊이 버퍼를 생성합니다.
        device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some(&format!("DepthBuffer({})", stringify!(Self))), 
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 }, 
                dimension: wgpu::TextureDimension::D2, 
                format: Self::DEPTH_FORMAT, 
                mip_level_count: 1, 
                sample_count: 1, 
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
                view_formats: &[]
            }
        ).create_view(
            &wgpu::TextureViewDescriptor { ..Default::default() }
        )
    }

    /// 쉐이더 모듈을 생성합니다.
    #[inline]
    #[must_use]
    fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
        // 쉐이더 모듈 설명자를 생성합니다.
        let desc = wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/texture.wgsl"));

        // 쉐이더 모듈을 생성합니다.
        #[cfg(feature = "enable-shader-validation")] {
            device.create_shader_module(desc)
        }
        #[cfg(not(feature = "enable-shader-validation"))] {
            unsafe { device.create_shader_module_unchecked(desc) }
        }
    }

    /// 파이프라인 레이아웃을 생성합니다.
    #[inline]
    #[must_use]
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("PipelineLayout({})", stringify!(Self))), 
                bind_group_layouts: &[
                    GlobalBindGroup::layout(device), 
                    EntityBindGroup::layout(device), 
                ], 
                push_constant_ranges: &[]
            }
        )
    }

    /// 렌더 파이프라인을 생성합니다.
    #[inline]
    fn create_render_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
        // 쉐이더 모듈을 생성합니다.
        let module = Self::create_shader_module(device);

        // 파이프라인 레이아웃을 생성합니다.
        let pipeline_layout = Self::create_pipeline_layout(device);

        // 렌더 파이프라인을 생성합니다.
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some(&format!("RenderPipeline({})", stringify!(Self))), 
                layout: Some(&pipeline_layout), 
                vertex: wgpu::VertexState {
                    module: &module, 
                    entry_point: "vs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: (mem::size_of::<f32>() * 3) as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    shader_location: 0, 
                                    offset: 0, 
                                    format: wgpu::VertexFormat::Float32x3, 
                                }, 
                            ], 
                        }, 
                        wgpu::VertexBufferLayout {
                            array_stride: (mem::size_of::<f32>() * 2) as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    shader_location: 1, 
                                    offset: 0, 
                                    format: wgpu::VertexFormat::Float32x2, 
                                }, 
                            ], 
                        }, 
                    ], 
                }, 
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back), 
                    front_face: wgpu::FrontFace::Ccw, 
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Self::DEPTH_FORMAT, 
                    depth_write_enabled: true, 
                    depth_compare: wgpu::CompareFunction::Less, 
                    stencil: wgpu::StencilState::default(), 
                    bias: wgpu::DepthBiasState::default(), 
                }), 
                multisample: wgpu::MultisampleState::default(), 
                fragment: Some(wgpu::FragmentState {
                    module: &module, 
                    entry_point: "fs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            blend: None, 
                            format: SWAPCHAIN_FORMAT, 
                            write_mask: wgpu::ColorWrites::all(), 
                        }), 
                    ], 
                }), 
                multiview: None, 
                cache: None, 
            }
        )
    }
}

impl TextureMaterial {
    pub fn new(
        window: &Window, 
        device: &wgpu::Device
    ) -> Self {
        // 깊이 버퍼를 생성합니다.
        let depth_buffer = Self::create_depth_buffer(window, device);

        // 렌더 파이프라인을 생성합니다.
        let pipeline = Self::create_render_pipeline(device);

        Self { depth_buffer, pipeline }
    }
}

impl GraphicsPipeline for TextureMaterial {
    #[inline]
    fn attributes(&self) -> &'static [u32] {
        &[ModelMesh3D::ATTRIBUTE_POSITION, ModelMesh3D::ATTRIBUTE_TEXCOORD0]
    }

    #[inline]
    fn resize_buffer(
        &mut self, 
        _: RenderScale, 
        window: &Window, 
        device: &wgpu::Device, 
    ) {
        self.depth_buffer = Self::create_depth_buffer(window, device);
    }

    fn process(
        &self, 
        world: &hecs::World, 
        camera: hecs::Entity, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        render_target: &wgpu::TextureView
    ) {
        let mut camera_query = match world.query_one::<&Arc<GlobalBindGroup>>(camera) {
            Ok(query) => query, 
            Err(err) => return log::warn!("{}", err.to_string()),
        };

        let camera_bind_group = match camera_query.get() {
            Some(bind_group) => bind_group, 
            None => return log::warn!("Could not find `GlobalBindGroup` on given entity!"), 
        };

        let mut entities = world.query::<(&Arc<ModelMesh3D>, &Arc<EntityBindGroup>)>()
            .with::<&TextureMaterialID>();

        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { ..Default::default() }
        );

        {
            let mut rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some(&format!("RenderPass({})", stringify!(Self))), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: render_target, 
                            resolve_target: None, 
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, 
                                store: wgpu::StoreOp::Store, 
                            }, 
                        }), 
                    ], 
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_buffer, 
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), 
                            store: wgpu::StoreOp::Store, 
                        }), 
                        stencil_ops: None, 
                    }), 
                    timestamp_writes: None, 
                    occlusion_query_set: None, 
                }
            );

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &camera_bind_group, &[]);
            for (_id, (mesh, bind_group)) in entities.iter() {
                rpass.set_bind_group(1, &bind_group, &[]);
                mesh.bind(self.attributes(), &mut rpass);
                mesh.draw(0..1, &mut rpass);
            }
        }

        queue.submit([encoder.finish()]);
    }
}

impl fmt::Debug for TextureMaterial {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(Self))
    }
}
