pub mod animation;
mod aris_original;
mod midori_original;
mod momoi_original;
mod yuuka_original;

use std::{collections::VecDeque, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, With, World};
use mod_app::asset::AssetManager;
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, LatLon, MovementState, MovementStateTimer,
    Player, ViewState, ViewStateTimer,
};
use mod_render::{
    AttributeKind, CameraResource, GraphicsPipelinePool, MaterialResource, Mesh, MeshResource,
};

use crate::{
    asset::{AssetError, ModelHierarchyPool, MotionPool},
    component::{Acceleration, Child, Force, Sibling, ToParentTrans, Velocity, WorldTransform},
    render::{
        create_character_halo_render_pipeline, create_character_render_pipeline,
        CHARACTER_HALO_PIPELINE_NAME, CHARACTER_PIPELINE_NAME,
    },
};

pub use self::animation::*;

use super::{ControllerInputFlags, MoveDirection, ThirdPersonCamera};

/// 캐릭터의 수
const NUM_CHARACTERS: usize = 4;

/// 캐릭터 헤일로의 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterHaloKind {
    ArisOriginalHalo = 0,
    MomoiOriginalHalo = 1,
    MidoriOriginalHalo = 2,
    YuukaOriginalHalo = 3,
}

impl From<CharacterKind> for CharacterHaloKind {
    fn from(value: CharacterKind) -> Self {
        match value {
            CharacterKind::ArisOriginal => CharacterHaloKind::ArisOriginalHalo,
            CharacterKind::MomoiOriginal => CharacterHaloKind::MomoiOriginalHalo,
            CharacterKind::MidoriOriginal => CharacterHaloKind::MidoriOriginalHalo,
            CharacterKind::YuukaOriginal => CharacterHaloKind::YuukaOriginalHalo,
        }
    }
}

impl ToString for CharacterHaloKind {
    fn to_string(&self) -> String {
        match self {
            CharacterHaloKind::ArisOriginalHalo => "Aris Original Halo",
            CharacterHaloKind::MomoiOriginalHalo => "Momoi Original Halo",
            CharacterHaloKind::MidoriOriginalHalo => "Midori Original Halo",
            CharacterHaloKind::YuukaOriginalHalo => "Yuuka Original Halo",
        }
        .to_string()
    }
}

/// 플레이어 캐릭터 모델의 애니메이션과 계층 구조 데이터를 풀 객체에 로드합니다.
pub fn load_character_model(
    asset_manager: &AssetManager,
    character_kind: CharacterKind,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), AssetError> {
    const MODELS: [(&'static str, &'static str); NUM_CHARACTERS] = [
        (
            aris_original::WORKSPACE,
            aris_original::MODEL_NAME,
        ),
        (
            momoi_original::WORKSPACE,
            momoi_original::MODEL_NAME,
        ),
        (
            midori_original::WORKSPACE,
            midori_original::MODEL_NAME,
        ),
        (
            yuuka_original::WORKSPACE,
            yuuka_original::MODEL_NAME,
        )
    ];

    let i = character_kind as usize;
    let (workspace, model_name) = MODELS[i];

    // 캐릭터 모델 애니메이션을 로드합니다.
    MotionPool::get_or_init(model_name, workspace, asset_manager)?;

    // 캐릭터 모델 계층 구조를 로드합니다.
    ModelHierarchyPool::get_or_init(model_name, workspace, asset_manager, device, queue)?;

    Ok(())
}

