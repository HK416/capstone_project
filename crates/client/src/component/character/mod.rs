pub mod animation;
pub mod aris_original;

use std::sync::Arc;

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, With, World};
use mod_app::asset::AssetManager;
use mod_network::{
    components::{CharacterKind, MovementState, ViewState, ViewStateTimer},
    Player,
};
use mod_parallelism::collections::Queue;
use mod_render::{
    AttributeKind, CameraResource, GraphicsPipelinePool, MaterialResource, Mesh, MeshResource,
};

use crate::{
    asset::ModelAssetError,
    component::{Acceleration, Child, Force, Sibling, ToParentTrans, Velocity, WorldTransform},
    render::{create_character_render_pipeline, CHARACTER_PIPELINE_NAME},
};

pub use self::animation::*;

use super::{MoveDirection, ThirdPersonCamera};

/// 캐릭터 헤일로의 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterHaloKind {
    ArisOriginalHalo = 0,
}

impl From<CharacterKind> for CharacterHaloKind {
    fn from(value: CharacterKind) -> Self {
        match value {
            CharacterKind::ArisOriginal => CharacterHaloKind::ArisOriginalHalo,
            CharacterKind::MomoiOriginal => todo!(),
        }
    }
}

impl ToString for CharacterHaloKind {
    fn to_string(&self) -> String {
        match self {
            CharacterHaloKind::ArisOriginalHalo => "Aris Original Halo",
        }
        .to_string()
    }
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
/// - 행동 상태(`ActionState`)
/// - 행동 상태 지속 시간 타이머(`ActionStateTimer`)
/// - 움직임 상태(`MovementState`)
/// - 움직임 상태 지속 시간 타이머(`MovementStateTimer`)
/// - 시야 상태(`ViewState`)
/// - 시야 상태 지속 시간 타이머(`ViewStateTimer`)
///
pub fn spawn_player_character(
    player_data: &Player,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let character_kind = player_data.character_kind;
    let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
        glam::Quat::from_array(player_data.rotation),
        glam::Vec3::from_array(player_data.translation),
    ));
    let world_transform = WorldTransform::default();
    let action_state = player_data.action_state;
    let action_state_timer = player_data.action_state_timer;
    let movement_state = player_data.movement_state;
    let movement_state_timer = player_data.movement_state_timer;
    let view_state = player_data.view_state;
    let view_state_timer = player_data.view_state_timer;

    // 컴포넌트를 추가합니다.
    builder.add(character_kind);
    builder.add(local_transform);
    builder.add(world_transform);
    builder.add_bundle((
        Force::default(),
        Acceleration::default(),
        Velocity::default(),
    ));
    builder.add_bundle((action_state, action_state_timer));
    builder.add_bundle((movement_state, movement_state_timer));
    builder.add_bundle((view_state, view_state_timer));

    // 캐릭터 종류에 따른 캐릭터 모델을 구성하는 엔터티를 생성합니다.
    let parent = entity;
    let (model_root_entity, skinning_animation, mut batch_commands) = match character_kind {
        CharacterKind::ArisOriginal => {
            aris_original::spawn_aris_original_model(asset_manager, device, queue, world, parent)
        }
        CharacterKind::MomoiOriginal => todo!(),
    }?;

    // 캐릭터 모델 루트 노드와 스키닝 애니메이션 컴포넌트를 추가합니다.
    builder.add(Child(model_root_entity));
    builder.add(skinning_animation);

    // 캐릭터 종류에 따른 캐릭터 헤일로 모델을 구성하는 엔터티를 생성합니다.
    let parent = entity;
    let (halo_root_entity, mut halo_batch_commands) = match character_kind {
        CharacterKind::ArisOriginal => aris_original::spawn_aris_original_model_halo(
            asset_manager,
            device,
            queue,
            world,
            parent,
        ),
        CharacterKind::MomoiOriginal => todo!(),
    }?;

    // 캐릭터 헤일로 모델의 최상위 엔터티를 캐릭터 모델 엔터티의 형제 엔터티로 추가합니다.
    let (last_entity, last_builder) = batch_commands
        .last_mut()
        .expect("entity builder must not be empty");
    assert_eq!(*last_entity, model_root_entity);
    last_builder.add(Sibling(halo_root_entity));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.append(&mut halo_batch_commands);
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
    let map = categorize_character_resource(world, &entities);

    // 캐릭터 모델 렌더링 파이프라인을 가져와 렌더 패스에 바인드합니다.
    let pipeline = GraphicsPipelinePool::get_or_init(CHARACTER_PIPELINE_NAME, || {
        create_character_render_pipeline(device, depth_stencil_format, render_target_format)
    });
    rpass.set_pipeline(&pipeline);

    // 카메라 쉐이더 리소스를 렌더 패스에 바인드합니다.
    rpass.set_bind_group(0, &camera_resource.bind_group, &[]);

    for (mesh, queue) in map.iter() {
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
type MeshResourcesMap = HashMap<Arc<Mesh>, Queue<(Arc<MeshResource>, Vec<Arc<MaterialResource>>)>>;

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
    let resource_view = &world.view::<With<DrawQuery, &CharacterKind>>();
    let mut mesh_resource_map: MeshResourcesMap = HashMap::default();
    for &entity in entities {
        categorize_character_resource_recursion(
            child_view,
            sibling_view,
            resource_view,
            &mut mesh_resource_map,
            entity,
        );
    }
    mesh_resource_map
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
    resource_view: &ViewBorrow<'_, With<DrawQuery, &CharacterKind>>,
    mesh_resource_map: &mut MeshResourcesMap,
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

/// 플레이어 캐릭터의 방향을 갱신합니다.
/// 이 함수는 캐릭터가 바라보는 방향을 변경합니다. (플레이어 움직임 방향과 다름)
///
/// # Note
/// 이 함수를 호출하기 전에 `MovementState`, `ViewState`, `ViewStateTimer`, `MoveDirection`, `ThirdPersonCamera`가
/// 갱신되어야 합니다.
///
pub fn update_character_direction(
    movement_state: MovementState,
    view_state: ViewState,
    view_state_timer: ViewStateTimer,
    move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    type Func = fn(ViewStateTimer, &MoveDirection, &ThirdPersonCamera, &mut ToParentTrans);
    const FUNC_TABLE: [[Func; 4]; 3] = [
        // `MovementState::Idle`
        [
            update_character_direction_when_idle,     // ViewState::Idle
            update_character_direction_when_zoom_in,  // ViewState::ZoomIn
            update_character_direction_when_zoom_out, // ViewState::ZoomOut
            update_character_direction_when_aiming,   // ViewState::Aiming
        ],
        // `MovementState::Moving`
        [
            update_character_direction_when_moving, // ViewState::Idle
            update_character_direction_when_zoom_in_move, // ViewState::ZoomIn
            update_character_direction_when_zoom_out_move, // ViewState::ZoomOut
            update_character_direction_when_aiming, // ViewState::Aiming
        ],
        // `MovementState::MoveToEnd`
        [
            update_character_direction_when_idle,     // ViewState::Idle
            update_character_direction_when_zoom_in,  // ViewState::ZoomIn
            update_character_direction_when_zoom_out, // ViewState::ZoomOut
            update_character_direction_when_aiming,   // ViewState::Aiming
        ],
    ];

    let i = movement_state as usize;
    let j = view_state as usize;
    FUNC_TABLE[i][j](
        view_state_timer,
        move_direction,
        third_person_camera,
        local_transform,
    );
}

/// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ViewState::Idle`일 때 캐릭터의 방향을 갱신합니다.
fn update_character_direction_when_idle(
    _view_state_timer: ViewStateTimer,
    _move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    _local_transform: &mut ToParentTrans,
) {
    /* empty */
}

/// `MovementState::Moving`, `ViewState::Idle`일 때 캐릭터의 방향을 갱신합니다.
fn update_character_direction_when_moving(
    _view_state_timer: ViewStateTimer,
    move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 현재 캐릭터의 방향을 가져옵니다.
    let look = local_transform.get_look_vector();

    // 플레이어 이동 방향을 가져옵니다.
    let direction = move_direction.0;

    // 두 방향을 각도에 따라 선형 보간합니다.
    let dir = look.lerp(direction, 0.1);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(dir, glam::Vec4::Y);
}

/// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ViewState::ZoomIn`일 때 캐릭터의 방향을 갱신합니다.
fn update_character_direction_when_zoom_in(
    view_state_timer: ViewStateTimer,
    _move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 삼인칭 카메라의 방향을 계산합니다.
    let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let s = view_state_timer.normalize();
    let look = look.lerp(direction, s).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}

/// `MovementState::Moving`, `ViewState::ZoomIn`일 때 캐릭터의 방향을 갱신합니다.
fn update_character_direction_when_zoom_in_move(
    view_state_timer: ViewStateTimer,
    _move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 삼인칭 카메라의 방향을 계산합니다.
    let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let s = view_state_timer.normalize();
    let look = look.lerp(direction, s).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}

/// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ViewState::ZoomOut`일 때 캐릭터의 방향을 갱신합니다.
fn update_character_direction_when_zoom_out(
    _view_state_timer: ViewStateTimer,
    _move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    _local_transform: &mut ToParentTrans,
) {
    /* empty */
}

/// `MovementState::Moving`, `ViewState::ZoomOut`일 때 캐릭터의 방향을 갱신합니다.
fn update_character_direction_when_zoom_out_move(
    view_state_timer: ViewStateTimer,
    move_direction: &MoveDirection,
    _third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 캐릭터의 방향을 가져옵니다.
    let look = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let s = view_state_timer.normalize();
    let look = move_direction
        .0
        .lerp(look, s)
        .normalize_or(move_direction.0);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}

fn update_character_direction_when_aiming(
    _view_state_timer: ViewStateTimer,
    _move_direction: &MoveDirection,
    third_person_camera: &ThirdPersonCamera,
    local_transform: &mut ToParentTrans,
) {
    // 삼인칭 카메라의 방향을 계산합니다.
    let mat = glam::Mat4::from_rotation_y(third_person_camera.yaw_angle);
    let look = mat.z_axis.normalize_or(glam::Vec4::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let look = look.lerp(direction, 0.1).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec4::Y);
}
