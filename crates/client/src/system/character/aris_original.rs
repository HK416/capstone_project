use std::sync::Arc;

use ahash::{HashMap, HashSet};
use constcat::concat;
use hecs::{Entity, EntityBuilder, NoSuchEntity, ViewBorrow, World};
use mod_app::asset::AssetManager;
use mod_render::{MeshResource, SkinningDataLayout};

use crate::{
    asset::{ModelAssetError, ModelHierarchyPool, MotionPool, Node},
    component::{
        BoneCollection, Character, CharacterHalo, Child, MovementState, MovementStateTimer, Parent,
        Sibling, SkinningAnimation, ToParentTrans, ViewState, ViewStateTimer, WorldTransform,
        ZoomLength,
    },
    system::{
        ATTACK_END_ANIMATION_SUFFIX, ATTACK_ING_ANIMATION_SUFFIX, ATTACK_START_ANIMATION_SUFFIX,
        CAFE_WALK_ANIMATION_SUFFIX, IDLE_ANIMATION_SUFFIX, MOVE_TO_END_ANIMATION_SUFFIX,
        MOVING_ANIMATION_SUFFIX,
    },
};

use super::{MODEL_BONE_L_THIGH, MODEL_BONE_PELVIS, MODEL_BONE_ROOT, MODEL_BONE_R_THIGH};

const IDLE_ANIMATION: &'static str = concat!(MODEL_NAME, IDLE_ANIMATION_SUFFIX);
const MOVING_ANIMATION: &'static str = concat!(MODEL_NAME, MOVING_ANIMATION_SUFFIX);
const MOVE_TO_END_ANIMATION: &'static str = concat!(MODEL_NAME, MOVE_TO_END_ANIMATION_SUFFIX);
const CAFE_WALK_ANIMATION: &'static str = concat!(MODEL_NAME, CAFE_WALK_ANIMATION_SUFFIX);
const ATTACK_START_ANIMATION: &'static str = concat!(MODEL_NAME, ATTACK_START_ANIMATION_SUFFIX);
const ATTACK_ING_ANIMATION: &'static str = concat!(MODEL_NAME, ATTACK_ING_ANIMATION_SUFFIX);
const ATTACK_END_ANIMATION: &'static str = concat!(MODEL_NAME, ATTACK_END_ANIMATION_SUFFIX);

const WORKSPACE: &'static str = "characters/aris_original";
const MODEL_NAME: &'static str = "Aris_Original";
const MODEL_HALO_NAME: &'static str = "Aris_Original_Halo";

/// `aris_original` 캐릭터 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 다음 컴포넌트를 기본으로 가집니다.
/// - `Parent`
/// - `ToParentTrans`
/// - `WorldTransform`
///
/// 생성된 엔터티는 선택적으로 다음 컴포넌트를 가집니다.
/// - `Arc<Mesh>`
/// - `Arc<MeshResource>`
/// - `Arc<BoneCollection>`
/// - `Child`
/// - `Sibling`
/// - `Vec<Arc<MaterialResource>>`
///
pub fn spawn_aris_original_model(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    parent: Entity,
) -> Result<(Entity, Arc<SkinningAnimation>, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let root =
        ModelHierarchyPool::get_or_init(&MODEL_NAME, &WORKSPACE, asset_manager, device, queue)?;

    let mut meshes = HashMap::default();
    let mut entities = HashMap::default();
    let mut lower_nodes = HashSet::default();
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_model_recursive(
        world,
        device,
        queue,
        &mut meshes,
        &mut entities,
        &mut lower_nodes,
        false,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    )
    .map_err(|_| ModelAssetError::NoSuchEntity)?;

    let collection = Arc::new(SkinningAnimation {
        root: entities
            .get(MODEL_BONE_ROOT)
            .cloned()
            .ok_or(ModelAssetError::NoSuchEntity)?,
        meshes,
        lower_nodes,
    });

    Ok((entity, collection, batch_commands))
}

