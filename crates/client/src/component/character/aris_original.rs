use std::sync::Arc;

use ahash::{HashMap, HashSet};
use constcat::concat;
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_app::asset::AssetManager;
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, MovementState, MovementStateTimer,
};
use mod_render::{MaterialResource, MeshResource, SkinningDataLayout};

use crate::{
    asset::{ModelAssetError, ModelHierarchyPool, Motion, MotionPool, Node},
    component::{
        BoneCollection, CharacterHaloKind, Child, Parent, Sibling, SkinningAnimation,
        ToParentTrans, WorldTransform, ATTACK_END_ANIMATION_SUFFIX, ATTACK_ING_ANIMATION_SUFFIX,
        ATTACK_START_ANIMATION_SUFFIX, CAFE_WALK_ANIMATION_SUFFIX, IDLE_ANIMATION_SUFFIX,
        MODEL_BONE_L_THIGH, MODEL_BONE_PELVIS, MODEL_BONE_ROOT, MODEL_BONE_R_THIGH,
        MOVE_TO_END_ANIMATION_SUFFIX, MOVING_ANIMATION_SUFFIX,
    },
};

/// 캐릭터 모델 에셋의 상대 경로입니다.
pub const WORKSPACE: &'static str = "characters/aris_original/";
/// 캐릭터 모델의 이름입니다.
pub const MODEL_NAME: &'static str = "Aris_Original";
/// 캐릭터 헤일로 모델의 이름입니다.
pub const MODEL_HALO_NAME: &'static str = "Aris_Original_Halo";

/// 캐릭터의 Idle 애니메이션 이름입니다.
const IDLE_ANIMATION: &'static str = concat!(MODEL_NAME, IDLE_ANIMATION_SUFFIX);
/// 캐릭터의 Moving 애니메이션 이름입니다.
const MOVING_ANIMATION: &'static str = concat!(MODEL_NAME, MOVING_ANIMATION_SUFFIX);
/// 캐릭터의 MoveToEnd 애니메이션 이름입니다.
const MOVE_TO_END_ANIMATION: &'static str = concat!(MODEL_NAME, MOVE_TO_END_ANIMATION_SUFFIX);
/// 캐릭터의 CafeWalk 애니메이션 이름입니다.
const CAFE_WALK_ANIMATION: &'static str = concat!(MODEL_NAME, CAFE_WALK_ANIMATION_SUFFIX);
/// 캐릭터의 AttackStart 애니메이션 이름입니다.
const ATTACK_START_ANIMATION: &'static str = concat!(MODEL_NAME, ATTACK_START_ANIMATION_SUFFIX);
/// 캐릭터의 Attacking 애니메이션 이름입니다.
const ATTACK_ING_ANIMATION: &'static str = concat!(MODEL_NAME, ATTACK_ING_ANIMATION_SUFFIX);
/// 캐릭터의 AttackEnd 애니메이션 이름입니다.
const ATTACK_END_ANIMATION: &'static str = concat!(MODEL_NAME, ATTACK_END_ANIMATION_SUFFIX);

/// ArisOriginal 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 뼈 엔터티 집합(`BoneCollection`)
/// - 캐릭터 종류(`CharacterKind`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>)`
///
/// # Panics
/// - 엔터티 목록에서 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn spawn_aris_original_model(
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
    parent: Entity,
) -> Result<(Entity, SkinningAnimation, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let root =
        ModelHierarchyPool::get_or_init(MODEL_NAME, WORKSPACE, asset_manager, device, queue)?;

    let mut meshes = HashMap::default();
    let mut entities = HashMap::default();
    let mut animation_mixing_bones = HashSet::default();
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_aris_original_model_recursive(
        world,
        device,
        queue,
        &mut meshes,
        &mut entities,
        &mut animation_mixing_bones,
        false,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    );

    // 스키닝 애니메이션 컴포넌트를 생성합니다.
    let skinning_animation = SkinningAnimation {
        root: entities
            .get(MODEL_BONE_ROOT)
            .cloned()
            .expect("no such entity"),
        meshes,
        animation_mixing_bones,
    };

    Ok((entity, skinning_animation, batch_commands))
}

