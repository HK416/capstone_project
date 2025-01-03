mod aris_original;

use std::sync::Arc;

use ahash::RandomState;
use dashmap::DashMap;
use hecs::{Entity, QueryOneError, ViewBorrow, With, World};
use mod_app::asset::AssetManager;
use mod_parallelism::collections::Queue;
use mod_render::{
    AttributeKind, CameraResource, GraphicsPipelinePool, MaterialResource, Mesh, MeshResource,
};

use crate::component::{
    create_character_render_pipeline, Acceleration, AnimationState, AnimationTimer, Character,
    CharacterInvMass, Child, Direction, Force, MaxCharacterSpeed, MovementState, Sibling,
    ThirdPersonCamera, Timer, ToParentTrans, Velocity, ViewState, MAX_CONTROL_INPUT_TIME,
    MAX_IN_OUT_TIME,
};

pub const IDLE_ANIMATION_SUFFIX: &'static str = "_Normal_Idle";
pub const MOVING_ANIMATION_SUFFIX: &'static str = "_Move_Ing";
pub const MOVE_TO_END_ANIMATION_SUFFIX: &'static str = "_Move_End_Normal";
pub const CAFE_WALK_ANIMATION_SUFFIX: &'static str = "_Cafe_Walk";
pub const ATTACK_START_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Start";
pub const ATTACK_ING_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_Ing";
pub const ATTACK_END_ANIMATION_SUFFIX: &'static str = "_Normal_Attack_End";
pub const RELOAD_ANIMATION_SUFFIX: &'static str = "_Normal_Reload";
pub const EXS_ANIMATION_SUFFIX: &'static str = "_Exs";

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
/// - `player_entity`는 캐릭터 식별자(`Character`), 로컬 변환 행렬(`ToParentTrans`)을 갖고 있어야 합니다.
/// 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - `camera_entity`는 삼인칭 카메라 요소(`ThirdPersonCamera`)를 갖고 있어야 합니다.
/// 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_player_character_direction(
    world: &mut World,
    player_entity: Entity,
    camera_entity: Entity,
    direction: &Direction,
    view_state: ViewState,
    view_state_timer: Timer,
) {
    const FUNC_TABLE: [fn(&mut World, Entity, &Direction, Timer) -> glam::Vec4; 4] = [
        update_player_character_direction_when_idle_state,
        update_player_character_direction_when_zoom_in_state,
        update_player_character_direction_when_zoom_out_state,
        update_player_character_direction_when_aiming_state,
    ];
    let index = view_state as usize;
    let direction = FUNC_TABLE[index](world, camera_entity, direction, view_state_timer);

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
    _view_state_timer: Timer,
) -> glam::Vec4 {
    direction.0
}

/// `ViewState::ZoomIn`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_zoom_in_state(
    world: &mut World,
    camera_entity: Entity,
    direction: &Direction,
    view_state_timer: Timer,
) -> glam::Vec4 {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 뷰 상태 경과 시간에 따라 플레이어 방향과 삼인칭 카메라가 바라보는 방향을 선형보간합니다.
    let t = view_state_timer.0 / MAX_IN_OUT_TIME;
    let look = third_person_camera
        .view_matrix_xz
        .z_axis
        .normalize_or(glam::Vec4::Z);
    direction.0.lerp(look, t)
}

/// `ViewState::ZoomOut`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_zoom_out_state(
    world: &mut World,
    camera_entity: Entity,
    direction: &Direction,
    view_state_timer: Timer,
) -> glam::Vec4 {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 뷰 상태 경과 시간에 따라 플레이어 방향과 삼인칭 카메라가 바라보는 방향을 선형보간합니다.
    let t = view_state_timer.0 / MAX_IN_OUT_TIME;
    let look = third_person_camera
        .view_matrix_xz
        .z_axis
        .normalize_or(glam::Vec4::Z);

    look.lerp(direction.0, t)
}

