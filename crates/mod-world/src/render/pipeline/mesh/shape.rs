use std::{mem, sync::{Arc, OnceLock}};

use crate::{
    component::WorldID, 
    render::{
        camera::CameraResource, material::Material, mesh::{Attribute, Mesh, NonSkinnedMesh}, DEPTH_STENCIL_FORMAT, SWAPCHAIN_FORMAT
    }
};

use super::MeshRenderer;



/// 모델을 그리는 렌더러입니다.
#[derive(Debug)]
pub struct ShapeRenderer {
    /// 렌더러의 게임 오브젝트 식별자입니다.
    game_object_id: WorldID, 

    /// 렌더러에 연결된 메쉬입니다.
    mesh: Mesh, 

    /// 렌더러에 연결된 재질입니다.
    materials: Vec<Arc<Material>>, 

    /// 렌더러의 그래픽스 파이프라인입니다.
    pipeline: &'static wgpu::RenderPipeline,  
}

impl ShapeRenderer {
    /// 새로운 모델 메쉬 렌더러를 생성합니다.
    #[must_use]
    pub fn new(
        id: WorldID, 
        mesh: Mesh, 
        materials: Vec<Arc<Material>>, 
        device: &wgpu::Device
    ) -> Self {
        Self { 
            game_object_id: id, 
            mesh, 
            materials, 
            pipeline: get_render_pipeline(device) 
        }
    }
}

impl MeshRenderer for ShapeRenderer {
    #[inline]
    #[must_use]
    fn game_object(&self) -> &WorldID {
        &self.game_object_id
    }

    #[inline]
    #[must_use]
    fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    #[inline]
    #[must_use]
    fn materials(&self) -> &[Arc<Material>] {
        &self.materials
    }

    fn bind<'a>(&'a self, camera: &CameraResource, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_pipeline(&self.pipeline);

        rpass.set_bind_group(0, camera.bind_group(), &[]);
        rpass.set_bind_group(1, self.mesh().bind_group(), &[]);

        rpass.set_vertex_buffer(0, self.mesh().vertex().slice(..));
        rpass.set_vertex_buffer(1, self.mesh().attribute(&Attribute::Texcoords0).unwrap().slice(..));
    }

    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        let materials = self.materials();
        for (index, submesh) in self.mesh().submeshes().iter().enumerate() {
            let material = materials.get(index).unwrap();
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.set_index_buffer(submesh.slice(..), wgpu::IndexFormat::Uint32);

            rpass.draw_indexed(0..submesh.count(), 0, 0..1);
        }
    }
}



/// 렌더러의 그래픽스 파이프라인을 가져옵니다.
#[must_use]
fn get_render_pipeline(device: &wgpu::Device) -> &'static wgpu::RenderPipeline {
    static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
    PIPELINE.get_or_init(|| {
        let module = shader_module(device);
        let layout = pipeline_layout(device);
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(ShapeRenderer)"), 
                layout: Some(&layout), 
                vertex: wgpu::VertexState {
                    module: &module, 
                    entry_point: "vs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    buffers: &[
                        // 0번 입력 속성: 위치 데이터
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float3>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3, 
                                }
                            ]
                        }, 
                        // 1번 입력 속성: 0번 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float2>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 1, 
                                    format: wgpu::VertexFormat::Float32x2, 
                                }
                            ]
                        }, 
                    ],
                }, 
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    front_face: wgpu::FrontFace::Cw, 
                    cull_mode: Some(wgpu::Face::Back), 
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState { 
                    format: DEPTH_STENCIL_FORMAT, 
                    depth_write_enabled: true, 
                    depth_compare: wgpu::CompareFunction::Less, 
                    stencil: wgpu::StencilState::default(), 
                    bias: wgpu::DepthBiasState::default() 
                }), 
                multisample: wgpu::MultisampleState::default(), 
                fragment: Some(wgpu::FragmentState {
                    module: &module, 
                    entry_point: "fs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: SWAPCHAIN_FORMAT, 
                            blend: None, 
                            write_mask: wgpu::ColorWrites::all()
                        }), 
                    ], 
                }), 
                multiview: None, 
                cache: None, 
            }
        )
    })
}

/// 렌더러의 쉐이더 모듈을 가져옵니다.
#[inline]
#[must_use]
fn shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    #[cfg(feature = "enable-shader-validation")] {
        device.create_shader_module(
            wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/shape.wgsl"))
        )
    }
    #[cfg(not(feature = "enable-shader-validation"))] {
        unsafe {
            device.create_shader_module_unchecked(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/shape.wgsl"))
            )
        }
    }
}

/// 렌더러의 [wgpu::PipelineLayout]을 가져옵니다.
#[inline]
#[must_use]
fn pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(ModelRenderer)"), 
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device), 
                NonSkinnedMesh::bind_group_layout(device), 
                Material::bind_group_layout(device)
            ], 
            push_constant_ranges: &[]
        }
    )
}
