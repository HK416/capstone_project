pub mod aris_original;

use std::sync::Arc;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_render::{CameraResource, MaterialResource, MeshResource};

use crate::{
    asset::ModelAssetError,
    component::{Acceleration, Child, Force, Sibling, ToParentTrans, Velocity, WorldTransform},
};

use super::{AnimationTimer, Parent};

/// 모든 학생 모델의 최상위 뼈 노드 이름입니다.
const MODEL_BONE_ROOT: &'static str = "Bip001";

/// ## Tag
/// 엔터티가 캐릭터임을 식별하는 태그입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Character {
    ArisOriginal,
}

impl ToString for Character {
    fn to_string(&self) -> String {
        match self {
            Character::ArisOriginal => "Aris Original",
        }
        .to_string()
    }
}

impl Into<CharacterHalo> for Character {
    fn into(self) -> CharacterHalo {
        match self {
            Character::ArisOriginal => CharacterHalo::ArisOriginalHalo,
        }
    }
}

/// ## Tag
/// 엔터티가 캐릭터의 헤일로임을 식별하는 태그입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterHalo {
    ArisOriginalHalo,
}

impl ToString for CharacterHalo {
    fn to_string(&self) -> String {
        match self {
            CharacterHalo::ArisOriginalHalo => "Aris Original Halo",
        }
        .to_string()
    }
}

/// ## Character Animation States
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnimationState {
    Idle,
    Moving,
    MoveToEnd,
}

/// 캐릭터 쉐이더 모듈을 생성합니다.
fn create_character_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/character.wgsl"
    ));

    if cfg!(feature = "enable-shader-validation") {
        device.create_shader_module(desc)
    } else {
        unsafe { device.create_shader_module_unchecked(desc) }
    }
}

/// 캐릭터 헤일로 쉐이더 모듈을 생성합니다.
fn create_character_halo_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    let desc = wgpu::include_wgsl!(concat!(
        env!("CARGO_WORKSPACE_DIR"),
        "/assets/shaders/character_halo.wgsl"
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

/// 캐릭터 모델 렌더링 파이프라인을 생성합니다.
pub fn create_character_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_character_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(Character)"),
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

/// 캐릭터 헤일로 렌더링 파이프라인을 생성합니다.
pub fn create_student_halo_render_pipeline(
    device: &wgpu::Device,
    depth_stencil_format: wgpu::TextureFormat,
    render_target_format: wgpu::TextureFormat,
) -> Arc<wgpu::RenderPipeline> {
    let module = create_character_halo_shader_module(device);
    let layout = create_pipeline_layout(device);
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("RenderPipeline(CharacterHalo)"),
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

/// 캐릭터 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 다음 컴포넌트를 가집니다.
/// - `AnimationState`
/// - `AnimationTimer`
/// - `Arc<SkinningAnimation>`
/// - `Character`
/// - `Child`
/// - `ToParentTrans`
/// - `WorldTransform`
///
pub fn spawn_character(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    kind: Character,
    state: AnimationState,
    timer: AnimationTimer,
    transform: glam::Mat4,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(kind);
    builder.add(ToParentTrans(transform));
    builder.add(WorldTransform::default());
    builder.add(Force(glam::Vec4::ZERO));
    builder.add(Acceleration(glam::Vec4::ZERO));
    builder.add(Velocity(glam::Vec4::ZERO));

    let spawn_model_fn = match kind {
        Character::ArisOriginal => aris_original::spawn_aris_original_model,
    };

    let (model, collection, mut batch_commands) =
        spawn_model_fn(world, asset_manager, device, queue, entity)?;

    builder.add(timer);
    builder.add(state);
    builder.add(collection);
    builder.add(Child(model));

    let (halo, mut batch_commands_1) = spawn_character_halo(
        world,
        asset_manager,
        device,
        queue,
        kind.into(),
        entity,
        transform,
    )?;
    let (last_entity, last_builder) = batch_commands.last_mut().unwrap();
    debug_assert_eq!(*last_entity, model);
    last_builder.add(Sibling(halo));

    batch_commands.append(&mut batch_commands_1);
    batch_commands.push((entity, builder));
    Ok((entity, batch_commands))
}

/// 학생 캐릭터 헤일로 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 다음 컴포넌트를 가집니다.
/// - `Child`
/// - `CharacterHalo`
/// - `ToParentTrans`
/// - `WorldTransform`
///
pub fn spawn_character_halo(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    kind: CharacterHalo,
    parent: Entity,
    transform: glam::Mat4,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(kind);
    builder.add(Parent(parent));
    builder.add(ToParentTrans(transform));
    builder.add(WorldTransform::default());

    let spawn_model = match kind {
        CharacterHalo::ArisOriginalHalo => aris_original::spawn_aris_original_halo_model,
    };

    let (model, mut batch_commands) = spawn_model(world, asset_manager, device, queue, entity)?;
    builder.add(Child(model));

    batch_commands.push((entity, builder));
    Ok((entity, batch_commands))
}