/// `ViewState::Aiming`일 때 플레이어 캐릭터의 방향을 갱신합니다.
fn update_player_character_direction_when_aiming_state(
    world: &mut World,
    camera_entity: Entity,
    _direction: &Direction,
    _view_state_timer: Timer,
) -> glam::Vec4 {
    // 카메라 엔터티의 삼인칭 카메라 요소를 가져옵니다.
    let third_person_camera = world
        .query_one_mut::<&ThirdPersonCamera>(camera_entity)
        .expect("invalid entity or invalid entity component");

    // 삼인칭 카메라가 바라보는 방향을 반환합니다.
    third_person_camera
        .view_matrix_xz
        .z_axis
        .normalize_or(glam::Vec4::Z)
}

/// 플레이어 캐릭터 엔터티의 위치를 갱신하는 함수입니다.
///
/// # Note
/// - 이 함수는 플레이어 방향을 갱신한 후 호출되어야 합니다.
/// - 이 함수는 클라이언트에서 위치를 보정하는 용도로 사용됩니다. 실제 플레이어의 위치는 서버에서 계산됩니다.
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

/// 플레이어 캐릭터 엔터티의 애니메이션 상태를 갱신하는 함수입니다.
///
/// # Panics
/// - 주어진 엔터티는 유효해야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 캐릭터 식별자(`Character`), 애니메이션 타이머(`AnimationTimer`), 애니메이션 상태 머신(`AnimationState`)을
/// 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_player_character_animation_state(
    world: &mut World,
    entity: Entity,
    movement_state: MovementState,
) {
    // 엔터티의 애니메이션 타이머와 애니메이션 상태 머신을 가져옵니다.
    type Q<'a> = (&'a mut AnimationTimer, &'a mut AnimationState);
    let (timer, state) = world
        .query_one_mut::<With<Q, &Character>>(entity)
        .expect("invalid entity or invalid entity component");

    // 애니메이션 상태 머신을 갱신합니다.
    let (reset_timer, next_state) = match movement_state {
        MovementState::Idle => match state {
            AnimationState::Idle => (false, AnimationState::Idle),
            AnimationState::Moving => (true, AnimationState::MoveToEnd),
            AnimationState::MoveToEnd => (false, AnimationState::MoveToEnd),
        },
        MovementState::MovingLeft
        | MovementState::MovingRight
        | MovementState::MovingForward
        | MovementState::MovingBackward
        | MovementState::MovingLeftForward
        | MovementState::MovingRightForward
        | MovementState::MovingLeftBackward
        | MovementState::MovingRightBackward => match state {
            AnimationState::Idle => (true, AnimationState::Moving),
            AnimationState::Moving => (false, AnimationState::Moving),
            AnimationState::MoveToEnd => (true, AnimationState::Moving),
        },
    };

    *state = next_state;
    if reset_timer {
        timer.reset();
    }
}

/// # System
/// 캐릭터 엔터티의 애니메이션 타이머를 갱신하는 시스템입니다.
///
/// # Panics
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않은 경우 [`panic!`]을 호출합니다.
///
pub fn update_character_animation_system(
    asset_manager: &AssetManager,
    world: &World,
    elapsed_time_sec: f32,
    batch_size: u32,
) {
    type Q<'a> = (
        &'a Character,
        &'a mut AnimationTimer,
        &'a mut AnimationState,
    );

    let mut query = world.query::<Q>();
    let mut batched_iter = query.iter_batched(batch_size);
    while let Some(query) = batched_iter.next() {
        for (_, (kind, timer, state)) in query {
            match kind {
                Character::ArisOriginal => aris_original::update_aris_original_animation_timer(
                    asset_manager,
                    timer,
                    state,
                    elapsed_time_sec,
                ),
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
    for &entity in entities {
        let query = world.query_one_mut::<&Character>(entity).cloned();
        match query {
            Ok(kind) => match kind {
                Character::ArisOriginal => {
                    aris_original::update_aris_original_animation(asset_manager, world, entity)
                }
            },
            Err(e) => match e {
                QueryOneError::NoSuchEntity => panic!("invalid entity"),
                _ => {}
            },
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