/// ArisOriginal 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 뼈 엔터티 집합(`BoneCollection`)
/// - 캐릭터 종류(`CharacterKind`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>)`
///
/// # Panics
/// - 엔터티 목록에서 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn spawn_aris_original_model_recursive(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    meshes: &mut HashMap<String, Entity>,
    entities: &mut HashMap<String, Entity>,
    animation_mixing_bones: &mut HashSet<Entity>,
    contains_mixing_bones: bool,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 엔터티 목록에 현재 엔터티를 추가합니다.
    let node_name = current.name.clone();
    entities.insert(node_name, entity);

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform.into_mat4()));
    builder.add(WorldTransform::default());

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(child) = current.children.first() {
        /// 노드가 애니메이션 믹싱에 사용되는 뼈 집합에 포함되는지 여부를 반환합니다.
        fn contains_set(name: &str) -> bool {
            name == MODEL_BONE_L_THIGH || name == MODEL_BONE_R_THIGH || name.contains("skirt")
        }

        // 자식 엔터티를 생성하기 위한 매개변수를 준비합니다.
        let node_name = current.name.clone();
        let contains_mixing_bones = contains_mixing_bones || contains_set(&node_name);

        // 자식 엔터티를 생성합니다.
        let entity = spawn_aris_original_model_recursive(
            world,
            device,
            queue,
            meshes,
            entities,
            animation_mixing_bones,
            contains_mixing_bones,
            batch_commands,
            entity,
            child,
            &current.children[1..],
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(entity));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(sibling) = siblings.first() {
        // 형제 엔터티를 생성하기 위한 매개변수를 준비합니다.
        let current = sibling;
        let siblings = &siblings[1..];

        // 형제 엔터티를 생성합니다.
        let entity = spawn_aris_original_model_recursive(
            world,
            device,
            queue,
            meshes,
            entities,
            animation_mixing_bones,
            contains_mixing_bones,
            batch_commands,
            parent,
            current,
            siblings,
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(entity));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = current.mesh.clone() {
        // 메쉬 쉐이더 리소스를 생성합니다.
        let mesh_name = mesh.name().to_string();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh_name), device));

        // 스키닝 데이터가 존재하는 경우 스키닝 데이터를 추가합니다.
        if let Some(skinning) = &current.skinning {
            // 메쉬 쉐이더 리소스의 스키닝 데이터를 초기화합니다.
            let data = SkinningDataLayout {
                quality: skinning.quality,
                num_bones: skinning.num_bones,
                ..Default::default()
            };
            mesh_resource.skinning_uniform.update(device, queue, data);

            // 메쉬 쉐이더 리소스의 바인드 포즈 데이터를 초기화합니다.
            let data = skinning.bindposes.clone();
            mesh_resource.bindpose_uniform.update(device, queue, data);

            // 스키닝된 메쉬를 구성하는 뼈 엔터티 집합을 생성합니다.
            let collection = BoneCollection {
                root: entities
                    .get(&skinning.root_bone)
                    .cloned()
                    .expect("no such entity"),
                bones: skinning
                    .bones
                    .iter()
                    .map(|name| entities.get(name).cloned().expect("no such entity"))
                    .collect(),
            };

            // 뼈 엔터티 집합 컴포넌트를 추가합니다.
            builder.add(collection);
        }

        // 메쉬, 메쉬 쉐이더 리소스, 캐릭터 종류 컴포넌트를 추가합니다.
        builder.add_bundle((mesh, mesh_resource, CharacterKind::ArisOriginal));

        // 메쉬 집합에 현제 엔터티를 추가합니다.
        meshes.insert(mesh_name, entity);
    }

    // 현제 노드에 재질 데이터가 존재하는 경우 재질 데이터를 추가합니다.
    if !current.materials.is_empty() {
        let materials: Vec<Arc<MaterialResource>> = current.materials.iter().cloned().collect();
        builder.add(materials);
    }

    {
        /// 노드가 애니메이션 믹싱에 사용되는 뼈 집합에 포함되는지 여부를 반환합니다.
        fn contains_set(name: &str) -> bool {
            name == MODEL_BONE_PELVIS
                || name == MODEL_BONE_L_THIGH
                || name == MODEL_BONE_R_THIGH
                || name.contains("skirt")
        }

        // 뼈 집합에 포함되는 경우 엔터티를 추가합니다.
        let node_name = current.name.clone();
        if contains_mixing_bones || contains_set(&node_name) {
            animation_mixing_bones.insert(entity);
        }
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}

