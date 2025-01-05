pub mod aris_original;

use std::sync::Arc;

use ahash::RandomState;
use dashmap::DashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, With, World};
use mod_app::asset::AssetManager;
use mod_parallelism::collections::Queue;
use mod_render::{
    AttributeKind, CameraResource, GraphicsPipelinePool, MaterialResource, Mesh, MeshResource,
};

use crate::{
    asset::ModelAssetError,
    component::{
        Acceleration, BoneCollection, Character, CharacterHalo, CharacterInvMass, Child,
        ControllerState, Direction, Force, MaxCharacterSpeed, MovementState, MovementStateTimer,
        Parent, Sibling, SkinningAnimation, ThirdPersonCamera, Timer, ToParentTrans, Velocity,
        ViewState, ViewStateTimer, WorldTransform, ZoomLength, MAX_CONTROL_INPUT_TIME,
    },
};

/// 모든 캐릭터 모델의 최상위 뼈 노드 이름입니다.
pub const MODEL_BONE_ROOT: &'static str = "Bip001";

pub const MODEL_BONE_PELVIS: &'static str = "Bip001_Pelvis";
pub const MODEL_BONE_L_THIGH: &'static str = "Bip001_L_Thigh";
pub const MODEL_BONE_R_THIGH: &'static str = "Bip001_R_Thigh";

pub const IDLE_ANIMATION_SUFFIX: &'static str = "_Normal_Idle";
pub const MOVING_ANIMATION_SUFFIX: &'static str = "_Move_Ing";
pub const MOVE_TO_END_ANIMATION_SUFFIX: &'static str = "_Move_End_Normal";
pub const CAFE_WALK_ANIMATION_SUFFIX: &'static str = "_Cafe_Walk";
pub const ATTACK_START_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Start";
pub const ATTACK_ING_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Ing";
pub const ATTACK_END_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_End";
pub const RELOAD_ANIMATION_SUFFIX: &'static str = "_Normal_Reload";
pub const EXS_ANIMATION_SUFFIX: &'static str = "_Exs";

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
        label: Some("PipelineLayout(Character)"),
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
/// - `Arc<SkinningAnimation>`
/// - `Character`
/// - `Child`
/// - `MovementState`
/// - `MovementStateTimer`
/// - `ToParentTrans`
/// - `ViewState`
/// - `ViewStateTimer`
/// - `WorldTransform`
///
pub fn spawn_character(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    character_kind: Character,
    movement_state: MovementState,
    movement_state_timer: MovementStateTimer,
    view_state: ViewState,
    view_state_timer: ViewStateTimer,
    transform: glam::Mat4,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(character_kind);
    builder.add(ToParentTrans(transform));
    builder.add(WorldTransform::default());
    builder.add(Force(glam::Vec4::ZERO));
    builder.add(Acceleration(glam::Vec4::ZERO));
    builder.add(Velocity(glam::Vec4::ZERO));

    let length = match character_kind {
        Character::ArisOriginal => aris_original::get_aris_original_zoom_length(asset_manager),
    };
    builder.add(length);

    let spawn_model_fn = match character_kind {
        Character::ArisOriginal => aris_original::spawn_aris_original_model,
    };

    let (model, collection, mut batch_commands) =
        spawn_model_fn(world, asset_manager, device, queue, entity)?;

    builder.add(movement_state);
    builder.add(movement_state_timer);
    builder.add(view_state);
    builder.add(view_state_timer);
    builder.add(collection);
    builder.add(Child(model));

    let (halo, mut batch_commands_1) = spawn_character_halo(
        world,
        asset_manager,
        device,
        queue,
        character_kind.into(),
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

/// 플레이어 캐릭터 속력 함수입니다.
fn speed_function(t: f32) -> f32 {
    debug_assert!(0.0 <= t && t <= 1.0, "out of bounds");
    3.0 * t * t - 2.0 * t * t * t
}

/// 플레이어 캐릭터 엔터티의 방향을 갱신하는 함수입니다.  
/// 이 함수는 캐릭터가 바라보는 방향을 변경합니다. (플레이어 방향과 다름에 주의)
///
/// # Note
/// - 이 함수는 플레이어 방향을 갱신한 후 호출되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - `player_entity`는 캐릭터 식별자(`Character`), 로컬 변환 행렬(`ToParentTrans`), 뷰 상태 머신(`ViewState`),
/// 뷰 상태 타이머(`ViewStateTimer`)를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - `camera_entity`는 삼인칭 카메라 요소(`ThirdPersonCamera`)를 갖고 있어야 합니다.
/// 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_player_character_direction(
    world: &mut World,
    player_entity: Entity,
    camera_entity: Entity,
    direction: &Direction,
) {
    type Func = for<'a, 'b> fn(
        &'a mut World,
        Entity,
        &'b Direction,
        ViewStateTimer,
        ZoomLength,
    ) -> glam::Vec4;
    const FUNC_TABLE: [Func; 4] = [
        update_player_character_direction_when_idle_state,
        update_player_character_direction_when_zoom_in_state,
        update_player_character_direction_when_zoom_out_state,
        update_player_character_direction_when_aiming_state,
    ];

    // 플레이어 엔터티에서 뷰 상태와 뷰 상태 타이머를 가져옵니다.
    type Q<'a> = (&'a ViewState, &'a ViewStateTimer, &'a ZoomLength);
    let (&view_state, &view_state_timer, &length) = world
        .query_one_mut::<With<Q, &Character>>(player_entity)
        .expect("invaild entity or invalid entity component");

    let index = view_state as usize;
    let direction = FUNC_TABLE[index](world, camera_entity, direction, view_state_timer, length);

    // 플레이어 캐릭터의 방향을 갱신합니다.
    let local_transform = world
        .query_one_mut::<With<&mut ToParentTrans, &Character>>(player_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.look_to(direction, glam::Vec4::Y);
}

/// `ViewState::Idle`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_idle_state(
    _world: &mut World,
    _camera_entity: Entity,
    direction: &Direction,
    _view_state_timer: ViewStateTimer,
    _length: ZoomLength,
) -> glam::Vec4 {
    direction.0
}