/// `aris_original` 모델을 구성하는 `Entity`를 생성하는 재귀함수입니다.
fn spawn_model_recursive(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    meshes: &mut HashMap<String, Entity>,
    entities: &mut HashMap<String, Entity>,
    lower_nodes: &mut HashSet<Entity>,
    is_lower_node: bool,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Result<Entity, NoSuchEntity> {
    let name = current.name.clone();
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform.into_mat4()));
    builder.add(WorldTransform::default());

    if let Some(child) = current.children.first() {
        let is_lower_node = is_lower_node
            || name == MODEL_BONE_L_THIGH
            || name == MODEL_BONE_R_THIGH
            || name.contains("skirt");
        let entity = spawn_model_recursive(
            world,
            device,
            queue,
            meshes,
            entities,
            lower_nodes,
            is_lower_node,
            batch_commands,
            entity,
            child,
            &current.children[1..],
        )?;
        builder.add(Child(entity));
    }

    if let Some(sibling) = siblings.first() {
        let entity = spawn_model_recursive(
            world,
            device,
            queue,
            meshes,
            entities,
            lower_nodes,
            is_lower_node,
            batch_commands,
            parent,
            sibling,
            &siblings[1..],
        )?;
        builder.add(Sibling(entity));
    }

    if let Some(mesh) = &current.mesh {
        let mesh = mesh.clone();
        let mesh_name = mesh.name().to_string();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh.name()), device));

        if let Some(skinning) = &current.skinning {
            mesh_resource.skinning_uniform.update(
                device,
                queue,
                SkinningDataLayout {
                    quality: skinning.quality,
                    num_bones: skinning.num_bones,
                    ..Default::default()
                },
            );
            mesh_resource
                .bindpose_uniform
                .update(device, queue, skinning.bindposes.clone());

            let root = entities
                .get(&skinning.root_bone)
                .cloned()
                .ok_or(NoSuchEntity)?;
            let mut bones = Vec::with_capacity(skinning.bones.len());
            for name in skinning.bones.iter() {
                bones.push(entities.get(name).cloned().ok_or(NoSuchEntity)?);
            }
            builder.add(Arc::new(BoneCollection { root, bones }));
        }

        builder.add(mesh);
        builder.add(mesh_resource);
        builder.add(Character::ArisOriginal);

        meshes.insert(mesh_name, entity);
    }

    if !current.materials.is_empty() {
        let mut materials = Vec::with_capacity(current.materials.len());
        for resource in current.materials.iter() {
            materials.push(resource.clone());
        }
        builder.add(materials);
    }

    let is_lower_node = is_lower_node
        || name == MODEL_BONE_PELVIS
        || name == MODEL_BONE_L_THIGH
        || name == MODEL_BONE_R_THIGH
        || name.contains("skirt");
    if is_lower_node {
        lower_nodes.insert(entity);
    }
    entities.insert(name, entity);
    batch_commands.push((entity, builder));
    Ok(entity)
}

/// `aris_original_halo` 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 다음 컴포넌트를 기본으로 가집니다.
/// - `Parent`
/// - `ToParentTrans`
/// - `WorldTransform`
///
/// 생성된 엔터티는 다음 컴포넌트를 선택적으로 가집니다.
/// - `Arc<Mesh>`
/// - `Arc<MeshResource>`
/// - `Child`
/// - `Sibling`
/// - `Vec<Arc<MaterialResource>>`
///
pub(super) fn spawn_aris_original_halo_model(
    world: &World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    parent: Entity,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    let root = ModelHierarchyPool::get_or_init(
        &MODEL_HALO_NAME,
        &WORKSPACE,
        asset_manager,
        device,
        queue,
    )?;

    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_model_halo_recursive(
        world,
        device,
        queue,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    )
    .map_err(|_| ModelAssetError::NoSuchEntity)?;

    Ok((entity, batch_commands))
}

