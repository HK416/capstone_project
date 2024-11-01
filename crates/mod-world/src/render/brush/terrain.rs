use std::{mem, sync::{Arc, OnceLock}};

use crate::render::{
    camera::CameraResource, 
    material::{universal::UniversalMaterialResource, MaterialResource}, 
    mesh::{Attribute, Mesh, StaticMeshResource}, 
    DEPTH_STENCIL_FORMAT, 
    SWAPCHAIN_FORMAT
};

use super::MeshBrush;



/// [wgpu::ShaderModule]을 생성합니다.
#[inline]
#[must_use]
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    #[cfg(feature = "enable-shader-validation")] {
        device.create_shader_module(
            wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/terrain.wgsl"))
        )
    }
    #[cfg(not(feature = "enable-shader-validation"))] {
        unsafe {
            device.create_shader_module_unchecked(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/terrain.wgsl"))
            )
        }
    }
}



/// [wgpu::PipelineLayout]을 생성합니다.
#[inline]
#[must_use]
fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(TerrainBrush)"), 
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device), 
                StaticMeshResource::bind_group_layout(device), 
                UniversalMaterialResource::bind_group_layout(device) 
            ], 
            push_constant_ranges: &[]
        }
    )
}



/// [wgpu::RenderPipeline]을 가져옵니다.
pub fn get_render_pipeline(device: &wgpu::Device) -> &'static wgpu::RenderPipeline {
    static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
    PIPELINE.get_or_init(|| {
        let module = create_shader_module(device);
        let layout = create_pipeline_layout(device);
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(TerrainBrush)"), 
                layout: Some(&layout), 
                vertex: wgpu::VertexState {
                    module: &module, 
                    entry_point: "vs_main", 
                    buffers: &[
                        // 0번 입력 속성: 위치
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3 
                                }
                            ]
                        }, 
                        // 1번 입력 속성: 노멀
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 1, 
                                    format: wgpu::VertexFormat::Float32x3 
                                }
                            ]
                        }, 
                        // 2번 입력 속성: 0번 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 2, 
                                    format: wgpu::VertexFormat::Float32x2 
                                }
                            ]
                        }, 
                        // 3번 입력 속성: 1번 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 3, 
                                    format: wgpu::VertexFormat::Float32x2 
                                }
                            ]
                        }
                    ], 
                    compilation_options: wgpu::PipelineCompilationOptions::default()
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
                fragment: Some(
                    wgpu::FragmentState {
                        module: &module, 
                        entry_point: "fs_main", 
                        targets: &[
                            Some(wgpu::ColorTargetState {
                                format: SWAPCHAIN_FORMAT, 
                                blend: None, 
                                write_mask: wgpu::ColorWrites::ALL
                            })
                        ], 
                        compilation_options: wgpu::PipelineCompilationOptions::default()
                    }
                ), 
                multiview: None, 
                cache: None 
            }
        )
    })
}





/// 지형을 그리는 브러쉬입니다.
#[derive(Debug)]
pub struct TerrainBrush {
    /// 그리기 대상 메쉬입니다.
    mesh: Arc<Mesh>, 

    /// 메쉬의 쉐이더 리소스입니다.
    mesh_resource: Arc<StaticMeshResource>, 

    /// 재질의 쉐이더 리소스입니다.
    materials: Vec<Arc<UniversalMaterialResource>>, 

    /// 그래픽스 파이프라인입니다.
    pipeline: &'static wgpu::RenderPipeline 
}

