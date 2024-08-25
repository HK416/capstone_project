use std::mem;
use std::sync::OnceLock;
use hecs::World;

use crate::camera::CameraObject;
use crate::material::Material;
use crate::material::MaterialComponent;
use crate::mesh::Attribute;
use crate::mesh::MeshComponent;
use crate::object::GameObject;
use crate::object::GameObjectComponent;
use crate::skin::Skin;
use crate::skin::SkinComponent;
use crate::DEPTH_STENCIL_FORMAT;
use crate::SWAPCHAIN_FORMAT;



#[derive(Debug, Clone, Copy)]
pub struct TextureBrush;

impl TextureBrush {
    /// 오브젝트를 그립니다.
    pub fn draw<'a>(
        world: &'a World, 
        device: &wgpu::Device, 
        rpass: &mut wgpu::RenderPass<'a>
    ) {
        Self::draw_mesh(world, device, rpass);
        Self::draw_skinned_mesh(world, device, rpass);
    }

    /// 메쉬를 그립니다.
    fn draw_mesh<'a>(
        world: &'a World, 
        device: &wgpu::Device, 
        rpass: &mut wgpu::RenderPass<'a>
    ) {
        rpass.set_pipeline(MeshTextureShader::get(device));

        type QueryMesh<'a> = (&'a MeshComponent, &'a GameObjectComponent, &'a Vec<MaterialComponent>);
        let mut query = world.query::<QueryMesh>().with::<&TextureBrush>().without::<&SkinComponent>();
        for (_, (mesh, object, materials)) in query.iter() {
            rpass.set_bind_group(1, object.bind_group(), &[]);
            rpass.set_vertex_buffer(0, mesh.vertices().slice(..));
            rpass.set_vertex_buffer(1, mesh.attribute(Attribute::Texcoords0).unwrap().slice(..));
            for (idx, submesh) in mesh.submeshes().iter().enumerate() {
                rpass.set_bind_group(2, materials[idx].bind_group(), &[]);
                rpass.set_index_buffer(submesh.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..submesh.count(), 0, 0..1);
            }
        }
    }

    /// 스키닝된 메쉬를 그립니다.
    fn draw_skinned_mesh<'a>(
        world: &'a World, 
        device: &wgpu::Device, 
        rpass: &mut wgpu::RenderPass<'a>
    ) {
        rpass.set_pipeline(&SkinMeshTextureShader::get(device));

        type QuerySkinMesh<'a> = (&'a MeshComponent, &'a SkinComponent, &'a Vec<MaterialComponent>);
        let mut query = world.query::<QuerySkinMesh>().with::<&TextureBrush>();
        for (_, (mesh, skin, materials)) in query.iter() {
            rpass.set_bind_group(1, skin.bind_group(), &[]);
            rpass.set_vertex_buffer(0, mesh.vertices().slice(..));
            rpass.set_vertex_buffer(1, mesh.attribute(Attribute::Texcoords0).unwrap().slice(..));
            rpass.set_vertex_buffer(2, mesh.attribute(Attribute::BoneIndices).unwrap().slice(..));
            rpass.set_vertex_buffer(3, mesh.attribute(Attribute::BoneWeights).unwrap().slice(..));
            for (idx, submesh) in mesh.submeshes().iter().enumerate() {
                rpass.set_bind_group(2, materials[idx].bind_group(), &[]);
                rpass.set_index_buffer(submesh.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..submesh.count(), 0, 0..1);
            }
        }
    }
}



/// 메쉬의 텍스처 쉐이더입니다.
#[derive(Debug)]
struct MeshTextureShader;