/// `aris_original_halo` 모델을 구성하는 `Entity`를 생성하는 재귀함수입니다.
fn spawn_model_halo_recursive(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Result<Entity, NoSuchEntity> {
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform.into_mat4()));
    builder.add(WorldTransform::default());

    if let Some(child) = current.children.first() {
        let entity = spawn_model_halo_recursive(
            world,
            device,
            queue,
            batch_commands,
            entity,
            child,
            &current.children[1..],
        )?;
        builder.add(Child(entity));
    }

    if let Some(sibling) = siblings.first() {
        let entity = spawn_model_halo_recursive(
            world,
            device,
            queue,
            batch_commands,
            parent,
            sibling,
            &siblings[1..],
        )?;
        builder.add(Sibling(entity));
    }

    if let Some(mesh) = &current.mesh {
        let mesh = mesh.clone();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh.name()), device));

        builder.add(mesh);
        builder.add(mesh_resource);
        builder.add(CharacterHalo::ArisOriginalHalo);
    }

    if !current.materials.is_empty() {
        let mut materials = Vec::with_capacity(current.materials.len());
        for resource in current.materials.iter() {
            materials.push(resource.clone());
        }
        builder.add(materials);
    }

    batch_commands.push((entity, builder));
    Ok(entity)
}

/// `Aris_Original` 캐릭터 모델의 줌 인/아웃 길이를 가져옵니다.
pub fn get_aris_original_zoom_length(asset_manager: &AssetManager) -> ZoomLength {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let attack_start_motion = character_motion_set.get(ATTACK_START_ANIMATION).unwrap();
    let attack_end_motion = character_motion_set.get(ATTACK_END_ANIMATION).unwrap();

    ZoomLength {
        in_time: attack_start_motion.length,
        out_time: attack_end_motion.length,
    }
}

/// `Aris Original` 캐릭터 모델의 움직임 상태 타이머를 갱신합니다.
pub fn update_aris_original_movement_state_timer(
    asset_manager: &AssetManager,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    const FUNC_TABLE: [fn(&AssetManager, &mut MovementState, &mut MovementStateTimer, f32); 3] = [
        update_movement_state_timer_when_idle,
        update_movement_state_timer_when_moving_state,
        update_movement_state_timer_when_move_to_end_state,
    ];

    let index = *movement_state as usize;
    FUNC_TABLE[index](
        asset_manager,
        movement_state,
        movement_state_timer,
        elapsed_time_sec,
    );
}

/// `MovementState::Idle`일 때 움직임 상태 타이머를 갱신합니다.
fn update_movement_state_timer_when_idle(
    asset_manager: &AssetManager,
    state: &mut MovementState,
    timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    debug_assert_eq!(*state, MovementState::Idle);

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(IDLE_ANIMATION).unwrap();

    // 타이머를 갱신합니다.
    timer.0 = (timer.0 + elapsed_time_sec) % character_motion.length;
}

/// `MovementState::Moving`일 때 움직임 상태 타이머를 갱신합니다.
fn update_movement_state_timer_when_moving_state(
    asset_manager: &AssetManager,
    state: &mut MovementState,
    timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    debug_assert_eq!(*state, MovementState::Moving);

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(MOVING_ANIMATION).unwrap();

    // 타이머를 갱신합니다.
    timer.0 = (timer.0 + elapsed_time_sec) % character_motion.length;
}

/// `MovementState::MoveToEnd`일 때 움직임 상태 타이머를 갱신합니다.
fn update_movement_state_timer_when_move_to_end_state(
    asset_manager: &AssetManager,
    state: &mut MovementState,
    timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    debug_assert_eq!(*state, MovementState::MoveToEnd);

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(MOVE_TO_END_ANIMATION).unwrap();

    // 타이머를 갱신합니다.
    timer.0 = timer.0 + elapsed_time_sec;
    let diff_t = timer.0 - character_motion.length;
    if diff_t >= 0.0 {
        *state = MovementState::Idle;
        timer.0 = diff_t;
    }
}