/// 플레이어 캐릭터를 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다
/// - 자식 엔터티(`Child`)
/// - 캐릭터 종류(`CharacterKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - 스키닝 애니메이션(`SkinningAnimation`)
/// - 힘의 총량(`Force`)
/// - 가속도(`Acceleration`)
/// - 속도(`Velocity`)
/// - 체력(`HealthPoint`)
/// - 행동 상태(`ActionState`)
/// - 행동 상태 지속 시간 타이머(`ActionStateTimer`)
/// - 움직임 상태(`MovementState`)
/// - 움직임 상태 지속 시간 타이머(`MovementStateTimer`)
/// - 시야 상태(`ViewState`)
/// - 시야 상태 지속 시간 타이머(`ViewStateTimer`)
/// - 시야 방향(`Latlon`)
///
pub fn spawn_player_character(
    player: &Player,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError> {
    type CharacterFunc =
        fn(
            &AssetManager,
            &wgpu::Device,
            &wgpu::Queue,
            &World,
            Entity,
        )
            -> Result<(Entity, SkinningAnimation, Vec<(Entity, EntityBuilder)>), AssetError>;
    const CHARACTER_FN: [CharacterFunc; NUM_CHARACTERS] = [
        aris_original::spawn_character_model,
        momoi_original::spawn_character_model,
        midori_original::spawn_character_model,
        yuuka_original::spawn_character_model,
    ];

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let character_kind = player.character_kind;
    let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
        glam::Quat::from_array(player.rotation),
        glam::Vec3::from_array(player.translation),
    ));
    let world_transform = WorldTransform::default();
    let health_point = player.health_point;
    let action_state = player.action_state;
    let action_state_timer = player.action_state_timer;
    let movement_state = player.movement_state;
    let movement_state_timer = player.movement_state_timer;
    let view_state = player.view_state;
    let view_state_timer = player.view_state_timer;
    let view_rotation = player.view_rotation;

    // 컴포넌트를 추가합니다.
    builder.add(character_kind);
    builder.add(local_transform);
    builder.add(world_transform);
    builder.add_bundle((
        Force::default(),
        Acceleration::default(),
        Velocity::default(),
    ));
    builder.add(health_point);
    builder.add_bundle((action_state, action_state_timer));
    builder.add_bundle((movement_state, movement_state_timer));
    builder.add_bundle((view_state, view_state_timer, view_rotation));

    // 캐릭터 종류에 따른 캐릭터 모델을 구성하는 엔터티를 생성합니다.
    let i = character_kind as usize;
    let parent = entity;
    let (model_root_entity, skinning_animation, mut batch_commands) =
        CHARACTER_FN[i](asset_manager, device, queue, world, parent)?;

    // 캐릭터 모델 루트 노드와 스키닝 애니메이션 컴포넌트를 추가합니다.
    builder.add(Child(model_root_entity));
    builder.add(skinning_animation);

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
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
    let (character, character_halo) = categorize_character_resource(world, &entities);

    // 캐릭터 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init(CHARACTER_PIPELINE_NAME, || {
        create_character_render_pipeline(device, depth_stencil_format, render_target_format)
    });
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

    for (mesh, mut queue) in character.into_iter() {
        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Tangent, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
        rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
        rpass.set_vertex_buffer(5, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

        while let Some((mesh_resource, materials)) = queue.pop_front() {
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

    // 캐릭터 헤일로 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init(CHARACTER_HALO_PIPELINE_NAME, || {
        create_character_halo_render_pipeline(device, depth_stencil_format, render_target_format)
    });
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

    for (mesh, mut queue) in character_halo.into_iter() {
        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

        while let Some((mesh_resource, materials)) = queue.pop_front() {
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

/// 캐릭터 메쉬 집합
type CharacterSet = HashMap<Arc<Mesh>, VecDeque<(Arc<MeshResource>, Vec<Arc<MaterialResource>>)>>;
/// 캐릭터 헤일로 메쉬 집합
type CharacterHaloSet =
    HashMap<Arc<Mesh>, VecDeque<(Arc<MeshResource>, Vec<Arc<MaterialResource>>)>>;

/// 모델을 그릴 때 사용되는 쉐이더 리소스 자료형
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
fn categorize_character_resource(
    world: &World,
    entities: &[Entity],
) -> (CharacterSet, CharacterHaloSet) {
    // 컴포넌트 뷰를 준비합니다.
    let child_view = &world.view::<&Child>();
    let sibling_view = &world.view::<&Sibling>();
    let character_view = &world.view::<With<DrawQuery, &CharacterKind>>();
    let character_halo_view = &world.view::<With<DrawQuery, &CharacterHaloKind>>();

    // 결과를 저장할 집합 컨테이너를 준비합니다.
    let mut character_set = HashMap::default();
    let mut character_halo_set = HashMap::default();

    // 엔터티 계층 구조를 순회합니다.
    for &entity in entities {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            character_view,
            character_halo_view,
            &mut character_set,
            &mut character_halo_set,
            entity,
        );
    }

    (character_set, character_halo_set)
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
    character_view: &ViewBorrow<'_, With<DrawQuery, &CharacterKind>>,
    character_halo_view: &ViewBorrow<'_, With<DrawQuery, &CharacterHaloKind>>,
    character_set: &mut CharacterSet,
    character_halo_set: &mut CharacterHaloSet,
    entity: Entity,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 탐색합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            character_view,
            character_halo_view,
            character_set,
            character_halo_set,
            *sibling_entity,
        );
    }

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 탐색합니다.
    if let Some(child_entity) = child_view.get(entity).cloned() {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            character_view,
            character_halo_view,
            character_set,
            character_halo_set,
            *child_entity,
        );
    }

    // 엔터티의 캐릭터 쉐이더 리소스 데이터를 가져옵니다.
    let results = character_view.get(entity);
    if let Some((mesh, mesh_resource, materials)) = results {
        let queue = character_set
            .entry(mesh.clone())
            .or_insert(VecDeque::default());
        queue.push_back((mesh_resource.clone(), materials.clone()));
    }

    // 엔터티의 캐릭터 헤일로 쉐이더 리소스 데이터를 가져옵니다.
    let results = character_halo_view.get(entity);
    if let Some((mesh, mesh_resource, materials)) = results {
        let queue = character_halo_set
            .entry(mesh.clone())
            .or_insert(VecDeque::default());
        queue.push_back((mesh_resource.clone(), materials.clone()));
    }
}