/// ArisOriginalHalo 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 캐릭터 헤일로 종류(`CharacterKind`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>)`
///
/// # Panics
/// - 엔터티 목록에서 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn spawn_aris_original_model_halo(
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
    parent: Entity,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let root =
        ModelHierarchyPool::get_or_init(MODEL_HALO_NAME, WORKSPACE, asset_manager, device, queue)?;

    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_aris_original_model_halo_recursive(
        world,
        device,
        queue,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    );

    Ok((entity, batch_commands))
}

/// ArisOriginalHalo 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 캐릭터 헤일로 종류(`CharacterKind`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>)`
///
/// # Panics
/// - 엔터티 목록에서 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn spawn_aris_original_model_halo_recursive(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform.into_mat4()));
    builder.add(WorldTransform::default());

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(child) = current.children.first() {
        // 자식 엔터티를 생성하기 위한 매개변수를 준비합니다.
        let parent = entity;
        let current = child;
        let siblings = &current.children[1..];

        // 자식 엔터티를 생성합니다.
        let entity = spawn_aris_original_model_halo_recursive(
            world,
            device,
            queue,
            batch_commands,
            parent,
            current,
            siblings,
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(entity));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(sibling) = siblings.first() {
        // 형제 엔터티를 생성하기 위한 매개변수를 준비합니다.
        let current = sibling;
        let siblings = &siblings[1..];

        // 형제 엔터티를 생성합니다.
        let entity = spawn_aris_original_model_halo_recursive(
            world,
            device,
            queue,
            batch_commands,
            parent,
            current,
            siblings,
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(entity));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = current.mesh.clone() {
        // 메쉬 쉐이더 리소스를 생성합니다.
        let mesh_name = mesh.name().to_string();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh_name), device));

        // 메쉬, 메쉬 쉐이더 리소스, 캐릭터 종류 컴포넌트를 추가합니다.
        builder.add_bundle((mesh, mesh_resource, CharacterHaloKind::ArisOriginalHalo));
    }

    // 현제 노드에 재질 데이터가 존재하는 경우 재질 데이터를 추가합니다.
    if !current.materials.is_empty() {
        let materials: Vec<Arc<MaterialResource>> = current.materials.iter().cloned().collect();
        builder.add(materials);
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}

//
// # 애니메이션 테이블
// 애니메이션은 `ActionState`와 `MovementState`로 결정된다.
//
// +----------------------+-----------------------+-------------------------+--------------------------+
// |                      | MovementState::Idle   | MovementState::Moving   | MovementState::MoveToEnd |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::Idle    | Idle                  | Moving                  | MoveToEnd                |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::Aiming  | Aim                   | Aim_Move                | Aim                      |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::AimAt   | Idle_To_Aim           | Move_To_Aim_Move        | Idle_To_Aim              |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::AimOff  | Aim_To_Idle           | Aim_Move_To_Move        | Aim_To_Idle              |
// +----------------------+-----------------------+-------------------------+--------------------------+
// | ActionState::Attack  | Attack_Ing            | Attack_Move             | Attack_Ing               |
// +----------------------+-----------------------+-------------------------+--------------------------+
//
// # 애니메이션 목록 & 사용 타이머
// - Idle
// - Moving
// - MoveToEnd
// - Aim
// - AimMove
// - IdleToAim
// - MoveToAimMove
// - AimToIdle
// - AimMoveToMove
// - Attacking
// - AttackMove
//

/// `ArisOriginal` 모델의 `ActionState`와 `ActionStateTimer`를 갱신합니다.
///
/// # Note
/// - 이 함수를 호출하기 전에 사용자 입력에 따른 ActionState를 먼저 갱신해야합니다.
///
pub fn update_action_state_timer(
    asset_manager: &AssetManager,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&Arc<HashMap<String, Motion>>, &mut ActionState, &mut ActionStateTimer, f32);
    const FUNC_TABLE: [Func; 4] = [
        update_action_state_timer_when_idle,
        update_action_state_timer_when_aiming,
        update_action_state_timer_when_aim_at,
        update_action_state_timer_when_aim_off,
    ];

    // 캐릭터 애니메이션 데이터를 가져옵니다.
    let motions = MotionPool::get_or_init(MODEL_NAME, WORKSPACE, asset_manager)
        .expect("no such character motions");

    let i = *action_state as usize;
    FUNC_TABLE[i](&motions, action_state, action_state_timer, elapsed_time_sec);
}

/// `ActionState::Idle`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_idle(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Normal_Idle` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_sec) % length;
}

/// `ActionState::Aiming`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aiming(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Normal_Idle` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_sec) % length;
}