/// `Aris_Original` 캐릭터 모델의 애니메이션을 갱신합니다.
pub fn update_aris_original_animation(
    asset_manager: &AssetManager,
    entity: Entity,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    skinning_view: &ViewBorrow<&Arc<SkinningAnimation>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
    state_view: &ViewBorrow<(
        &MovementState,
        &MovementStateTimer,
        &ViewState,
        &ViewStateTimer,
        &ZoomLength,
    )>,
) {
    type Func = for<'a, 'b, 'c, 'd, 'e, 'f, 'g> fn(
        &'a AssetManager,
        MovementStateTimer,
        ViewStateTimer,
        ZoomLength,
        Arc<SkinningAnimation>,
        &'b ViewBorrow<'c, &'d Arc<BoneCollection>>,
        &'e mut ViewBorrow<'f, &'g mut ToParentTrans>,
    );
    // FUNC_TABLE[ViewState][MovementState]
    const FUNC_TABLE: [[Func; 3]; 4] = [
        [
            update_aris_original_animation_when_idle,
            update_aris_original_animation_when_moving,
            update_aris_original_animation_when_move_to_end,
        ],
        [
            update_aris_original_animation_when_idle_to_aim,
            update_aris_original_animation_when_move_to_aim_move,
            update_aris_original_animation_when_idle_to_aim,
        ],
        [
            update_aris_original_animation_when_aim_to_idle,
            update_aris_original_animation_when_aim_move_to_move,
            update_aris_original_animation_when_aim_to_idle,
        ],
        [
            update_aris_original_animation_when_aim,
            update_aris_original_animation_when_aim_move,
            update_aris_original_animation_when_aim,
        ],
    ];

    // 엔터티로 부터 상태 머신과 스키닝 애니메이션 정보를 가져옵니다.
    let (&movement_state, &movement_state_timer, &view_state, &view_state_timer, &length) =
        state_view
            .get(entity)
            .expect("invalid entity or invalid entity component");
    let skinning = skinning_view
        .get(entity)
        .cloned()
        .expect("invalid entity or invalid entity component");

    let i = view_state as usize;
    let j = movement_state as usize;
    FUNC_TABLE[i][j](
        asset_manager,
        movement_state_timer,
        view_state_timer,
        length,
        skinning,
        bones_view,
        local_transform_view,
    );
}