/// `ViewState::ZoomIn`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_zoom_in_state(
    world: &mut World,
    camera_entity: Entity,
    direction: &Direction,
    view_state_timer: ViewStateTimer,
    length: ZoomLength,
) -> glam::Vec4 {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 뷰 상태 경과 시간에 따라 플레이어 방향과 삼인칭 카메라가 바라보는 방향을 선형보간합니다.
    debug_assert!(length.in_time > f32::EPSILON, "divide zero");
    let t = view_state_timer.0 / length.in_time;
    let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    direction.0.lerp(look, t)
}

/// `ViewState::ZoomOut`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_zoom_out_state(
    world: &mut World,
    camera_entity: Entity,
    direction: &Direction,
    view_state_timer: ViewStateTimer,
    length: ZoomLength,
) -> glam::Vec4 {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 뷰 상태 경과 시간에 따라 플레이어 방향과 삼인칭 카메라가 바라보는 방향을 선형보간합니다.
    debug_assert!(length.out_time > f32::EPSILON, "divide zero");
    let t = view_state_timer.0 / length.out_time;
    let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    look.lerp(direction.0, t)
}

/// `ViewState::Aiming`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_aiming_state(
    world: &mut World,
    camera_entity: Entity,
    _direction: &Direction,
    _view_state_timer: ViewStateTimer,
    _length: ZoomLength,
) -> glam::Vec4 {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 삼인칭 카메라가 바라보는 방향을 반환합니다.
    let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    look
}

