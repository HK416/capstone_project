//! 스테이지와 관련된 그래픽스, 컴퓨트 파이프라인 코드를 관리합니디.
//!

use std::sync::{Arc, OnceLock};

use hecs::{With, World};

use crate::component::{
    AttributeKind, CameraResource, MaterialResource, Mesh, MeshResource, StageMaterialResource,
    StageTag,
};

/// 쉐이더 모듈을 생성합니다.
fn create_shadoer_module(device: &wgpu::Device) -> &'static wgpu::ShaderModule {
    static MODULE: OnceLock<wgpu::ShaderModule> = OnceLock::new();
    MODULE.get_or_init(|| unsafe {
        let desc = wgpu::include_wgsl!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/shaders/stage.wgsl",
        ));

        if cfg!(feature = "enable-shader-validation") {
            device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::checked())
        } else {
            device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked())
        }
    })
}

/// 스테이지를 그리는 그래픽스 파이프라인입니다.
pub struct StageRenderPipeline;

impl StageRenderPipeline {
    /// 파이프라인 레이아웃을 생성합니다.
    fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PipelineLayout(Stage)"),
            bind_group_layouts: &[
                CameraResource::bind_group_layout(device),
                MeshResource::bind_group_layout(device),
                StageMaterialResource::bind_group_layout(device),
            ],
            push_constant_ranges: &[],
        })
    }

    /// 렌더링 파이프라인을 가져옵니다.
    pub fn get(
        device: &wgpu::Device,
        render_target_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
    ) -> &'static wgpu::RenderPipeline {
        static PIPELINE: OnceLock<wgpu::RenderPipeline> = OnceLock::new();
        PIPELINE.get_or_init(|| {
            let module = create_shadoer_module(device);
            let layout = Self::create_pipeline_layout(device);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("RenderPipeline(Stage)"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &[
                        // 0번 입력 속성: 위치
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            }],
                            step_mode: wgpu::VertexStepMode::Vertex,
                        },
                        // 1번 입력 속성: 노멀
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            }],
                            step_mode: wgpu::VertexStepMode::Vertex,
                        },
                        // 2번 입력 속성: 0번 텍스처 좌표
                        wgpu::VertexBufferLayout {
                            array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            }],
                            step_mode: wgpu::VertexStepMode::Vertex,
                        },
                    ],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    front_face: wgpu::FrontFace::Cw,
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    depth_compare: wgpu::CompareFunction::Less,
                    depth_write_enabled: true,
                    format: depth_stencil_format,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
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
        })
    }
}

/// 스테이지 지역 모델을 그립니다.
pub fn draw_stage<'a>(
    world: &'a World,
    camera_resource: &'a CameraResource,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    // 스테이지 지역 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = StageRenderPipeline::get(device, render_target_format, depth_stencil_format);
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

    type Query<'a> = (&'a Arc<Mesh>, &'a MeshResource, &'a Vec<MaterialResource>);
    let mut query = world.query::<With<Query, &StageTag>>();
    for (_, (mesh, mesh_resource, materials)) in query.iter() {
        // 메쉬 쉐이더 리소스를 렌더 패스에 바인드합니다.
        rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);

        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Tangent, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

        for (index, submesh) in mesh.submeshes().iter().enumerate() {
            // 메쉬의 인덱스 버퍼를 바인드합니다.
            rpass.set_index_buffer(submesh.slice(..), submesh.format());

            // 재질의 쉐이더 리소스를 바인드합니다.
            rpass.set_bind_group(2, materials[index].bind_group(), &[]);

            // 인덱스 버퍼를 사용하여 그립니다.
            rpass.draw_indexed(0..submesh.count(), 0, 0..1);
        }
    }
}