/// 플레이어 캐릭터의 방향을 갱신합니다.
/// 이 함수는 캐릭터가 바라보는 방향을 변경합니다. (플레이어 움직임 방향과 다름)
///
/// # Note
/// 이 함수를 호출하기 전에 `MovementState`, `ViewState`, `ViewStateTimer`, `MoveDirection`, `ThirdPersonCamera`가
/// 갱신되어야 합니다.
///
pub fn update_character_direction(
    character_kind: CharacterKind,
    movement_state: MovementState,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    type Func =
        fn(CharacterKind, ActionStateTimer, &MoveDirection, &ThirdPersonCamera, &mut ToParentTrans);
    const FUNC_TABLE: [[Func; 5]; 3] = [
        // `MovementState::Idle`
        [
            set_character_direction_to_none,                // ActionState::Idle
            set_character_direction_to_camera,              // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_none,                // ActionState::AimOff
            set_character_direction_to_camera,              // ActionState::Attack
        ],
        // `MovementState::Moving`
        [
            set_character_direction_to_movement, // ActionState::Idle
            set_character_direction_to_camera,   // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_current_from_camera, // ActionState::AimOff
            set_character_direction_to_camera,   // ActionState::Attack
        ],
        // `MovementState::MoveToEnd`
        [
            set_character_direction_to_none,                // ActionState::Idle
            set_character_direction_to_camera,              // ActionState::Aiminig
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_none,                // ActionState::AimOff
            set_character_direction_to_camera,              // ActionState::Attack
        ],
    ];

    let i = movement_state as usize;
    let j = action_state as usize;
    FUNC_TABLE[i][j](
        character_kind,
        action_state_timer,
        move_direction,
        third_person_camera,
        local_transform,
    );
}

/// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ActionState::Idle`일 때 캐릭터의 방향을 갱신합니다.
fn set_character_direction_to_none(
    _character_kind: CharacterKind,
    _view_state_timer: ActionStateTimer,
    _move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    _local_transform: &mut ToParentTrans,
) {
    /* empty */
}

/// `MovementState::Moving`, `ActionState::Idle`일 때 캐릭터의 방향을 갱신합니다.
fn set_character_direction_to_movement(
    _character_kind: CharacterKind,
    _view_state_timer: ActionStateTimer,
    move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 현재 캐릭터의 방향을 가져옵니다.
    let look = local_transform.get_look_vector();

    // 플레이어 이동 방향을 가져옵니다.
    let direction = move_direction.0;

    // 두 방향을 각도에 따라 선형 보간합니다.
    let dir = look.lerp(direction, 0.5);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(dir, glam::Vec4::Y);
}

/// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ViewState::ZoomIn`일 때 캐릭터의 방향을 갱신합니다.
fn set_character_direction_to_camera_from_current(
    character_kind: CharacterKind,
    view_state_timer: ActionStateTimer,
    _move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    const ZOOM_IN_LEN: [f32; NUM_CHARACTERS] = [
        aris_original::NORMAL_ATTACK_START_DURATION,
        momoi_original::NORMAL_ATTACK_START_DURATION,
        midori_original::NORMAL_ATTACK_START_DURATION,
        yuuka_original::NORMAL_ATTACK_START_DURATION,
    ];

    // 삼인칭 카메라의 방향을 계산합니다.
    let mat = glam::Mat4::from_rotation_y(third_person_camera.rotation.lon);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let i = character_kind as usize;
    let s = view_state_timer.0 / ZOOM_IN_LEN[i];
    let look = look.lerp(direction, s).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}