/// 플레이어 캐릭터 엔터티의 위치를 갱신하는 함수입니다.
///
/// # Note
/// - 이 함수는 플레이어 방향을 갱신한 후 호출되어야 합니다.
/// - 이 함수는 클라이언트에서 위치를 보정하는 용도로 사용됩니다. 실제 플레이어의 위치는 서버에서 계산됩니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 힘의 총량(`Force`), 가속도(`Acceleration`), 속도(`Velocity`), 로컬 변환 행렬(`ToParentTrans`)을
/// 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn assist_player_character_translation(
    world: &mut World,
    entity: Entity,
    direction: &Direction,
    inv_mass: CharacterInvMass,
    max_speed: MaxCharacterSpeed,
    keyboard_input_time: Timer,
    fixed_time_sec: f32,
) {
    // 플레이어 캐릭터 엔터티에서 컴포넌트를 가져옵니다.
    type Q<'a> = (
        &'a mut Force,
        &'a mut Acceleration,
        &'a mut Velocity,
        &'a mut ToParentTrans,
    );
    let (force, acceleration, velocity, local_transform) = world
        .query_one_mut::<With<Q, &Character>>(entity)
        .expect("invalid entity or invalid entity component");

    // 플레이어 캐릭터의 가속도를 갱신합니다.
    acceleration.0 = force.0 * inv_mass.0;

    // 플레이어 키보드 입력 시간에 따른 캐릭터의 이동 속력을 계산합니다.
    let t = keyboard_input_time.0 / MAX_CONTROL_INPUT_TIME;
    let delta_t = speed_function(t);
    let speed = max_speed.0 * delta_t;

    // 플레이어 캐릭터의 속도를 갱신합니다.
    velocity.0 = acceleration.0 * fixed_time_sec + direction.0 * speed;

    // 플레이어의 위치를 갱신합니다.
    let distance = velocity.0 * fixed_time_sec;
    local_transform.translate_world(distance);
}

/// 플레이어 캐릭터 엔터티의 움직임 상태를 갱신하는 함수입니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 캐릭터 식별자(`Character`), 움직임 상태 머신(`MovementState`),
/// 움직임 상태 머신 타이머(`MovementStateTimer`)를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_player_character_movement_state(
    world: &mut World,
    entity: Entity,
    controller_state: ControllerState,
) {
    // 엔터티의 애니메이션 타이머와 움직임 상태 머신을 가져옵니다.
    type Q<'a> = (&'a mut MovementState, &'a mut MovementStateTimer);
    let (movement_state, movement_state_timer) = world
        .query_one_mut::<With<Q, &Character>>(entity)
        .expect("invalid entity or invalid entity component");

    // 움직임 상태 머신을 갱신합니다.
    let (reset_timer, next_state) = match controller_state {
        ControllerState::Idle => match movement_state {
            MovementState::Idle => (false, MovementState::Idle),
            MovementState::Moving => (true, MovementState::MoveToEnd),
            MovementState::MoveToEnd => (false, MovementState::MoveToEnd),
        },
        ControllerState::MovingLeft
        | ControllerState::MovingRight
        | ControllerState::MovingForward
        | ControllerState::MovingBackward
        | ControllerState::MovingLeftForward
        | ControllerState::MovingRightForward
        | ControllerState::MovingLeftBackward
        | ControllerState::MovingRightBackward => match movement_state {
            MovementState::Idle => (true, MovementState::Moving),
            MovementState::Moving => (false, MovementState::Moving),
            MovementState::MoveToEnd => (true, MovementState::Moving),
        },
    };

    *movement_state = next_state;
    if reset_timer {
        movement_state_timer.reset();
    }
}

/// # System
/// 캐릭터 엔터티의 상태 머신과 타이머를 갱신하는 시스템입니다.
///
/// # Panics
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않은 경우 [`panic!`]을 호출합니다.
///
pub fn update_character_state_and_timer_system(
    asset_manager: &AssetManager,
    world: &mut World,
    elapsed_time_sec: f32,
    batch_size: u32,
) {
    type Q<'a> = (
        &'a Character,
        &'a mut MovementState,
        &'a mut MovementStateTimer,
    );
    let mut query = world.query::<Q>();
    let mut batched_iter = query.iter_batched(batch_size);
    while let Some(query) = batched_iter.next() {
        for (_, (kind, state, timer)) in query {
            match kind {
                Character::ArisOriginal => {
                    aris_original::update_aris_original_movement_state_timer(
                        asset_manager,
                        state,
                        timer,
                        elapsed_time_sec,
                    )
                }
            };
        }
    }
}