/// `Aris_Original` 캐릭터 모델의 기본 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_idle(
    asset_manager: &AssetManager,
    movement_state_timer: MovementStateTimer,
    _view_state_timer: ViewStateTimer,
    _length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(IDLE_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let keyframe = character_motion.linear_sampling(movement_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original` 캐릭터 모델의 움직이는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_moving(
    asset_manager: &AssetManager,
    movement_state_timer: MovementStateTimer,
    _view_state_timer: ViewStateTimer,
    _length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(MOVING_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let keyframe = character_motion.linear_sampling(movement_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original` 캐릭터 모델의 움직였다 멈추는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_move_to_end(
    asset_manager: &AssetManager,
    movement_state_timer: MovementStateTimer,
    _view_state_timer: ViewStateTimer,
    _length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(MOVE_TO_END_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let keyframe = character_motion.linear_sampling(movement_state_timer.0);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original` 캐릭터 모델이 사격 자세를 취하는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_idle_to_aim(
    asset_manager: &AssetManager,
    _movement_state_timer: MovementStateTimer,
    view_state_timer: ViewStateTimer,
    length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(ATTACK_START_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = view_state_timer.0 / length.in_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original` 캐릭터 모델이 사격 자세를 푸는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_aim_to_idle(
    asset_manager: &AssetManager,
    _movement_state_timer: MovementStateTimer,
    view_state_timer: ViewStateTimer,
    length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(ATTACK_END_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = view_state_timer.0 / length.out_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original` 캐릭터 모델이 이동하며 사격 자세를 취하는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_move_to_aim_move(
    asset_manager: &AssetManager,
    movement_state_timer: MovementStateTimer,
    view_state_timer: ViewStateTimer,
    length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(ATTACK_START_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = view_state_timer.0 / length.in_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion = character_motion_set.get(CAFE_WALK_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = movement_state_timer.0 / length.in_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            if skinning.lower_nodes.contains(&bone_entity) {
                let local_transform = local_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
            }
        }
    }
}

/// `Aris_Original` 캐릭터 모델이 이동하며 사격 자세를 해제하는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_aim_move_to_move(
    asset_manager: &AssetManager,
    movement_state_timer: MovementStateTimer,
    view_state_timer: ViewStateTimer,
    length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(ATTACK_END_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = view_state_timer.0 / length.in_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];

            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion = character_motion_set.get(CAFE_WALK_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = movement_state_timer.0 / length.in_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            if skinning.lower_nodes.contains(&bone_entity) {
                let local_transform = local_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
            }
        }
    }
}

/// `Aris_Original` 캐릭터 모델이 이동하며 사격 자세를 취하는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_aim(
    asset_manager: &AssetManager,
    _movement_state_timer: MovementStateTimer,
    _view_state_timer: ViewStateTimer,
    _length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(ATTACK_ING_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let keyframe = character_motion.keyframes.first().unwrap();

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];

            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }
}

/// `Aris_Original` 캐릭터 모델이 이동하며 사격 자세를 취하며 이동하는 애니메이션을 갱신합니다.
fn update_aris_original_animation_when_aim_move(
    asset_manager: &AssetManager,
    movement_state_timer: MovementStateTimer,
    _view_state_timer: ViewStateTimer,
    length: ZoomLength,
    skinning: Arc<SkinningAnimation>,
    bones_view: &ViewBorrow<&Arc<BoneCollection>>,
    local_transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion_set =
        MotionPool::get_or_init(MODEL_NAME, &WORKSPACE, asset_manager).unwrap();
    let character_motion = character_motion_set.get(ATTACK_ING_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let keyframe = character_motion.keyframes.first().unwrap();

    // 최상위 뼈 변환 행렬의 로컬 변환 행렬을 갱신합니다.
    let local_transform = local_transform_view
        .get_mut(skinning.root)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = keyframe.root_matrix;

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];

            let local_transform = local_transform_view
                .get_mut(bone_entity)
                .expect("invalid entity or invalid entity component");
            local_transform.0 = bone_transform;
        }
    }

    // `Aris_Original` 캐릭터 모델의 애니메이션을 가져옵니다.
    let character_motion = character_motion_set.get(MOVING_ANIMATION).unwrap();

    // 애니메이션 타이머에 맞는 키 프레임을 샘플링합니다
    let time_point = movement_state_timer.0 / length.in_time;
    let time_point = time_point * character_motion.length;
    let keyframe = character_motion.linear_sampling(time_point);

    for keyframe_mesh in keyframe.meshes.iter() {
        // 키 프레임 메쉬의 엔터티를 가져옵니다.
        let entity = skinning
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");

        // 스키닝된 메쉬의 뼈 집합을 가져옵니다.
        let bone_collection = bones_view
            .get(entity)
            .cloned()
            .expect("invaild entity or invalid entity component");

        // 뼈 집합의 로컬 변환 행렬을 갱신합니다.
        for (bone_index, bone_transform) in keyframe_mesh.bone_trans.iter().cloned().enumerate() {
            let bone_entity = bone_collection.bones[bone_index];
            if skinning.lower_nodes.contains(&bone_entity) {
                let local_transform = local_transform_view
                    .get_mut(bone_entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.0 = local_transform.0 * 0.2 + bone_transform * 0.8;
            }
        }
    }
}