/// `MovementState::Moving`, `ViewState::ZoomOut`일 때 캐릭터의 방향을 갱신합니다.
fn set_character_direction_to_current_from_camera(
    character_kind: CharacterKind,
    view_state_timer: ActionStateTimer,
    move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    const ZOOM_OUT_LEN: [f32; NUM_CHARACTERS] = [
        aris_original::NORMAL_ATTACK_END_DURATION,
        momoi_original::NORMAL_ATTACK_END_DURATION,
        midori_original::NORMAL_ATTACK_END_DURATION,
        yuuka_original::NORMAL_ATTACK_END_DURATION,
    ];

    // 캐릭터의 방향을 가져옵니다.
    let look = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let i = character_kind as usize;
    let s = view_state_timer.0 / ZOOM_OUT_LEN[i];
    let look = move_direction
        .0
        .lerp(look, s)
        .normalize_or(move_direction.0);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}

fn set_character_direction_to_camera(
    _character_kind: CharacterKind,
    _view_state_timer: ActionStateTimer,
    _move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 삼인칭 카메라의 방향을 계산합니다.
    let mat = glam::Mat4::from_rotation_y(third_person_camera.rotation.lon);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let look = look.lerp(direction, 0.1).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}

/// `ControllerInputFlags`에 따라 `ActionState`를 갱신합니다.
pub fn update_action_state_by_controller_input_flags(
    character_kind: CharacterKind,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    controller_input_flags: ControllerInputFlags,
) {
    type Func = fn(&mut ActionState, &mut ActionStateTimer, ControllerInputFlags);
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::update_character_action_state,
        momoi_original::update_character_action_state,
        midori_original::update_character_action_state,
        yuuka_original::update_character_action_state,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](action_state, action_state_timer, controller_input_flags);
}

/// 주어진 경과 시간 만큼 `ActionStateTimer`를 갱신합니다.
pub fn update_action_state_timer(
    character_kind: CharacterKind,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&mut ActionState, &mut ActionStateTimer, f32);
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::update_character_action_state_timer,
        momoi_original::update_character_action_state_timer,
        midori_original::update_character_action_state_timer,
        yuuka_original::update_character_action_state_timer,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](action_state, action_state_timer, elapsed_time_sec);
}

/// 주어진 경과 시간 만큼 `MovementStateTimer`를 갱신합니다.
pub fn update_movement_state_timer(
    character_kind: CharacterKind,
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(ActionState, &mut MovementState, &mut MovementStateTimer, f32);
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::update_character_movement_state_timer,
        momoi_original::update_character_movement_state_timer,
        midori_original::update_character_movement_state_timer,
        yuuka_original::update_character_movement_state_timer,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](
        action_state,
        movement_state,
        movement_state_timer,
        elapsed_time_sec,
    );
}

/// `ControllerInputFlags`에 따라 `ViewState`를 갱신합니다.
pub fn update_view_state_by_controller_input_flags(
    character_kind: CharacterKind,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: ControllerInputFlags,
) {
    type Func = fn(&mut ViewState, &mut ViewStateTimer, ControllerInputFlags);
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::update_character_view_state,
        momoi_original::update_character_view_state,
        midori_original::update_character_view_state,
        yuuka_original::update_character_view_state,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](view_state, view_state_timer, controller_input_flags);
}

/// 주어진 경과 시간 만큼 `ViewStateTimer`를 갱신합니다.
pub fn update_view_state_timer(
    character_kind: CharacterKind,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&mut ViewState, &mut ViewStateTimer, f32);
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::update_character_view_state_timer,
        momoi_original::update_character_view_state_timer,
        midori_original::update_character_view_state_timer,
        yuuka_original::update_character_view_state_timer,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](view_state, view_state_timer, elapsed_time_sec);
}