/// `ActionState::AimAt`일 때 `ActionState`와 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aim_at(
    motions: &Arc<HashMap<String, Motion>>,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Normal_Attack_Start` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    action_state_timer.0 = action_state_timer.0 + elapsed_time_sec;

    // `Aris_Original_Normal_Attack_Start` 애니메이션 길이보다 클 경우 `ActionState`를 갱신합니다.
    let diff_t = action_state_timer.0 - length;
    if diff_t >= 0.0 {
        *action_state = ActionState::Aiming;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::AimOff`일 때 `ActionState`와 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aim_off(
    motions: &Arc<HashMap<String, Motion>>,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Normal_Attack_End` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    action_state_timer.0 = action_state_timer.0 + elapsed_time_sec;

    // `Aris_Original_Normal_Attack_End` 애니메이션 길이보다 클 경우 `ActionState`를 갱신합니다.
    let diff_t = action_state_timer.0 - length;
    if diff_t >= 0.0 {
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t;
    }
}

/// `ArisOriginal` 모델의 `MovementState`와 `MovementStateTimer`를 갱신합니다.
///
/// # Note
/// - 이 함수를 호출하기 전에 ActionState를 먼저 갱신해야합니다.
/// - 이 함수를 호출하기 전에 ControllerState에 따른 MovementState 갱신이 필요합니다.
///
pub fn update_movement_state_timer(
    asset_manager: &AssetManager,
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&Arc<HashMap<String, Motion>>, &mut MovementState, &mut MovementStateTimer, f32);
    const FUNC_TABLE: [[Func; 3]; 4] = [
        // `ActionState::Idle`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_moving,
            update_movement_state_timer_when_move_to_end,
        ],
        // `ActionState::Aiming`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
        // `ActionState::AimAt`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
        // `ActionState::AimOff`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
        ],
    ];

    // 캐릭터 애니메이션 데이터를 가져옵니다.
    let motions = MotionPool::get_or_init(MODEL_NAME, WORKSPACE, asset_manager)
        .expect("no such character motions");

    let i = action_state as usize;
    let j = *movement_state as usize;
    FUNC_TABLE[i][j](
        &motions,
        movement_state,
        movement_state_timer,
        elapsed_time_sec,
    );
}

/// `Aris_Original_Normal_Idle` 애니메이션 데이터로 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_idle(
    motions: &Arc<HashMap<String, Motion>>,
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Normal_Idle` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec) % length;
}

/// `Aris_Original_Move_Ing` 애니메이션 데이터로 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_moving(
    motions: &Arc<HashMap<String, Motion>>,
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Move_Ing` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(MOVING_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec) % length;
}

/// `Aris_Original_Move_End_Normal` 애니메이션 데이터로 `MovementState`와 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_move_to_end(
    motions: &Arc<HashMap<String, Motion>>,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Move_End_Normal` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(MOVE_TO_END_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    movement_state_timer.0 = movement_state_timer.0 + elapsed_time_sec;

    // `Aris_Original_Move_End_Normal` 애니메이션 길이보다 클 경우 `MovemenetState`를 갱신합니다.
    let diff_t = movement_state_timer.0 - length;
    if diff_t >= 0.0 {
        *movement_state = MovementState::Idle;
        movement_state_timer.0 = diff_t;
    }
}

/// `Aris_Original_Cafe_Walk` 애니메이션 데이터로 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_walking(
    motions: &Arc<HashMap<String, Motion>>,
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    // `Aris_Original_Cafe_Walk` 애니메이션 길이를 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");
    let length = motion.length;

    // 타이머를 갱신합니다.
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec) % length;
}