impl TerrainBrush {
    /// 지형을 그리는 브러쉬를 생성합니다.
    /// 
    /// # Panics
    /// 아래와 같은 경우 [`panic!`]을 호출합니다.
    /// - 주어진 메쉬가 브러쉬에서 필요한 정점 속성을 갖고 있지 않는 경우.
    /// - 주어진 재질이 없는 경우.
    /// - 주어진 메쉬의 하위메쉬 개수와 재질의 개수가 다른 경우.
    /// 
    pub fn new(
        device: &wgpu::Device, 
        mesh: Arc<Mesh>, 
        mesh_resource: Arc<StaticMeshResource>, 
        materials: Vec<Arc<UniversalMaterialResource>>
    ) -> Arc<Self> {
        assert!(mesh.attribute(&Attribute::Normal).is_some(), "The given mesh does not have vaertex attribute `Normal` used by the brush!");
        assert!(mesh.attribute(&Attribute::Texcoord0).is_some(), "The given mesh does not have vaertex attribute `Texcoord0` used by the brush!");
        assert!(mesh.attribute(&Attribute::Texcoord1).is_some(), "The given mesh does not have vaertex attribute `Texcoord1` used by the brush!");
        assert!(!materials.is_empty(), "The given material is empty!");
        assert!(mesh.submeshes().len() <= materials.len(), "The number of submeshes and the number of materials are different!");
        unsafe { Self::new_unchecked(device, mesh, mesh_resource, materials) }
    }


    /// 지형을 그리는 브러쉬를 생성합니다.
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked(
        device: &wgpu::Device, 
        mesh: Arc<Mesh>, 
        mesh_resource: Arc<StaticMeshResource>, 
        materials: Vec<Arc<UniversalMaterialResource>>
    ) -> Arc<Self> {
        Arc::new(Self {
            mesh, 
            mesh_resource, 
            materials, 
            pipeline: get_render_pipeline(device)
        })
    }
}

impl MeshBrush for TerrainBrush {
    #[inline]
    #[must_use]
    fn mesh(&self) -> &Arc<Mesh> {
        &self.mesh
    }

    #[inline]
    #[must_use]
    fn pipeline(&self) -> &'static wgpu::RenderPipeline {
        self.pipeline
    }

    fn bind<'a>(&'a self, camera: &CameraResource, rpass: &mut wgpu::RenderPass<'a>) {
        // 렌더 패스에 그래픽스 파이프라인을 바인드합니다.
        rpass.set_pipeline(&self.pipeline);

        // 렌더 패스에 쉐이더 리소스를 바인드합니다.
        rpass.set_bind_group(0, camera.bind_group(), &[]);
        rpass.set_bind_group(1, self.mesh_resource.bind_group(), &[]);

        // 렌더 패스에 메쉬를 바인드합니다.
        rpass.set_vertex_buffer(0, self.mesh.vertex().slice(..));
        rpass.set_vertex_buffer(1, self.mesh.attribute(&Attribute::Normal).unwrap().slice(..));
        rpass.set_vertex_buffer(2, self.mesh.attribute(&Attribute::Texcoord0).unwrap().slice(..));
        rpass.set_vertex_buffer(3, self.mesh.attribute(&Attribute::Texcoord1).unwrap().slice(..));
    }

    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        if !self.mesh.submeshes().is_empty() {
            for (index, submesh) in self.mesh.submeshes().iter().enumerate() {
                // 렌더 패스에 재질 쉐이더 리소스를 바인드합니다.
                let material = self.materials.get(index).unwrap();
                rpass.set_bind_group(2, material.bind_group(), &[]);

                // 렌더 패스에 인덱스 버퍼를 바인드합니다.
                rpass.set_index_buffer(submesh.slice(..), submesh.format());

                // 인덱스 버퍼를 사용하여 지형을 그립니다.
                rpass.draw_indexed(0..submesh.count(), 0, 0..1);
            }
        } else {
            // 랜더 패스에 재질 쉐이더 리소스를 바인드합니다.
            let material = self.materials.get(0).unwrap();
            rpass.set_bind_group(2, material.bind_group(), &[]);
            
            // 정점 버퍼를 사용하여 지형을 그립니다.
            rpass.draw(0..self.mesh.num_vertices(), 0..1);
        }
    }
}