/// 주어진 엔터티의 캐릭터 애니메이션을 갱신합니다.
///
/// 주어진 엔터티 가 캐릭터 식별자(`Character`)를 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_character_animation(
    asset_manager: &AssetManager,
    world: &mut World,
    entities: &[Entity],
) {
    let character_kind_view = world.view::<&Character>();
    let bones_view = world.view::<&Arc<BoneCollection>>();
    let skinning_view = world.view::<&Arc<SkinningAnimation>>();
    let mut local_transform_view = world.view::<&mut ToParentTrans>();
    let state_view = world.view::<(
        &MovementState,
        &MovementStateTimer,
        &ViewState,
        &ViewStateTimer,
        &ZoomLength,
    )>();

    for &entity in entities {
        let query = character_kind_view.get(entity).cloned();
        if let Some(kind) = query {
            match kind {
                Character::ArisOriginal => aris_original::update_aris_original_animation(
                    asset_manager,
                    entity,
                    &bones_view,
                    &skinning_view,
                    &mut local_transform_view,
                    &state_view,
                ),
            }
        }
    }
}

/// 캐릭터 모델을 그립니다.
pub fn draw_character<'a>(
    world: &'a World,
    entities: &[Entity],
    camera_resource: &'a CameraResource,
    device: &wgpu::Device,
    render_target_format: wgpu::TextureFormat,
    depth_stencil_format: wgpu::TextureFormat,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    // 엔터티의 쉐이더 리소스를 분류합니다.
    let map = categorize_character_resource(world, &entities);

    // 캐릭터 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init("character", || {
        create_character_render_pipeline(device, depth_stencil_format, render_target_format)
    });
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

    for pair in map.iter() {
        let mesh = pair.key();
        let queue = pair.value();

        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Tangent, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
        rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
        rpass.set_vertex_buffer(5, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

        while let Some((mesh_resource, materials)) = queue.pop() {
            // 메쉬 쉐이더 리소스를 렌더 패스에 바인드합니다.
            rpass.set_bind_group(1, &mesh_resource.bind_group, &[]);

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
}

/// 캐릭터 메쉬 - 쉐이더 리소스 맵 자료형
type MeshResourcesMap =
    DashMap<Arc<Mesh>, Queue<(Arc<MeshResource>, Vec<Arc<MaterialResource>>)>, RandomState>;

/// 캐릭터 모델을 그릴 때 사용되는 쉐이더 리소스 자료형
type DrawQuery<'a> = (
    &'a Arc<Mesh>,
    &'a Arc<MeshResource>,
    &'a Vec<Arc<MaterialResource>>,
);

/// 주어진 엔터티의 쉐이더 리소스를 분류합니다.
///
/// 엔터티가 메쉬(`Arc<Mesh>`), 메쉬 쉐이더 리소스(`Arc<MeshResource>`), 머태리얼(`Vec<Arc<MaterialResource>>`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn categorize_character_resource(world: &World, entities: &[Entity]) -> MeshResourcesMap {
    let child_view = &world.view::<&Child>();
    let sibling_view = &world.view::<&Sibling>();
    let resource_view = &world.view::<With<DrawQuery, &Character>>();
    let map: MeshResourcesMap = DashMap::default();
    let mesh_resource_map = &map;
    for &entity in entities {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            resource_view,
            mesh_resource_map,
            entity,
        );
    }
    map
}

/// 주어진 엔터티의 쉐이더 리소스를 분류합니다.
///
/// 엔터티가 메쉬(`Arc<Mesh>`), 메쉬 쉐이더 리소스(`Arc<MeshResource>`), 머태리얼(`Vec<Arc<MaterialResource>>`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn categorize_character_resource_recursion(
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    resource_view: &ViewBorrow<'_, With<DrawQuery, &Character>>,
    mesh_resource_map: &MeshResourcesMap,
    entity: Entity,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 탐색합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            resource_view,
            mesh_resource_map,
            *sibling_entity,
        );
    }

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 탐색합니다.
    if let Some(child_entity) = child_view.get(entity).cloned() {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            resource_view,
            mesh_resource_map,
            *child_entity,
        );
    }

    // 엔터티의 쉐이더 리소스 데이터를 가져옵니다.
    let results = resource_view.get(entity);
    if let Some((mesh, mesh_resource, materials)) = results {
        // 쉐이더 리소스 데이터를 분류합니다.
        let queue = mesh_resource_map
            .entry(mesh.clone())
            .or_insert(Queue::new());
        queue.push((mesh_resource.clone(), materials.clone()));
    }
}