/// `Aris_Original` 모델의 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn animate_aris_original(
    asset_manager: &AssetManager,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    movement_state: MovementState,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    type Func = fn(
        &Arc<HashMap<String, Motion>>,
        ActionStateTimer,
        MovementStateTimer,
        &SkinningAnimation,
        &ViewBorrow<&BoneCollection>,
        &mut ViewBorrow<&mut ToParentTrans>,
    );
    const FUNC_TABLE: [[Func; 3]; 4] = [
        // `ActionState::Idle`
        [
            animate_aris_original_when_idle,        // `MovementState::Idle`
            animate_aris_original_when_moving,      // `MovementState::Moving`
            animate_aris_original_when_move_to_end, // `MovementState::MoveToEnd`
        ],
        // `ActionState::Aiming`
        [
            animate_aris_original_when_aim,      // `MovementState::Idle`
            animate_aris_original_when_aim_move, // `MovementState::Moving`
            animate_aris_original_when_aim,      // `MovementState::MoveToEnd`
        ],
        // `ActionState::AimAt`
        [
            animate_aris_original_when_idle_to_aim, // `MovementState::Idle`
            animate_aris_original_when_move_to_aim_move, // `MovementState::Moving`
            animate_aris_original_when_idle_to_aim, // `MovementState::MoveToEnd`
        ],
        // `ActionState::AimOff`
        [
            animate_aris_original_when_aim_to_idle, // `MovementState::Idle`
            animate_aris_original_when_aim_move_to_move, // `MovementState::Moving`
            animate_aris_original_when_aim_to_idle, // `MovementState::MoveToEnd`
        ],
    ];

    // `ArisOriginal` 모델 애니메이션 집합을 가져옵니다.
    let motions = MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager)
        .expect("no such character motion");

    let i = action_state as usize;
    let j = movement_state as usize;
    FUNC_TABLE[i][j](
        &motions,
        action_state_timer,
        movement_state_timer,
        skinning_animation,
        collection_view,
        transform_view,
    );
}

/// `Aris_Original_Normal_Idle` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_idle(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Idle` 애니메이션을 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(movement_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original_Move_Ing` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_moving(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Move_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(MOVING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(movement_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original_Move_End_Normal` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_move_to_end(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Move_End_Normal` 애니메이션을 가져옵니다.
    let motion = motions.get(MOVE_TO_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(movement_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original_Normal_Attack_Start` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_idle_to_aim(
    motions: &Arc<HashMap<String, Motion>>,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(action_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original_Normal_Attack_End` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_aim_to_idle(
    motions: &Arc<HashMap<String, Motion>>,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(action_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original_Normal_Attack_Start`와 `Aris_Original_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_move_to_aim_move(
    motions: &Arc<HashMap<String, Motion>>,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(action_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }

    // `Aris_Original_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(movement_state_timer.0);

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            // 뼈 엔터티가 애니메이션 믹싱 뼈 집합에 포함되는 겨우 로컬 변환 행렬을 선형 보간합니다.
            let bone_entity = bone_collection.bones[bone_index];
            if skinning_animation
                .animation_mixing_bones
                .contains(&bone_entity)
            {
                let local_transform = transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
            }
        }
    }
}

/// `Aris_Original_Normal_Attack_End`와 `Aris_Original_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_aim_move_to_move(
    motions: &Arc<HashMap<String, Motion>>,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(action_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }

    // `Aris_Original_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(movement_state_timer.0);

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            // 뼈 엔터티가 애니메이션 믹싱 뼈 집합에 포함되는 겨우 로컬 변환 행렬을 선형 보간합니다.
            let bone_entity = bone_collection.bones[bone_index];
            if skinning_animation
                .animation_mixing_bones
                .contains(&bone_entity)
            {
                let local_transform = transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
            }
        }
    }
}

/// `Aris_Original_Normal_Attack_Ing` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_aim(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion
        .keyframes
        .first()
        .expect("keyframes must not be empty");

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original_Normal_Attack_Ing`와 `Aris_Original_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_aris_original_when_aim_move(
    motions: &Arc<HashMap<String, Motion>>,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion
        .keyframes
        .first()
        .expect("keyframes must not be empty");

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = transform_view
        .get_mut(skinning_animation.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }

    // `Aris_Original_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let keyframe = motion.linear_sampling(movement_state_timer.0);

    // 키 프레임을 구성하는 스키닝된 메쉬 뼈 노드의 로컬 변환 행렬을 갱신합니다.
    for keyframe_mesh in keyframe.meshes.iter() {
        // 스키닝된 메쉬의 엔터티를 가져옵니다.
        let entity = skinning_animation
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬를 구성하는 뼈 노드의 집합을 가져옵니다.
        let bone_collection = collection_view
            .get(entity)
            .expect("invalid entity or invalid entity component");

        // 뼈 노드의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            // 뼈 엔터티가 애니메이션 믹싱 뼈 집합에 포함되는 겨우 로컬 변환 행렬을 선형 보간합니다.
            let bone_entity = bone_collection.bones[bone_index];
            if skinning_animation
                .animation_mixing_bones
                .contains(&bone_entity)
            {
                let local_transform = transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
            }
        }
    }
}