pub fn animate_character(
    asset_manager: &AssetManager,
    character_kind: CharacterKind,
    view_rotation: LatLon,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    movement_state: MovementState,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    type Func = fn(
        &AssetManager,
        LatLon,
        ActionState,
        ActionStateTimer,
        MovementState,
        MovementStateTimer,
        &SkinningAnimation,
        &ViewBorrow<&BoneCollection>,
        &mut ViewBorrow<&mut ToParentTrans>,
    );
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::animate_character,
        momoi_original::animate_character,
        midori_original::animate_character,
        yuuka_original::animate_character,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](
        asset_manager,
        view_rotation,
        action_state,
        action_state_timer,
        movement_state,
        movement_state_timer,
        skinning_animation,
        collection_view,
        transform_view,
    );
}

/// 무기의 위치를 설정합니다.
///
/// # NOTE
/// 이 함수는 캐릭터의 월드 변환 행렬이 계산된 후 호출해야 합니다.
///
pub fn set_weapon_position(
    character_kind: CharacterKind,
    action_state: ActionState,
    skinning_animation: &SkinningAnimation,
    child_view: &ViewBorrow<&Child>,
    sibling_view: &ViewBorrow<&Sibling>,
    transform_view: &mut ViewBorrow<(&ToParentTrans, &mut WorldTransform)>,
) {
    type Func = fn(
        ActionState,
        &SkinningAnimation,
        &ViewBorrow<&Child>,
        &ViewBorrow<&Sibling>,
        &mut ViewBorrow<(&ToParentTrans, &mut WorldTransform)>,
    );
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::set_weapon_position,
        momoi_original::set_weapon_position,
        midori_original::set_weapon_position,
        yuuka_original::set_weapon_position,
    ];

    let i = character_kind as usize;
    FUNC_TABLE[i](action_state, skinning_animation, child_view, sibling_view, transform_view);
}

/// 캐릭터의 삼인칭 카메라를 생성합니다.
pub fn create_third_person_camera_of_character(character_kind: CharacterKind) -> ThirdPersonCamera {
    const CAMERA_FOV_Y: [f32; NUM_CHARACTERS] = [
        aris_original::CAMERA_IDLE_FOV_Y,
        momoi_original::CAMERA_IDLE_FOV_Y,
        midori_original::CAMERA_IDLE_FOV_Y,
        yuuka_original::CAMERA_IDLE_FOV_Y,
    ];
    const CAMERA_POSITION: [glam::Vec3A; NUM_CHARACTERS] = [
        aris_original::CAMERA_IDLE_POSITION,
        momoi_original::CAMERA_IDLE_POSITION,
        midori_original::CAMERA_IDLE_POSITION,
        yuuka_original::CAMERA_IDLE_POSITION,
    ];

    let i = character_kind as usize;
    ThirdPersonCamera {
        fov_y: CAMERA_FOV_Y[i],
        rotation: LatLon::default(),
        position: CAMERA_POSITION[i],
    }
}

/// 삼인칭 카메라를 갱신합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 `ViewState`가 먼저 갱신되어야합니다.
///
pub fn update_third_person_camera(
    third_person_camera: &mut ThirdPersonCamera,
    character_kind: CharacterKind,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state: ViewState,
    view_state_timer: ViewStateTimer,
) {
    type Func = fn(&mut ThirdPersonCamera, ActionState, ActionStateTimer, ViewStateTimer);
    const FUNC_TABLE: [[Func; 4]; NUM_CHARACTERS] = [
        [
            aris_original::update_third_person_camera_when_idle,
            aris_original::update_third_person_camera_when_zoom_in,
            aris_original::update_third_person_camera_when_zoom_out,
            aris_original::update_third_person_camera_when_aiming,
        ],
        [
            momoi_original::update_third_person_camera_when_idle,
            momoi_original::update_third_person_camera_when_zoom_in,
            momoi_original::update_third_person_camera_when_zoom_out,
            momoi_original::update_third_person_camera_when_aiming,
        ],
        [
            midori_original::update_third_person_camera_when_idle,
            midori_original::update_third_person_camera_when_zoom_in,
            midori_original::update_third_person_camera_when_zoom_out,
            midori_original::update_third_person_camera_when_aiming,
        ],
        [
            yuuka_original::update_third_person_camera_when_idle,
            yuuka_original::update_third_person_camera_when_zoom_in,
            yuuka_original::update_third_person_camera_when_zoom_out,
            yuuka_original::update_third_person_camera_when_aiming,
        ],
    ];

    let i = character_kind as usize;
    let j = view_state as usize;
    FUNC_TABLE[i][j](
        third_person_camera,
        action_state,
        action_state_timer,
        view_state_timer,
    );
}