impl MeshTextureShader {
    /// 쉐이더 모듈을 반환합니다.
    #[must_use]
    fn shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        #[cfg(feature = "enable-shader-validation")] {
            device.create_shader_module(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/mesh_texture.wgsl"))
            )

        }
        #[cfg(not(feature = "enable-shader-validation"))] {
            device.create_shader_module_unchecked(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/mesh_texture.wgsl"))
            )
        }
    }

    /// 파이프라인 레이아웃을 반환합니다.
    #[must_use]
    fn layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("PipelineLayout(MeshTextureShader)"), 
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
    }

    /// 렌더링 파이프라인을 생성합니다.
    #[must_use]
    fn new(device: &wgpu::Device) -> wgpu::RenderPipeline {
        let module = Self::shader(device);
        let layout = Self::layout(device);
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(MeshTextureShader"), 
                layout: Some(&layout), 
                vertex: wgpu::VertexState {
                    module: &module, 
                    entry_point: "vs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    buffers: &[
                        // 메쉬의 위치 데이터입니다.
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float3>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3, 
                                }, 
                            ], 
                        }, 
                        // 메쉬의 텍스처 좌표계 데이터입니다.
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float2>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 1, 
                                    format: wgpu::VertexFormat::Float32x2, 
                                }, 
                            ], 
                        }, 
                    ], 
                }, 
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    front_face: wgpu::FrontFace::Ccw, 
                    cull_mode: Some(wgpu::Face::Back), 
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT, 
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
                            format: SWAPCHAIN_FORMAT, 
                            blend: None, 
                            write_mask: wgpu::ColorWrites::ALL, 
                        }), 
                    ], 
                }), 
                multiview: None, 
                cache: None, 
            }, 
        )
    }

    /// 렌더링 파이프라인을 반환합니다.
    #[must_use]
    pub fn get(device: &wgpu::Device) -> &'static wgpu::RenderPipeline {
        static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
        PIPELINE.get_or_init(|| MeshTextureShader::new(device))
    }
}



/// 스키닝된 메쉬의 텍스처 쉐이더입니다.
#[derive(Debug)]
struct SkinMeshTextureShader;

impl SkinMeshTextureShader {
    /// 쉐이더 모듈을 반환합니다.
    #[must_use]
    pub fn shader(device: &wgpu::Device) -> wgpu::ShaderModule {
        #[cfg(feature = "enable-shader-validation")] {
            device.create_shader_module(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/skin_mesh_texture.wgsl"))
            )

        }
        #[cfg(not(feature = "enable-shader-validation"))] {
            device.create_shader_module_unchecked(
                wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/shader/skin_mesh_texture.wgsl"))
            )
        }
    }

    /// 파이프라인 레이아웃을 반환합니다.
    #[must_use]
    pub fn layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("PipelineLayout(SkinMeshTextureShader)"), 
                bind_group_layouts: &[
                    // 0번 그룹: 카메라 데이터
                    CameraObject::layout(device), 
                    // 1번 그룹: 스키닝 오브젝트 데이터
                    Skin::layout(device), 
                    // 2번 그룹: 재질 데이터
                    Material::layout(device), 
                ], 
                push_constant_ranges: &[]
            }
        )
    }

    /// 렌더링 파이프라인을 생성합니다.
    #[must_use]
    fn new(device: &wgpu::Device) -> wgpu::RenderPipeline {
        let module = Self::shader(device);
        let layout = Self::layout(device);
        device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(SkinMeshTextureShader"), 
                layout: Some(&layout), 
                vertex: wgpu::VertexState {
                    module: &module, 
                    entry_point: "vs_main", 
                    compilation_options: wgpu::PipelineCompilationOptions::default(), 
                    buffers: &[
                        // 메쉬의 위치 데이터입니다.
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float3>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 0, 
                                    format: wgpu::VertexFormat::Float32x3, 
                                }, 
                            ], 
                        }, 
                        // 메쉬의 텍스처 좌표계 데이터입니다.
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float2>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 1, 
                                    format: wgpu::VertexFormat::Float32x2, 
                                }, 
                            ], 
                        }, 
                        // 메쉬의 뼈 번호 데이터입니다.
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::UInteger4>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 2, 
                                    format: wgpu::VertexFormat::Uint32x4, 
                                }, 
                            ], 
                        }, 
                        // 메쉬의 뼈 가중치 데이터입니다.
                        wgpu::VertexBufferLayout {
                            array_stride: mem::size_of::<gmm::Float4>() as wgpu::BufferAddress, 
                            step_mode: wgpu::VertexStepMode::Vertex, 
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0, 
                                    shader_location: 3, 
                                    format: wgpu::VertexFormat::Float32x4, 
                                }, 
                            ], 
                        }, 
                    ], 
                }, 
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, 
                    front_face: wgpu::FrontFace::Ccw, 
                    cull_mode: Some(wgpu::Face::Back), 
                    polygon_mode: wgpu::PolygonMode::Fill, 
                    ..Default::default()
                }, 
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT, 
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
                            format: SWAPCHAIN_FORMAT, 
                            blend: None, 
                            write_mask: wgpu::ColorWrites::ALL, 
                        }), 
                    ], 
                }), 
                multiview: None, 
                cache: None, 
            }, 
        )
    }

    /// 렌더링 파이프라인을 반환합니다.
    #[must_use]
    pub fn get(device: &wgpu::Device) -> &'static wgpu::RenderPipeline {
        static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
        PIPELINE.get_or_init(|| SkinMeshTextureShader::new(device))
    }
}
