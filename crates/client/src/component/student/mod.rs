pub mod aris_original;

use std::sync::Arc;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_render::{CameraResource, MaterialResource, MeshResource};

use crate::{
    asset::ModelAssetError,
    component::{Child, Sibling, ToParentTrans, WorldTransform},
};

use super::{AnimationTimer, Parent};

/// 모든 학생 모델의 최상위 뼈 노드 이름입니다.
const MODEL_BONE_ROOT: &'static str = "bip001";

/// ## Student Tag
/// `Entity`가 학생임을 식별하는 태그입니다.
pub struct StudentTag;

/// ## Student Halo Tag
/// `Entity`가 학생의 헤일로임을 식별하는 태그입니다.
pub struct StudentHaloTag;

/// ## Student Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StudentKind {
    ArisOriginal,
}

impl ToString for StudentKind {
    fn to_string(&self) -> String {
        match self {
            StudentKind::ArisOriginal => "Aris Original".to_string(),
        }
    }
}

/// ## Player Behavior States
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StudentBehaviorState {
    Idle,
    Moving,
    MoveToEnd,
}

impl Into<usize> for StudentBehaviorState {
    fn into(self) -> usize {
        match self {
            StudentBehaviorState::Idle => 0,
            StudentBehaviorState::Moving => 1,
            StudentBehaviorState::MoveToEnd => 2,
        }
    }
}

/// 쉐이더 모듈을 생성합니다.
fn create_student_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/student.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_unchecked(desc) }
    }
}

/// 쉐이더 모듈을 생성합니다.
fn create_student_halo_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/student_halo.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_unchecked(desc) }
    }
}

/// 파이프라인 레이아웃을 생성합니다.
fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("PipelineLayout(Student)"),
        bind_group_layouts: &[
            CameraResource::bind_group_layout(device),
            MeshResource::bind_group_layout(device),
            MaterialResource::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    })
}

/// 렌더링 파이프라인을 생성합니다.
pub fn create_student_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_student_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Student)"),
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
                // 4번 입력 속성: 뼈 번호
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[u32; 4]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 4,
                        format: wgpu::VertexFormat::Uint32x4,
                    }],
                    step_mode: wgpu::VertexStepMode::Vertex,
                },
                // 5번 입력 속성: 뼈 가중치
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 5,
                        format: wgpu::VertexFormat::Float32x4,
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

/// 렌더링 파이프라인을 생성합니다.
pub fn create_student_halo_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_student_halo_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Student)"),
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
                // 1번 입력 속성: 0번 텍스처 좌표
                wgpu::VertexBufferLayout {
                    array_stride: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 1,
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

/// 학생 모델을 구성하는 `Entity`를 생성합니다.
///
/// 기본으로 가지는 `Component`: `StudentTag`, `ToParentTrans`, `WorldTransform`,
/// `AnimationTimer`, `StudentBehaviorState`, `MotionCollection`, `Child`
///
/// 학생 모델별로 가지는 `Component`
/// - `Aris_Original`: `ArisOriginal`
///
pub fn spawn_student(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    kind: StudentKind,
    state: StudentBehaviorState,
    timer: AnimationTimer,
    transform: glam::Mat4,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(StudentTag);
    builder.add(ToParentTrans(transform));
    builder.add(WorldTransform::default());

    let (tag, spawn_model) = match kind {
        StudentKind::ArisOriginal => (
            aris_original::ArisOriginal,
            aris_original::spawn_aris_original_model,
        ),
    };

    let (model, collection, mut batch_commands) =
        spawn_model(world, asset_manager, device, queue, entity)?;

    builder.add(tag);
    builder.add(timer);
    builder.add(state);
    builder.add(collection);
    builder.add(Child(model));

    let (halo, mut batch_commands_1) =
        spawn_student_halo(world, asset_manager, device, queue, kind, entity, transform)?;
    let (last_entity, last_builder) = batch_commands.last_mut().unwrap();
    debug_assert_eq!(*last_entity, model);
    last_builder.add(Sibling(halo));

    batch_commands.append(&mut batch_commands_1);
    batch_commands.push((entity, builder));
    Ok((entity, batch_commands))
}

/// 학생 헤일로 모델을 구성하는 `Entity`를 생성합니다.
///
/// 기본으로 가지는 `Component`: `StudentHaloTag`, `ToParentTrans`, `WorldTransform`, `Child`
///
/// 학생 모델별로 가지는 `Component`
/// - `Aris_Original`: `ArisOriginalHalo`
///
pub fn spawn_student_halo(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    kind: StudentKind,
    parent: Entity,
    transform: glam::Mat4,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(Parent(parent));
    builder.add(StudentHaloTag);
    builder.add(ToParentTrans(transform));
    builder.add(WorldTransform::default());

    let spawn_model = match kind {
        StudentKind::ArisOriginal => aris_original::spawn_aris_original_halo_model,
    };

    let (model, mut batch_commands) = spawn_model(world, asset_manager, device, queue, entity)?;
    builder.add(Child(model));

    batch_commands.push((entity, builder));
    Ok((entity, batch_commands))
}
