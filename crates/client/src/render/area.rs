use std::sync::Arc;

use hecs::{With, World};
use mod_render::{CameraResource, GraphicsPipelinePool, MaterialResource};

use crate::component::{AttributeKind, Mesh, MeshResource, StageArea};

use super::shadow::ShadowMapResource;

pub const STAGE_PIPELINE_ID: &'static str = "Stage";
pub const STAGE_SHADOW_PIPELINE_ID: &'static str = "Shadow(Stage)";

/// 쉐이더 모듈을 생성합니다.
fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/area.wgsl"
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
        label: Some(&format!("PipelineLayout({})", &STAGE_PIPELINE_ID)),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            MeshResource::bind_group_layout(device),
            MaterialResource::bind_group_layout(device),
            ShadowMapResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 그림자 파이프라인 레이아웃을 생성합니다.
fn create_shadow_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("PipelineLayout({})", &STAGE_SHADOW_PIPELINE_ID)),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            MeshResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 스테이지 지역 모델 렌더링 파이프라인을 생성합니다.
pub fn create_stage_area_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("RenderPipeline({})", &STAGE_PIPELINE_ID)),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
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
                // 2번 입력 속성: 탄젠트 공간 노멀
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 3번 입력 속성: 0번 텍스처 좌표
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 3,
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
    });

    Arc::new(pipeline)
}

/// 그림자를 생성하는 그래픽스 파이프라인을 생성합니다.
pub fn create_stage_area_shadow_render_pipeline(
    device: &wgpu::Device,
    shadow_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_shader_module(device);
    let layout = create_shadow_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("RenderPipeline({})", STAGE_SHADOW_PIPELINE_ID)),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_bake"),
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
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            front_face: wgpu::FrontFace::Cw,
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: device
                .features()
                .contains(wgpu::Features::DEPTH_CLIP_CONTROL),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            depth_compare: wgpu::CompareFunction::LessEqual,
            depth_write_enabled: true,
            format: shadow_format,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview: None,
        cache: None,
    });

    Arc::new(pipeline)
}

/// 스테이지 지역 모델을 그립니다.
pub fn draw_stage_area<'a>(
    world: &'a World,
    camera_resource: &'a CameraResource,
    shadow_resource: &'a ShadowMapResource,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    // 스테이지 지역 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init(STAGE_PIPELINE_ID, || {
        create_stage_area_render_pipeline(device, depth_stencil_format, render_target_format)
    });
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

    // 그림자 쉐이더 리소스를 랜더 패스에 바인드합니다.
    rpass.set_bind_group(3, &shadow_resource.bind_group, &[]);

    type Query<'a> = (
        &'a Arc<Mesh>,
        &'a MeshResource,
        &'a Vec<Arc<MaterialResource>>,
    );
    let mut query = world.query::<With<Query, &StageArea>>();
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
            rpass.set_bind_group(2, &materials[index].bind_group, &[]);

            // 인덱스 버퍼를 사용하여 그립니다.
            rpass.draw_indexed(0..submesh.count(), 0, 0..1);
        }
    }
}

/// 그림자를 생성합니다.
pub fn bake_stage_area<'a>(
    world: &'a World,
    camera_resource: &'a CameraResource,
    device: &wgpu::Device,
    shadow_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    // 스테이지 지역 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init(STAGE_SHADOW_PIPELINE_ID, || {
        create_stage_area_shadow_render_pipeline(device, shadow_format)
    });
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

    type Query<'a> = (&'a Arc<Mesh>, &'a MeshResource);
    let mut query = world.query::<With<Query, &StageArea>>();
    for (_, (mesh, mesh_resource)) in query.iter() {
        // 메쉬 쉐이더 리소스를 렌더 패스에 바인드합니다.
        rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);

        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));

        for submesh in mesh.submeshes().iter() {
            // 메쉬의 인덱스 버퍼를 바인드합니다.
            rpass.set_index_buffer(submesh.slice(..), submesh.format());

            // 인덱스 버퍼를 사용하여 그립니다.
            rpass.draw_indexed(0..submesh.count(), 0, 0..1);
        }
    }
}
