use std::{ops::Deref, sync::Arc};

use ahash::{HashMap, HashSet};
use constcat::concat;
use glam::FloatExt;
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, GameInputBits, LatLon, MovementState,
    MovementStateTimer, ViewState, ViewStateTimer, MAX_JUMP_DURATION, NUM_ACTION_STATES,
    NUM_MOVEMENT_STATES, NUM_VIEW_STATES,
};

use crate::{
    asset::{ModelNode, ModelRoot, Motion, MotionPool, TextureDataPool, CHARACTER_URIS},
    component::{
        BoneCollection, BoneTransformUniform, CharacterMaterialDataLayout,
        CharacterMaterialResource, CharacterMaterialUniform, Child, EyeMouthMaterialDataLayout,
        EyeMouthMaterialResource, EyeMouthMaterialUniform, HaloMaterialDataLayout,
        HaloMaterialResource, HaloMaterialUniform, MaterialData, MaterialUniform, MeshResource,
        Parent, Sibling, SkinnedMeshResource, SkinningAnimation, ThirdPersonCamera, ToParentTrans,
        TransformUniform, WorldTransform, ATTACK_END_ANIMATION_SUFFIX, ATTACK_ING_ANIMATION_SUFFIX,
        ATTACK_START_ANIMATION_SUFFIX, CAFE_WALK_ANIMATION_SUFFIX, IDLE_ANIMATION_SUFFIX,
        MODEL_BONE_L_THIGH, MODEL_BONE_ROOT, MODEL_BONE_R_THIGH, MOVE_TO_END_ANIMATION_SUFFIX,
        MOVING_ANIMATION_SUFFIX, NORMAL_CALLSIGN_SUFFIX, RELOAD_ANIMATION_SUFFIX,
        VICTORY_END_SUFFIX, VICTORY_START_SUFFIX, VITAL_DEATH_ANIMATION_SUFFIX,
    },
};

use super::{
    CharacterHaloKind, MODEL_BONE_HEAD, MODEL_BONE_L_CALF, MODEL_BONE_L_FOOT, MODEL_BONE_R_CALF,
    MODEL_BONE_R_FOOT, MODEL_BONE_R_HAND, MODEL_BONE_SPINE, MODEL_BONE_SPINE_1, MODEL_BONE_WEAPON,
};

/// 캐릭터 모델의 Idle 애니메이션 길이입니다.
pub const NORMAL_IDLE_DURATION: f32 = 2.8;
/// 캐릭터 모델의 Moving 애니메이션 길이입니다.
pub const MOVE_ING_DURATION: f32 = 0.667;
/// 캐릭터 모델의 Move_To_End 애니메이션 길이입니다.
pub const MOVE_END_NORMAL_DURATION: f32 = 2.0;
/// 캐릭터 모델의 Cafe_Walk 애니메이션 길이입니다.
pub const CAFE_WALK_DURATION: f32 = 1.267;
/// 캐릭터 모델의 Attack_Start 애니메이션 길이입니다.
pub const NORMAL_ATTACK_START_DURATION: f32 = 0.667;
/// 캐릭터 모델의 Attack_End 애니메이션 길이입니다.
pub const NORMAL_ATTACK_END_DURATION: f32 = 0.667;
/// 캐릭터 모델의 Attack_Ing 애니메이션 길이입니다.
pub const NORMAL_ATTACK_ING_DURATION: f32 = 2.667;
/// 캐릭터 모델의 Vital_Death 애니메이션 길이입니다.
pub const VITAL_DEATH_DURATION: f32 = 1.8;
/// 캐릭터 모델의 Reload 애니메이션 길이입니다.
pub const NORMAL_RELOAD_DURATION: f32 = 2.0;
/// 캐릭터 모델의 Callsign 애니메이션 길이입니다.
pub const NORMAL_CALLSIGN_DURATION: f32 = 1.5;
/// 캐릭터 모델의 *_Victory_Start 애니메이션 길이입니다.
pub const VICTORY_START_DURATION: f32 = 3.0;
/// 캐릭터 모델의 *_Victory_End 애니메이션 길이입니다.
pub const VICTORY_END_DURATION: f32 = 3.2;

/// 캐릭터 모델의 카메라 기본 위치입니다.
pub const CAMERA_IDLE_POSITION: glam::Vec3A = glam::vec3a(0.25, 0.85, 2.0);
/// 캐릭터 모델의 카메라 줌 위치입니다.
pub const CAMERA_ZOOM_POSITION: glam::Vec3A = glam::vec3a(0.25, 0.7, 0.5);
/// 캐릭터 모델의 카메라 기본 Fov-y 라디안 각도입니다.
pub const CAMERA_IDLE_FOV_Y: f32 = 45f32.to_radians();
/// 캐릭터 모델의 카메라 줌 Fov-y 라디안 각도 입니다.
pub const CAMERA_ZOOM_FOV_Y: f32 = 40f32.to_radians();

/// `Bip001_Head`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
const HEAD_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(-0.06608068, 0.6346726, -0.7699505);
/// `Bip001_Spine`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
const SPINE_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(0.34206244, 0.9269642, -0.15404683);
/// `Bip001_Spine1`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 월드 좌표계 X축을 로컬 좌표계로 변환한 벡터입니다.
const SPINE1_W2L_X_NORMAL_ATTACK_ING: glam::Vec3 = glam::vec3(0.10889296, 0.92688817, -0.35919425);
/// `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임에서 `Bip001_L_Hand`에서 `Bip001_Weapon`까지의 변환 행렬입니다.
const WEAPON_OFFSET: glam::Mat4 = glam::mat4(
    glam::vec4(0.8068547, 0.58844215, 0.052159876, 0.0),
    glam::vec4(-0.22505426, 0.38781863, -0.89383775, 0.0),
    glam::vec4(-0.5462008, 0.70945907, 0.44534516, 0.0),
    glam::vec4(-0.26169276, 0.075413704, 0.07279274, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.914507, -0.009351098, -0.4044626, 0.0),
    glam::vec4(0.1610851, 0.9086536, -0.3852281, 0.0),
    glam::vec4(0.3711186, -0.4174466, -0.8294636, 0.0),
    glam::vec4(0.00000007629394, 0.0000001049042, 0.07303865, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9209496, -0.2404988, 0.306615, 0.0),
    glam::vec4(-0.1418219, 0.9397302, 0.3111172, 0.0),
    glam::vec4(-0.3629586, 0.2430385, -0.899552, 0.0),
    glam::vec4(-0.00000009536743, -0.0000001001358, -0.07303863, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.9008837, 0.4340602, -0.00000002980233, 0.0),
    glam::vec4(-0.4340602, 0.9008837, 0.00000001490116, 0.0),
    glam::vec4(0.00000003331644, -0.0000000004882125, 0.9999999, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.00000001907349, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.6638219, 0.7478905, -0.0000001192093, 0.0),
    glam::vec4(-0.7478906, 0.663822, 0.00000002980231, 0.0),
    glam::vec4(0.0000001014226, 0.00000006937208, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.000000009536743, 0.0, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8172209, -0.3939509, -0.4206576, 0.0),
    glam::vec4(0.3484892, 0.9191245, -0.1837534, 0.0),
    glam::vec4(0.4590265, 0.003572434, 0.8884154, 0.0),
    glam::vec4(-0.1460184, 0.000000009536743, 0.0, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Idle` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_IDLE_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.7886113, -0.4748127, 0.3906981, 0.0),
    glam::vec4(0.5236893, 0.8516232, -0.02207781, 0.0),
    glam::vec4(-0.3222447, 0.2220152, 0.9202539, 0.0),
    glam::vec4(-0.1460184, 0.00000001907349, -0.00000001907349, 1.0),
);

/// `Bip001_L_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.9689184, -0.1076179, -0.2227459, 0.0),
    glam::vec4(0.008400703, 0.8855835, -0.4644049, 0.0),
    glam::vec4(0.2472383, -0.4518415, -0.8571539, 0.0),
    glam::vec4(0.00000009536743, 0.00000009536743, 0.07303865, 1.0),
);

/// `Bip001_R_Thigh`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_THIGH_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(-0.8693725, -0.1518831, 0.4702372, 0.0),
    glam::vec4(-0.009138854, 0.9563732, 0.2920055, 0.0),
    glam::vec4(-0.4940729, 0.2495641, -0.8328325, 0.0),
    glam::vec4(-0.0000001144409, -0.0000001049042, -0.07303862, 1.0),
);

/// `Bip001_L_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8196427, 0.5728754, 0.00000005419786, 0.0),
    glam::vec4(-0.5728753, 0.8196425, 0.000000001892902, 0.0),
    glam::vec4(-0.00000004333847, -0.00000003260012, 1.0, 0.0),
    glam::vec4(-0.1590945, -0.000000007152557, 0.0, 1.0),
);

/// `Bip001_R_Calf`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_CALF_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.5968635, 0.8023427, -0.0000001490116, 0.0),
    glam::vec4(-0.8023427, 0.5968635, 0.00000002980232, 0.0),
    glam::vec4(0.0000001128513, 0.0000001017705, 1.0, 0.0),
    glam::vec4(-0.1590945, 0.0, 0.00000001907349, 1.0),
);

/// `Bip001_L_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const L_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.8093521, -0.4241125, -0.4062982, 0.0),
    glam::vec4(0.4180262, 0.9019042, -0.1087341, 0.0),
    glam::vec4(0.4125574, -0.08183914, 0.9072479, 0.0),
    glam::vec4(-0.1460184, 0.000000004768371, 0.00000003814697, 1.0),
);

/// `Bip001_R_Foot`의 `*_Normal_Attack_Ing` 애니메이션 첫 번째 키 프레임 변환 행렬입니다.
const R_FOOT_NORMAL_ATTACKING_IDENTITY: glam::Mat4 = glam::mat4(
    glam::vec4(0.6853706, -0.5776544, 0.4433765, 0.0),
    glam::vec4(0.616642, 0.7842523, 0.06856146, 0.0),
    glam::vec4(-0.3873239, 0.2264145, 0.8937094, 0.0),
    glam::vec4(-0.1460184, 0.00000001192093, -0.00000001907349, 1.0),
);

/// 캐릭터 모델의 이름입니다.
pub const MODEL_NAME: &'static str = "Aris_Original";

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
/// 캐릭터의 VitalDeath 애니메이션 이름입니다.
const VITAL_DEATH_ANIMATION: &'static str = concat!(MODEL_NAME, VITAL_DEATH_ANIMATION_SUFFIX);
/// 캐릭터의 Reload 애니메이션 이름입니다.
const NORMAL_RELOAD_ANIMATION: &'static str = concat!(MODEL_NAME, RELOAD_ANIMATION_SUFFIX);
/// 캐릭터의 Callsign 애니메이션 이름입니다.
const NORMAL_CALLSIGN_ANIMATION: &'static str = concat!(MODEL_NAME, NORMAL_CALLSIGN_SUFFIX);
/// 캐릭터의 *_Victory_Start 애니메이션 이름입니다.
const VICTORY_START_ANIMATION: &'static str = concat!(MODEL_NAME, VICTORY_START_SUFFIX);
/// 캐릭터의 *_Victory_End 애니메이션 이름입니다.
const VICTORY_END_ANIMATION: &'static str = concat!(MODEL_NAME, VICTORY_END_SUFFIX);

/// 캐릭터 모델을 구성하는 엔터티를 생성합니다.
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
/// - 스키닝된 메쉬 쉐이더 리소스(`SkinnedMeshResource`)
/// - 뼈 변환 해열 유니폼 버버(`BoneTransUniform`)
/// - 뼈 엔터티 집합(`BoneCollection`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>)`
/// - 재질 쉐이더 유니폼 버퍼(`Vec<MaterialUniform>`)
/// - 캐릭터 종류(`CharacterKind`)
///
/// # Panics
/// - 엔터티 목록에서 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn spawn_character_model(
    label: Option<&str>,
    texture_data_pool: &TextureDataPool,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    world: &World,
    parent: Entity,
    root: &ModelRoot,
) -> (Entity, SkinningAnimation, Vec<(Entity, EntityBuilder)>) {
    log::debug!("ModelRoot:{:?}", &root);

    let mut meshes = HashMap::default();
    let mut entities = HashMap::default();
    let mut animation_mixing_bones = HashSet::default();
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_character_model_recursive(
        label,
        texture_data_pool,
        device,
        encoder,
        staging_buffers,
        &mut meshes,
        &mut entities,
        false,
        &mut animation_mixing_bones,
        &mut batch_commands,
        world,
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
        head: entities
            .get(MODEL_BONE_HEAD)
            .cloned()
            .expect("no such entity"),
        muzzle: entities.get("fire_01").cloned().expect("no such entity"),
        weapon: entities
            .get(MODEL_BONE_WEAPON)
            .cloned()
            .expect("no such entity"),
        lower_spine: entities
            .get(MODEL_BONE_SPINE)
            .cloned()
            .expect("no such entity"),
        uppper_spine: entities
            .get(MODEL_BONE_SPINE_1)
            .cloned()
            .expect("no such entity"),
        right_hand: entities
            .get(MODEL_BONE_R_HAND)
            .cloned()
            .expect("no such entity"),
        left_thigh: entities
            .get(MODEL_BONE_L_THIGH)
            .cloned()
            .expect("no such entity"),
        right_thigh: entities
            .get(MODEL_BONE_R_THIGH)
            .cloned()
            .expect("no such entity"),
        left_calf: entities
            .get(MODEL_BONE_L_CALF)
            .cloned()
            .expect("no such entity"),
        right_calf: entities
            .get(MODEL_BONE_R_CALF)
            .cloned()
            .expect("no such entity"),
        left_foot: entities
            .get(MODEL_BONE_L_FOOT)
            .cloned()
            .expect("no such entity"),
        right_foot: entities
            .get(MODEL_BONE_R_FOOT)
            .cloned()
            .expect("no such entity"),
        meshes,
        animation_mixing_bones,
    };

    (entity, skinning_animation, batch_commands)
}

/// 캐릭터 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
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
/// - 스키닝된 메쉬 쉐이더 리소스(`SkinnedMeshResource`)
/// - 뼈 변환 해열 유니폼 버버(`BoneTransUniform`)
/// - 뼈 엔터티 집합(`BoneCollection`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>)`
/// - 재질 쉐이더 유니폼 버퍼(`Vec<MaterialUniform>`)
/// - 캐릭터 종류(`CharacterKind`)
///
/// # Panics
/// - 엔터티 목록에서 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn spawn_character_model_recursive(
    label: Option<&str>,
    texture_data_pool: &TextureDataPool,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    meshes: &mut HashMap<String, Entity>,
    entities: &mut HashMap<String, Entity>,
    contains_mixing_bones: bool,
    animation_mixing_bones: &mut HashSet<Entity>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    world: &World,
    parent: Entity,
    node: &ModelNode,
    siblings: &[ModelNode],
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 엔터티 목록에 현재 엔터티를 추가합니다.
    let node_name = node.name.clone();
    entities.insert(node_name, entity);

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add_bundle((
        Parent(parent),
        ToParentTrans(node.transform),
        WorldTransform::default(),
    ));

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(child_node) = node.children.first() {
        /// 노드가 애니메이션 믹싱에 사용되는 뼈 집합에 포함되는지 여부를 반환합니다.
        fn contains_set(name: &str) -> bool {
            name == MODEL_BONE_L_THIGH || name == MODEL_BONE_R_THIGH || name.contains("skirt")
        }

        let contains_mixing_bones = contains_mixing_bones || contains_set(&node.name);
        let child = spawn_character_model_recursive(
            label,
            texture_data_pool,
            device,
            encoder,
            staging_buffers,
            meshes,
            entities,
            contains_mixing_bones,
            animation_mixing_bones,
            batch_commands,
            world,
            entity,
            child_node,
            &node.children[1..],
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(child));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(sibling_node) = siblings.first() {
        let sibling = spawn_character_model_recursive(
            label,
            texture_data_pool,
            device,
            encoder,
            staging_buffers,
            meshes,
            entities,
            contains_mixing_bones,
            animation_mixing_bones,
            batch_commands,
            world,
            parent,
            sibling_node,
            &siblings[1..],
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(sibling));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = node.mesh.clone() {
        if let Some(skinning) = node.skinning.clone() {
            // 스키닝된 메쉬 쉐이더 리소스를 생성합니다.
            let bindpose_uniform = skinning.bindpose_uniform.clone();
            let bone_trans_uniform = BoneTransformUniform::uninit(
                Some(&format!("BoneTransform({})", label.unwrap_or("Unknown"))),
                device,
            );
            let mesh_resource =
                SkinnedMeshResource::new(label, device, &bindpose_uniform, &bone_trans_uniform);

            // 스키닝된 메쉬를 구성하는 뼈 엔터티 집합을 생성합니다.
            let collection = BoneCollection {
                root: entities
                    .get(&skinning.root_bone)
                    .cloned()
                    .expect("no such entity"),
                bones: skinning
                    .bones
                    .iter()
                    .map(|name| {
                        log::debug!("Name:{}", &name);
                        entities.get(name).cloned().expect("no such entity")
                    })
                    .collect(),
            };

            builder.add_bundle((bone_trans_uniform, mesh_resource, collection));
        } else {
            // 메쉬 쉐이더 리소스를 생성합니다.
            let transform_uniform = TransformUniform::uninit(
                Some(&format!("Transform({})", label.unwrap_or("Unknown"))),
                device,
            );
            let mesh_resource = MeshResource::new(label, device, &transform_uniform);
            builder.add_bundle((transform_uniform, mesh_resource));
        }

        // 메쉬 집합에 현제 엔터티를 추가합니다.
        meshes.insert(mesh.uri().into(), entity);

        if mesh.uri().contains("Halo") {
            // 메쉬, 캐릭터 헤일로 종류 컴포넌트를 추가합니다.
            builder.add_bundle((mesh, CharacterHaloKind::ArisOriginalHalo));
        } else {
            // 메쉬, 캐릭터 종류 컴포넌트를 추가합니다.
            builder.add_bundle((mesh, CharacterKind::ArisOriginal));
        }
    }

    // 현제 노드에 재질 데이터가 존재하는 경우 재질 데이터를 추가합니다.
    if !node.materials.is_empty() {
        let (uniforms, materials): (Vec<_>, Vec<_>) = node
            .materials
            .iter()
            .map(|data| {
                match data.deref() {
                    MaterialData::Character(data) => {
                        // 캐릭터 유니폼 버퍼를 생성합니다.
                        let character_uniform = CharacterMaterialUniform::new(
                            Some(&format!("Character({})", label.unwrap_or("Unknown"))),
                            device,
                            CharacterMaterialDataLayout {
                                glossiness: data.glossiness,
                                smoothness: data.smoothness,
                                metallic: data.metallic,
                                ..Default::default()
                            },
                        );

                        // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                        let (main_color_view, main_color_sampler) = texture_data_pool
                            .get(&data.main_color)
                            .expect("the texture data must exist!");

                        // 캐릭터 마스킹 텍스처를 가져옵니다.
                        let (detail_mask_view, detail_mask_sampler) = texture_data_pool
                            .get(&data.detail_mask)
                            .expect("the texture data must exist!");

                        let material_resource = CharacterMaterialResource::new(
                            label,
                            device,
                            &character_uniform,
                            &main_color_view,
                            &main_color_sampler,
                            &detail_mask_view,
                            &detail_mask_sampler,
                        );

                        (
                            MaterialUniform::Character(character_uniform),
                            material_resource,
                        )
                    }
                    MaterialData::CharacterEyeMouth(data) => {
                        // 캐릭터 유니폼 버퍼를 생성합니다.
                        let data_layout = EyeMouthMaterialDataLayout {
                            glossiness: data.glossiness,
                            smoothness: data.smoothness,
                            metallic: data.metallic,
                            index: data.index,
                            ..Default::default()
                        };
                        let character_uniform = EyeMouthMaterialUniform::new(
                            Some(&format!("EyeMouth({})", label.unwrap_or("Unknown"))),
                            device,
                            data_layout,
                        );

                        // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                        let (main_color_view, main_color_sampler) = texture_data_pool
                            .get(&data.main_color)
                            .expect("the texture data must exist!");
                        // 캐릭터 입 텍스처를 가져옵니다.
                        let (eye_mouth_view, eye_mouth_sampler) = texture_data_pool
                            .get(&data.eye_mouth)
                            .expect("the texture data must exist!");

                        let material_resource = EyeMouthMaterialResource::new(
                            label,
                            device,
                            &character_uniform,
                            &main_color_view,
                            &main_color_sampler,
                            &eye_mouth_view,
                            &eye_mouth_sampler,
                        );

                        (
                            MaterialUniform::CharacterEyeMouth {
                                data: data_layout,
                                buffer: character_uniform,
                            },
                            material_resource,
                        )
                    }
                    MaterialData::CharacterHalo(data) => {
                        // 캐릭터 유니폼 버퍼를 생성합니다.
                        let character_uniform = HaloMaterialUniform::new(
                            Some(&format!("CharacterHalo({})", label.unwrap_or("Unknown"))),
                            device,
                            HaloMaterialDataLayout {
                                glossiness: data.glossiness,
                                smoothness: data.smoothness,
                                metallic: data.metallic,
                                emissive: data.emissive.into(),
                                ..Default::default()
                            },
                        );

                        // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                        let (main_color_view, main_color_sampler) = texture_data_pool
                            .get(&data.main_color)
                            .expect("the texture data must exist!");

                        let material_resource = HaloMaterialResource::new(
                            label,
                            device,
                            &character_uniform,
                            &main_color_view,
                            &main_color_sampler,
                        );

                        (
                            MaterialUniform::CharacterHalo(character_uniform),
                            material_resource,
                        )
                    }
                    _ => panic!("invalid material data!"),
                }
            })
            .unzip();

        builder.add_bundle((uniforms, materials));
    }

    {
        /// 노드가 애니메이션 믹싱에 사용되는 뼈 집합에 포함되는지 여부를 반환합니다.
        fn contains_set(name: &str) -> bool {
            name == MODEL_BONE_L_THIGH || name == MODEL_BONE_R_THIGH || name.contains("skirt")
        }

        // 뼈 집합에 포함되는 경우 엔터티를 추가합니다.
        if contains_mixing_bones || contains_set(&node.name) {
            animation_mixing_bones.insert(entity);
        }
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}

/// 캐릭터 모델의 `ViewState`를 갱신합니다.
pub fn update_character_view_state(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: GameInputBits,
) {
    type Func = fn(&mut ViewState, &mut ViewStateTimer, GameInputBits);
    const FUNC_TABLE: [Func; NUM_VIEW_STATES] = [
        update_view_state_when_idle,
        update_view_state_when_zoom_in,
        update_view_state_when_zoom_out,
        update_view_state_when_aiming,
    ];

    let i = *view_state as usize;
    FUNC_TABLE[i](view_state, view_state_timer, controller_input_flags);
}

/// `ViewState::Idle`일 때, 캐릭터 모델의 `ViewState`를 갱신합니다.
fn update_view_state_when_idle(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: GameInputBits,
) {
    if controller_input_flags.contains(GameInputBits::Aiming) {
        *view_state = ViewState::ZoomIn;
        view_state_timer.reset();
    }
}

/// `ViewState::ZoomIn`일 때, 캐릭터 모델의 `ViewState`를 갱신합니다.
fn update_view_state_when_zoom_in(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: GameInputBits,
) {
    if !controller_input_flags.contains(GameInputBits::Aiming) {
        *view_state = ViewState::ZoomOut;
        view_state_timer.0 =
            (1.0 - view_state_timer.0 / NORMAL_ATTACK_START_DURATION) * NORMAL_ATTACK_END_DURATION;
    }
}

/// `ViewState::ZoomOut`일 때, 캐릭터 모델의 `ViewState`를 갱신합니다.
fn update_view_state_when_zoom_out(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: GameInputBits,
) {
    if controller_input_flags.contains(GameInputBits::Aiming) {
        *view_state = ViewState::ZoomIn;
        view_state_timer.0 =
            (1.0 - view_state_timer.0 / NORMAL_ATTACK_END_DURATION) * NORMAL_ATTACK_START_DURATION;
    }
}

/// `ViewState::Aiming`일 때, 캐릭터 모델의 `ViewState`를 갱신합니다.
fn update_view_state_when_aiming(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: GameInputBits,
) {
    if !controller_input_flags.contains(GameInputBits::Aiming) {
        *view_state = ViewState::ZoomOut;
        view_state_timer.reset();
    }
}

/// 캐릭터 모델의 `ViewStateTimer`를 갱신하는 함수입니다.
pub fn update_character_view_state_timer(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    elapsed_time_sec: f32,
) {
    const FUNC_TABLE: [fn(&mut ViewState, &mut ViewStateTimer, f32); NUM_VIEW_STATES] = [
        update_timer_when_idle_state,
        update_timer_when_zoom_in_state,
        update_timer_when_zoom_out_state,
        update_timer_when_aiming_state,
    ];

    let i = *view_state as usize;
    FUNC_TABLE[i](view_state, view_state_timer, elapsed_time_sec);
}

/// `ViewState::Idle`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_idle_state(_: &mut ViewState, _: &mut ViewStateTimer, _: f32) {
    /* empty */
}

/// `ViewState::ZoomIn`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_zoom_in_state(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    elapsed_time_sec: f32,
) {
    view_state_timer.0 = view_state_timer.0 + elapsed_time_sec;
    if view_state_timer.0 >= NORMAL_ATTACK_START_DURATION {
        *view_state = ViewState::Aiming;
        view_state_timer.reset();
    }
}

/// `ViewState::ZoomOut`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_zoom_out_state(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    elapsed_time_sec: f32,
) {
    view_state_timer.0 = view_state_timer.0 + elapsed_time_sec;
    if view_state_timer.0 >= NORMAL_ATTACK_END_DURATION {
        *view_state = ViewState::Idle;
        view_state_timer.reset();
    }
}

/// `ViewState::Aiming`일 때 `ViewStateTimer`를 갱신하는 함수입니다.
fn update_timer_when_aiming_state(_: &mut ViewState, _: &mut ViewStateTimer, _: f32) {
    /* empty */
}

/// 캐릭터 모델의 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn animate_character(
    motion_pool: &MotionPool,
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
        &Arc<HashMap<String, Motion>>,
        LatLon,
        ActionStateTimer,
        MovementStateTimer,
        &SkinningAnimation,
        &ViewBorrow<&BoneCollection>,
        &mut ViewBorrow<&mut ToParentTrans>,
    );
    const FUNC_TABLE: [[Func; NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
        // `ActionState::Idle`
        [
            animate_character_when_idle,             // `MovementState::Idle`
            animate_character_when_moving,           // `MovementState::Moving`
            animate_character_when_move_to_end,      // `MovementState::MoveToEnd`
            animate_character_when_in_place_jumping, // `MovementState::InPlaceJumping`
            animate_character_when_in_place_landing, // `MovementState::InPlaceLanding`
            animate_character_when_moving_jumping,   // `MovementState::MovingJumping`
            animate_character_when_moving_landing,   // `MovementState::MovingLanding`
        ],
        // `ActionState::Aiming`
        [
            animate_character_when_aim,              // `MovementState::Idle`
            animate_character_when_aim_move,         // `MovementState::Moving`
            animate_character_when_aim,              // `MovementState::MoveToEnd`
            animate_character_when_aim_jumping,      // `MovementState::InPlaceJumping`
            animate_character_when_aim_landing,      // `MovementState::InPlaceLanding`
            animate_character_when_aim_move_jumping, // `MovementState::MovingJumping`
            animate_character_when_aim_move_landing, // `MovementState::MovingLanding`
        ],
        // `ActionState::AimAt`
        [
            animate_character_when_idle_to_aim,      // `MovementState::Idle`
            animate_character_when_move_to_aim_move, // `MovementState::Moving`
            animate_character_when_idle_to_aim,      // `MovementState::MoveToEnd`
            animate_character_when_idle_to_aim_jumping, // `MovementState::InPlaceJumping`
            animate_character_when_idle_to_aim_landing, // `MovementState::InPlaceLanding`
            animate_character_when_move_to_aim_move_jumping, // `MovementState::MovingJumping`
            animate_character_when_move_to_aim_move_landing, // `MovementState::MovingLanding`
        ],
        // `ActionState::AimOff`
        [
            animate_character_when_aim_to_idle,      // `MovementState::Idle`
            animate_character_when_aim_move_to_move, // `MovementState::Moving`
            animate_character_when_aim_to_idle,      // `MovementState::MoveToEnd`
            animate_character_when_aim_to_idle_jumping, // `MovementState::InPlaceJumping`
            animate_character_when_aim_to_idle_landing, // `MovementState::InPlaceLanding`
            animate_character_when_aim_move_to_move_jumping, // `MovementState::MovingJumping`
            animate_character_when_aim_move_to_move_landing, // `MovementState::MovingLanding`
        ],
        // `ActionState::Attack`
        [
            animate_character_when_attacking,           // `MovementState::Idle`
            animate_character_when_attack_move,         // `MovementState::Moving`
            animate_character_when_attacking,           // `MovementState::MoveToEnd`
            animate_character_when_attack_jumping,      // `MovementState::InPlaceJumping`
            animate_character_when_attack_landing,      // `MovementState::InPlaceLanding`
            animate_character_when_attack_move_jumping, // `MovementState::MovingJumping`
            animate_character_when_attack_move_landing, // `MovementState::MovingLanding`
        ],
        // `ActionState::Dead`
        [
            animate_character_when_dead, // `MovementState::Idle`
            animate_character_when_dead, // `MovementState::Moving`
            animate_character_when_dead, // `MovementState::MoveToEnd`
            animate_character_when_dead, // `MovementState::InPlaceJumping`
            animate_character_when_dead, // `MovementState::InPlaceLanding`
            animate_character_when_dead, // `MovementState::MovingJumping`
            animate_character_when_dead, // `MovementState::MovingLanding`
        ],
        // `ActionState::Reload`
        [
            animate_character_when_reload,              // `MovementState::Idle
            animate_character_when_reload_move,         // `MovementState::Moving
            animate_character_when_reload,              // `MovementState::MoveToEnd
            animate_character_when_reload_jumping,      // `MovementState::InPlaceJumping
            animate_character_when_reload_landing,      // `MovementState::InPlaceLanding
            animate_character_when_reload_move_jumping, // `MovementState::MovingJumping
            animate_character_when_reload_move_landing, // `MovementState::MovingLanding
        ],
        // `ActionState::Skill`
        [
            animate_character_when_reload,              // `MovementState::Idle
            animate_character_when_reload_move,         // `MovementState::Moving
            animate_character_when_reload,              // `MovementState::MoveToEnd
            animate_character_when_reload_jumping,      // `MovementState::InPlaceJumping
            animate_character_when_reload_landing,      // `MovementState::InPlaceLanding
            animate_character_when_reload_move_jumping, // `MovementState::MovingJumping
            animate_character_when_reload_move_landing, // `MovementState::MovingLanding
        ],
        // `ActionState::ExSkill`
        [
            animate_character_when_reload,              // `MovementState::Idle
            animate_character_when_reload_move,         // `MovementState::Moving
            animate_character_when_reload,              // `MovementState::MoveToEnd
            animate_character_when_reload_jumping,      // `MovementState::InPlaceJumping
            animate_character_when_reload_landing,      // `MovementState::InPlaceLanding
            animate_character_when_reload_move_jumping, // `MovementState::MovingJumping
            animate_character_when_reload_move_landing, // `MovementState::MovingLanding
        ],
        // `AcstionState::Callsign`
        [
            animate_character_when_callsign, // `MovementState::Idle
            animate_character_when_callsign, // `MovementState::Moving
            animate_character_when_callsign, // `MovementState::MoveToEnd
            animate_character_when_callsign, // `MovementState::InPlaceJumping
            animate_character_when_callsign, // `MovementState::InPlaceLanding
            animate_character_when_callsign, // `MovementState::MovingJumping
            animate_character_when_callsign, // `MovementState::MovingLanding
        ],
        // `AcstionState::VictoryStart`
        [
            animate_character_when_victory_start, // `MovementState::Idle
            animate_character_when_victory_start, // `MovementState::Moving
            animate_character_when_victory_start, // `MovementState::MoveToEnd
            animate_character_when_victory_start, // `MovementState::InPlaceJumping
            animate_character_when_victory_start, // `MovementState::InPlaceLanding
            animate_character_when_victory_start, // `MovementState::MovingJumping
            animate_character_when_victory_start, // `MovementState::MovingLanding
        ],
        // `AcstionState::VictoryEnd`
        [
            animate_character_when_victory_end, // `MovementState::Idle
            animate_character_when_victory_end, // `MovementState::Moving
            animate_character_when_victory_end, // `MovementState::MoveToEnd
            animate_character_when_victory_end, // `MovementState::InPlaceJumping
            animate_character_when_victory_end, // `MovementState::InPlaceLanding
            animate_character_when_victory_end, // `MovementState::MovingJumping
            animate_character_when_victory_end, // `MovementState::MovingLanding
        ],
    ];

    // 캐릭터 모델 애니메이션 집합을 가져옵니다.
    let i = CharacterKind::ArisOriginal as usize;
    let motions = motion_pool
        .get(CHARACTER_URIS[i])
        .expect("no such character motion");

    let i = action_state as usize;
    let j = movement_state as usize;
    FUNC_TABLE[i][j](
        &motions,
        view_rotation,
        action_state_timer,
        movement_state_timer,
        skinning_animation,
        collection_view,
        transform_view,
    );
}

/// `*_Normal_Idle` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_idle(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Idle` 애니메이션을 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0 % NORMAL_IDLE_DURATION;
    let keyframe = motion.linear_sampling(s);

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

/// `*_Move_Ing` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_moving(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Move_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(MOVING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = movement_state_timer.0 % MOVE_ING_DURATION;
    let keyframe = motion.linear_sampling(s);

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

/// `*_Move_End_Normal` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_move_to_end(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Move_End_Normal` 애니메이션을 가져옵니다.
    let motion = motions.get(MOVE_TO_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = movement_state_timer.0.min(MOVE_END_NORMAL_DURATION);
    let keyframe = motion.linear_sampling(s);

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

/// `*_Normal_Attack_Start` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_idle_to_aim(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_START_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = s / NORMAL_ATTACK_START_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_End` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_to_idle(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_END_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0 - s / NORMAL_ATTACK_END_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_Start`와 `*_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_move_to_aim_move(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_START_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // `*_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let t = movement_state_timer.0 % CAFE_WALK_DURATION;
    let keyframe = motion.linear_sampling(t);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = s / NORMAL_ATTACK_START_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_End`와 `*_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_move_to_move(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_END_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // `*_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let t = movement_state_timer.0 % CAFE_WALK_DURATION;
    let keyframe = motion.linear_sampling(t);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0 - s / NORMAL_ATTACK_END_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_Ing` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_Ing`와 `*_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_move(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
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

    // `*_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = movement_state_timer.0 % CAFE_WALK_DURATION;
    let keyframe = motion.linear_sampling(s);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_Ing` 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_attacking(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_ING_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `*_Normal_Attack_Ing`와 `*_Cafe_Walk`가 믹싱된 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_attack_move(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_ING_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // `*_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = movement_state_timer.0 % CAFE_WALK_DURATION;
    let keyframe = motion.linear_sampling(s);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceJumping`이고, `ActionState::Idle`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_in_place_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Idle` 애니메이션을 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0 % NORMAL_IDLE_DURATION;
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    in_place_jumping_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::InPlaceLanding`이고, `ActionState::Idle`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_in_place_landing(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Idle` 애니메이션을 가져옵니다.
    let motion = motions.get(IDLE_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0 % NORMAL_IDLE_DURATION;
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    in_place_landing_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::MovingJumping`이고, `ActionState::Idle`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_moving_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 첫 번째 애니메이션 키 프레임을 가져옵니다.
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

    // 점프 애니메이션을 적용합니다.
    moving_jumping_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::MovingLanding`이고, `ActionState::Idle`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_moving_landing(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 첫 번째 애니메이션 키 프레임을 가져옵니다.
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

    // 착지 애니메이션을 적용합니다.
    moving_landing_anime(skinning_animation, transform_view);
}

/// `MovementState::InPlaceJumping`이고, `ActionState::Aim`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
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

    // 점프 애니메이션을 적용합니다.
    in_place_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceLanding`이고, `ActionState::Aim`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
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

    // 착지 애니메이션을 적용합니다.
    in_place_landing_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingJumping`이고, `ActionState::Aim`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_move_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
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

    // 점프 애니메이션을 적용합니다.
    moving_jumping_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::MovingLanding`이고, `ActionState::Aim`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_move_landing(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    _action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
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

    // 착지 애니메이션을 적용합니다.
    moving_landing_anime(skinning_animation, transform_view);
}

/// `MovementState::InPlaceJumping`이고, `ActionState::AimAt`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_idle_to_aim_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_START_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    in_place_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = s / NORMAL_ATTACK_START_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceLanding`이고, `ActionState::AimAt`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_idle_to_aim_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_START_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    in_place_landing_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = s / NORMAL_ATTACK_START_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingJumping`이고, `ActionState::AimAt`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_move_to_aim_move_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_START_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    moving_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = s / NORMAL_ATTACK_START_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingLanding`이고, `ActionState::AimAt`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_move_to_aim_move_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_START_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_START_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    moving_landing_anime(skinning_animation, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = s / NORMAL_ATTACK_START_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceJumping`이고, `ActionState::AimOff`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_to_idle_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_END_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    in_place_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0 - s / NORMAL_ATTACK_END_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceLanding`이고, `ActionState::AimOff`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_to_idle_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_END_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    in_place_landing_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0 - s / NORMAL_ATTACK_END_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingJumping`이고, `ActionState::AimOff`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_move_to_move_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_END_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    moving_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0 - s / NORMAL_ATTACK_END_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingLanding`이고, `ActionState::AimOff`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_aim_move_to_move_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_End` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_END_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    moving_landing_anime(skinning_animation, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0 - s / NORMAL_ATTACK_END_DURATION;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceJumping`이고, `ActionState::Attack`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_attack_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_ING_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    in_place_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceLanding`이고, `ActionState::Attack`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_attack_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_ING_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    in_place_landing_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingJumping`이고, `ActionState::Attack`일 때 점프 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_attack_move_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_ING_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    moving_jumping_anime(skinning_animation, movement_state_timer, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::MovingLanding`이고, `ActionState::Attack`일 때 착지 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_attack_move_landing(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Attack_Ing` 애니메이션을 가져옵니다.
    let motion = motions.get(ATTACK_ING_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_ATTACK_ING_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    moving_landing_anime(skinning_animation, transform_view);

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceJumping`일 때 점프 애니메이션을 적용합니다.
fn in_place_jumping_anime(
    skinning_animation: &SkinningAnimation,
    movement_state_timer: MovementStateTimer,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    let t = movement_state_timer.0.min(MAX_JUMP_DURATION) / MAX_JUMP_DURATION;

    let angle = IN_PLACE_JUMPING_THIGH_ANGLE * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_IDLE_IDENTITY * rotate;

    let entity = skinning_animation.right_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_IDLE_IDENTITY * rotate;

    let angle = IN_PLACE_JUMPING_CALF_ANGLE * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_IDLE_IDENTITY * rotate;

    let entity = skinning_animation.right_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_IDLE_IDENTITY * rotate;

    let entity = skinning_animation.left_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_IDLE_IDENTITY;

    let entity = skinning_animation.right_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_FOOT_NORMAL_IDLE_IDENTITY;
}

const IN_PLACE_JUMPING_THIGH_ANGLE: f32 = 25f32.to_radians();
const IN_PLACE_JUMPING_CALF_ANGLE: f32 = 60f32.to_radians();

/// `MovementState::InPlaceJumping`일 때 착지 애니메이션을 적용합니다.
fn in_place_landing_anime(
    skinning_animation: &SkinningAnimation,
    movement_state_timer: MovementStateTimer,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    let t = 1.0 - movement_state_timer.0.min(MAX_JUMP_DURATION) / MAX_JUMP_DURATION;

    let angle = IN_PLACE_JUMPING_THIGH_ANGLE * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_IDLE_IDENTITY * rotate;

    let entity = skinning_animation.right_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_IDLE_IDENTITY * rotate;

    let angle = IN_PLACE_JUMPING_CALF_ANGLE * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_IDLE_IDENTITY * rotate;

    let entity = skinning_animation.right_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_IDLE_IDENTITY * rotate;

    let entity = skinning_animation.left_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_IDLE_IDENTITY;

    let entity = skinning_animation.right_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_FOOT_NORMAL_IDLE_IDENTITY;
}

/// `MovementState::MovingJumping`일 때 점프 애니메이션을 적용합니다.
fn moving_jumping_anime(
    skinning_animation: &SkinningAnimation,
    movement_state_timer: MovementStateTimer,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    let t = movement_state_timer.0.min(MAX_JUMP_DURATION) / MAX_JUMP_DURATION;

    let angle = -25f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let angle = 10f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_ATTACKING_IDENTITY;

    let angle = 60f32.to_radians() * t;
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_ATTACKING_IDENTITY;

    let entity = skinning_animation.right_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_FOOT_NORMAL_ATTACKING_IDENTITY;
}

/// `MovementState::MovingLanding`일 때 점프 애니메이션을 적용합니다.
fn moving_landing_anime(
    skinning_animation: &SkinningAnimation,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    let angle = -25f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.left_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let angle = 10f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_thigh;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_THIGH_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_CALF_NORMAL_ATTACKING_IDENTITY;

    let angle = 60f32.to_radians();
    let rotate = glam::Mat4::from_rotation_z(angle);
    let entity = skinning_animation.right_calf;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_CALF_NORMAL_ATTACKING_IDENTITY * rotate;

    let entity = skinning_animation.left_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = L_FOOT_NORMAL_ATTACKING_IDENTITY;

    let entity = skinning_animation.right_foot;
    let local_transform = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = R_FOOT_NORMAL_ATTACKING_IDENTITY;
}

/// `ActionState::Death`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_dead(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Vital_Death` 애니메이션을 가져옵니다.
    let motion = motions.get(VITAL_DEATH_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(VITAL_DEATH_DURATION);
    let keyframe = motion.linear_sampling(s);

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

/// `ActionState::Reload`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_reload(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Reload` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_RELOAD_DURATION);
    let keyframe = motion.linear_sampling(s);

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

/// `MovementState::Moving`이고, `ActionState::Reload`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_reload_move(
    motions: &Arc<HashMap<String, Motion>>,
    view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Reload` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_RELOAD_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // `*_Cafe_Walk` 애니메이션을 가져옵니다.
    let motion = motions.get(CAFE_WALK_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = movement_state_timer.0 % CAFE_WALK_DURATION;
    let keyframe = motion.linear_sampling(s);

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

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let offset = 1.0;
    look_to_camera_direction(offset, skinning_animation, view_rotation, transform_view);
}

/// `MovementState::InPlaceJumping`이고, `ActionState::Reload`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_reload_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Reload` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_RELOAD_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    in_place_jumping_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::InPlaceLanding`이고, `ActionState::Reload`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_reload_landing(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Reload` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_RELOAD_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    in_place_landing_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::MovingJumping`이고, `ActionState::Reload`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_reload_move_jumping(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Reload` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_RELOAD_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 점프 애니메이션을 적용합니다.
    moving_jumping_anime(skinning_animation, movement_state_timer, transform_view);
}

/// `MovementState::MovingLanding`이고, `ActionState::Reload`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_reload_move_landing(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Reload` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_RELOAD_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0.min(NORMAL_RELOAD_DURATION);
    let keyframe = motion.linear_sampling(s);

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

    // 착지 애니메이션을 적용합니다.
    moving_landing_anime(skinning_animation, transform_view);
}

/// `ActionState::Callsign`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_callsign(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Normal_Callsign` 애니메이션을 가져옵니다.
    let motion = motions
        .get(NORMAL_CALLSIGN_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0 % NORMAL_CALLSIGN_DURATION;
    let keyframe = motion.linear_sampling(s);

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

/// `ActionState::VictoryStart`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_victory_start(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Victory_Start` 애니메이션을 가져옵니다.
    let motion = motions
        .get(VICTORY_START_ANIMATION)
        .expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0 % VICTORY_START_DURATION;
    let keyframe = motion.linear_sampling(s);

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

/// `ActionState::VictoryEnd`일 때 애니메이션을 재생합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 애니메이션 타이머를 먼저 갱신해야합니다.
///
/// # Panics
/// - 스키닝 애니메이션을 구성하는 엔터티는 유효애햐 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn animate_character_when_victory_end(
    motions: &Arc<HashMap<String, Motion>>,
    _view_rotation: LatLon,
    action_state_timer: ActionStateTimer,
    _movement_state_timer: MovementStateTimer,
    skinning_animation: &SkinningAnimation,
    collection_view: &ViewBorrow<&BoneCollection>,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    // `*_Victory_Start` 애니메이션을 가져옵니다.
    let motion = motions.get(VICTORY_END_ANIMATION).expect("no such motion");

    // 애니메이션 키 프레임을 샘플링합니다.
    let s = action_state_timer.0 % VICTORY_START_DURATION;
    let keyframe = motion.linear_sampling(s);

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

/// 캐릭터가 카메라가 바라보는 방향을 바라보도록 로컬 변환 행렬을 수정합니다.
fn look_to_camera_direction(
    offset: f32,
    skinning_animation: &SkinningAnimation,
    view_rotation: LatLon,
    transform_view: &mut ViewBorrow<&mut ToParentTrans>,
) {
    let latitude = view_rotation.lat + 3f32.to_radians();

    // 카메라가 바라보는 방향을 캐릭터가 바라보도록 합니다.
    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation.lower_spine;
    let local_transform = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 *= glam::Mat4::from_axis_angle(SPINE_W2L_X_NORMAL_ATTACK_ING, angle);

    let angle = 3.0 * latitude / 7.0 * offset;
    let bone_entity = skinning_animation.uppper_spine;
    let local_transform = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 *= glam::Mat4::from_axis_angle(SPINE1_W2L_X_NORMAL_ATTACK_ING, angle);

    let angle = latitude / 7.0 * offset;
    let bone_entity = skinning_animation.head;
    let local_transform = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 *= glam::Mat4::from_axis_angle(HEAD_W2L_X_NORMAL_ATTACK_ING, angle);
}

/// 무기의 위치를 설정합니다.
///
/// # Note
/// 이 함수는 캐릭터의 월드 변환 행렬이 계산된 후 호출해야 합니다.
///
pub fn set_weapon_position(
    action_state: ActionState,
    skinning_animation: &SkinningAnimation,
    child_view: &ViewBorrow<&Child>,
    sibling_view: &ViewBorrow<&Sibling>,
    transform_view: &mut ViewBorrow<(&ToParentTrans, &mut WorldTransform)>,
) {
    if action_state == ActionState::Idle {
        return;
    }

    let bone_entity = skinning_animation.right_hand;
    let (_, transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    let transform = transform.0.clone();

    let weapon_matrix = transform * WEAPON_OFFSET;
    let bone_entity = skinning_animation.weapon;
    let (_, transform) = transform_view
        .get_mut(bone_entity)
        .expect("invalid entity or invalid entity component");
    transform.0 = weapon_matrix;

    if let Some(child_entity) = child_view.get(bone_entity).cloned() {
        set_weapon_position_recursion(
            *child_entity,
            child_view,
            sibling_view,
            transform_view,
            weapon_matrix,
        );
    }
}

/// 무기의 위치를 설정합니다.
///
/// # Note
/// 이 함수는 캐릭터의 월드 변환 행렬이 계산된 후 호출해야 합니다.
///
fn set_weapon_position_recursion(
    entity: Entity,
    child_view: &ViewBorrow<&Child>,
    sibling_view: &ViewBorrow<&Sibling>,
    transform_view: &mut ViewBorrow<(&ToParentTrans, &mut WorldTransform)>,
    parent_transform: glam::Mat4,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 갱신합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        set_weapon_position_recursion(
            *sibling_entity,
            child_view,
            sibling_view,
            transform_view,
            parent_transform,
        );
    }

    // 현재 엔터티의 월드 변환 행렬을 갱신합니다.
    let (local_transform, world_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    world_transform.0 = parent_transform * local_transform.0;

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 갱신합니다.
    let parent_transform = world_transform.0;
    if let Some(child_entity) = child_view.get(entity).cloned() {
        set_weapon_position_recursion(
            *child_entity,
            child_view,
            sibling_view,
            transform_view,
            parent_transform,
        );
    }
}

/// `ViewState::Idle`일 때 캐릭터 모델의 삼인칭 카메라를 갱신합니다.
pub fn update_third_person_camera_when_idle(
    third_person_camera: &mut ThirdPersonCamera,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    /// 카메라 이펙트를 사용하지 않는 경우 사용되는 함수
    fn non_camera_effect(
        third_person_camera: &mut ThirdPersonCamera,
        _: ActionStateTimer,
        _: ViewStateTimer,
        default_position: glam::Vec3A,
        default_fov_y: f32,
    ) {
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y;
    }

    type Func = fn(&mut ThirdPersonCamera, ActionStateTimer, ViewStateTimer, glam::Vec3A, f32);
    const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        apply_camera_effect_when_attack,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
    ];

    let i = action_state as usize;
    FUNC_TABLE[i](
        third_person_camera,
        action_state_timer,
        view_state_timer,
        CAMERA_IDLE_POSITION,
        CAMERA_IDLE_FOV_Y,
    );
}

/// `ViewState::ZoomIn`일 때 캐릭터 모델의 삼인칭 카메라를 갱신합니다.
pub fn update_third_person_camera_when_zoom_in(
    third_person_camera: &mut ThirdPersonCamera,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    // 현재 카메라 위치와 Fov-y를 계산합니다.
    let s = view_state_timer.0 / NORMAL_ATTACK_START_DURATION;
    let position = CAMERA_IDLE_POSITION.lerp(CAMERA_ZOOM_POSITION, s);
    let fov_y = CAMERA_IDLE_FOV_Y.lerp(CAMERA_ZOOM_FOV_Y, s);

    /// 카메라 이펙트를 사용하지 않는 경우 사용되는 함수
    fn non_camera_effect(
        third_person_camera: &mut ThirdPersonCamera,
        _: ActionStateTimer,
        _: ViewStateTimer,
        default_position: glam::Vec3A,
        default_fov_y: f32,
    ) {
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y;
    }

    type Func = fn(&mut ThirdPersonCamera, ActionStateTimer, ViewStateTimer, glam::Vec3A, f32);
    const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        apply_camera_effect_when_attack,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
    ];

    let i = action_state as usize;
    FUNC_TABLE[i](
        third_person_camera,
        action_state_timer,
        view_state_timer,
        position,
        fov_y,
    );
}

/// `ViewState::ZoomOut`일 때 캐릭터 모델의 삼인칭 카메라를 갱신합니다.
pub fn update_third_person_camera_when_zoom_out(
    third_person_camera: &mut ThirdPersonCamera,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    // 현재 카메라 위치와 Fov-y를 계산합니다.
    let s = view_state_timer.0 / NORMAL_ATTACK_END_DURATION;
    let position: glam::Vec3A = CAMERA_ZOOM_POSITION.lerp(CAMERA_IDLE_POSITION, s);
    let fov_y = CAMERA_ZOOM_FOV_Y.lerp(CAMERA_IDLE_FOV_Y, s);

    /// 카메라 이펙트를 사용하지 않는 경우 사용되는 함수
    fn non_camera_effect(
        third_person_camera: &mut ThirdPersonCamera,
        _: ActionStateTimer,
        _: ViewStateTimer,
        default_position: glam::Vec3A,
        default_fov_y: f32,
    ) {
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y;
    }

    type Func = fn(&mut ThirdPersonCamera, ActionStateTimer, ViewStateTimer, glam::Vec3A, f32);
    const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        apply_camera_effect_when_attack,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
    ];

    let i = action_state as usize;
    FUNC_TABLE[i](
        third_person_camera,
        action_state_timer,
        view_state_timer,
        position,
        fov_y,
    );
}

/// `ViewState::Aiming`일 때 캐릭터 모델의 삼인칭 카메라를 갱신합니다.
pub fn update_third_person_camera_when_aiming(
    third_person_camera: &mut ThirdPersonCamera,
    action_state: ActionState,
    action_state_timer: ActionStateTimer,
    view_state_timer: ViewStateTimer,
) {
    /// 카메라 이펙트를 사용하지 않는 경우 사용되는 함수
    fn non_camera_effect(
        third_person_camera: &mut ThirdPersonCamera,
        _: ActionStateTimer,
        _: ViewStateTimer,
        default_position: glam::Vec3A,
        default_fov_y: f32,
    ) {
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y;
    }

    type Func = fn(&mut ThirdPersonCamera, ActionStateTimer, ViewStateTimer, glam::Vec3A, f32);
    const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        apply_camera_effect_when_attack,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
        non_camera_effect,
    ];

    let i = action_state as usize;
    FUNC_TABLE[i](
        third_person_camera,
        action_state_timer,
        view_state_timer,
        CAMERA_ZOOM_POSITION,
        CAMERA_ZOOM_FOV_Y,
    );
}

/// 카메라 이펙트 함수
fn effect_function(t: f32) -> f32 {
    t * t / (t * t + (1.0 - t) * (1.0 - t))
}

/// `ViewState::Idle`에서 일반 공격을 사용할 경우 카메라 이펙트를 적용합니다.
fn apply_camera_effect_when_attack(
    third_person_camera: &mut ThirdPersonCamera,
    action_state_timer: ActionStateTimer,
    _view_state_timer: ViewStateTimer,
    default_position: glam::Vec3A,
    default_fov_y: f32,
) {
    // 총알 발사 타이밍
    const ATTACK_TP_0: f32 = 0.9;
    const ATTACK_TP_1: f32 = 1.0;
    const ATTACK_TP_2: f32 = 1.5;

    // 줌인 오프셋
    const ZOOM_OFFSET: f32 = 0.174533;

    if (ATTACK_TP_0..ATTACK_TP_1).contains(&action_state_timer.0) {
        let t = (action_state_timer.0 - ATTACK_TP_0) / (ATTACK_TP_1 - ATTACK_TP_0);
        let delta = effect_function(t);
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y - ZOOM_OFFSET * delta;
    } else if (ATTACK_TP_1..=ATTACK_TP_2).contains(&action_state_timer.0) {
        let t = (action_state_timer.0 - ATTACK_TP_1) / (ATTACK_TP_2 - ATTACK_TP_1);
        let delta = 1.0 - effect_function(t);
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y - ZOOM_OFFSET * delta;
    } else {
        third_person_camera.position = default_position;
        third_person_camera.fov_y = default_fov_y;
    }
}

/// `ActionStateTimer`를 갱신합니다.
pub fn update_action_state_timer(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&mut ActionState, &mut ActionStateTimer, f32);
    const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
        update_action_state_timer_when_idle,
        update_action_state_timer_when_aiming,
        update_action_state_timer_when_aim_at,
        update_action_state_timer_when_aim_off,
        update_action_state_timer_when_attack,
        update_action_state_timer_when_dead,
        update_action_state_timer_when_reload,
        update_action_state_timer_when_skill,
        update_action_state_timer_when_ex_skill,
        update_action_state_timer_when_callsign,
        update_action_state_timer_when_victory_start,
        update_action_state_timer_when_victory_end,
    ];

    let i = *action_state as usize;
    FUNC_TABLE[i](action_state, action_state_timer, elapsed_time_sec);
}

/// `ActionState::Idle`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_idle(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    let duration = NORMAL_IDLE_DURATION;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_sec) % duration;
}

/// `ActionState::Aiming`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aiming(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    let duration = NORMAL_IDLE_DURATION;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_sec) % duration;
}

/// `ActionState::AimAt`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aim_at(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 += elapsed_time_sec;

    let duration = NORMAL_ATTACK_START_DURATION;
    let diff_t = action_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *action_state = ActionState::Aiming;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::AimOff`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_aim_off(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 += elapsed_time_sec;

    let duration = NORMAL_ATTACK_END_DURATION;
    let diff_t = action_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::Attack`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_attack(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 += elapsed_time_sec;

    let duration = NORMAL_ATTACK_ING_DURATION;
    let diff_t = action_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::AimOff`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_dead(
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_sec).min(VITAL_DEATH_DURATION);
}

/// `ActionState::Reload`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_reload(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 += elapsed_time_sec;

    let duration = NORMAL_RELOAD_DURATION;
    let diff_t = action_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::Skill`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_skill(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    // TODO!
}

/// `ActionState::ExSkill`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_ex_skill(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    // TODO!
}

/// `ActionState::Callsign`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_callsign(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 += elapsed_time_sec;

    let duration = NORMAL_CALLSIGN_DURATION;
    let diff_t = action_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::VictoryStart`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_victory_start(
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    action_state_timer.0 += elapsed_time_sec;

    let duration = VICTORY_START_DURATION;
    let diff_t = action_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *action_state = ActionState::VictoryEnd;
        action_state_timer.0 = diff_t;
    }
}

/// `ActionState::VictoryEnd`일 때 `ActionStateTimer`를 갱신합니다.
fn update_action_state_timer_when_victory_end(
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    elapsed_time_sec: f32,
) {
    let duration = VICTORY_END_DURATION;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_sec) % duration;
}

/// `MovementStateTimer`를 갱신합니다.
pub fn update_movement_state_timer(
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    type Func = fn(&mut MovementState, &mut MovementStateTimer, f32);
    const FUNC_TABLE: [[Func; NUM_MOVEMENT_STATES]; NUM_ACTION_STATES] = [
        // `ActionState::Idle`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_moving,
            update_movement_state_timer_when_move_to_end,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::Aiming`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::AimAt`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::AimOff`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::Attack`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::Dead`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::Reload`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::Skill`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::ExSkill`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_walking,
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::Callsign`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_moving,
            update_movement_state_timer_when_move_to_end,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::VictoryStart`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_moving,
            update_movement_state_timer_when_move_to_end,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
        // `ActionState::VictoryEnd`
        [
            update_movement_state_timer_when_idle,
            update_movement_state_timer_when_moving,
            update_movement_state_timer_when_move_to_end,
            update_movement_state_timer_when_in_place_jumping,
            update_movement_state_timer_when_landing,
            update_movement_state_timer_when_moving_jumping,
            update_movement_state_timer_when_landing,
        ],
    ];

    let i = action_state as usize;
    let j = *movement_state as usize;
    FUNC_TABLE[i][j](movement_state, movement_state_timer, elapsed_time_sec);
}

/// `MovementState::Idle`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_idle(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    let duration = NORMAL_IDLE_DURATION;
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec) % duration;
}

/// `ActionState::Idle`이고, `MovementState::Moving`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_moving(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    let duration = MOVE_ING_DURATION;
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec) % duration;
}

/// `MovementState::MoveToEnd`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_move_to_end(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    movement_state_timer.0 += elapsed_time_sec;

    let duration = MOVE_END_NORMAL_DURATION;
    let diff_t = movement_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *movement_state = MovementState::Idle;
        movement_state_timer.0 = diff_t;
    }
}

/// `ActionState::Aiming`이고, `MovementState::Moving`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_walking(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    let duration = CAFE_WALK_DURATION;
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec) % duration;
}

/// `MovementState::InPlaceJumping`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_in_place_jumping(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    movement_state_timer.0 += elapsed_time_sec;

    let duration = MAX_JUMP_DURATION;
    let diff_t = movement_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *movement_state = MovementState::InPlaceLanding;
        movement_state_timer.0 = diff_t;
    }
}

/// `MovementState::MovingJumping`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_moving_jumping(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    movement_state_timer.0 += elapsed_time_sec;

    let duration = MAX_JUMP_DURATION;
    let diff_t = movement_state_timer.0 - duration;
    if diff_t >= 0.0 {
        *movement_state = MovementState::MovingLanding;
        movement_state_timer.0 = diff_t;
    }
}

/// `MovementState::InPlaceLanding` 또는 `MovementState::MovingLanding`일 때 `MovementStateTimer`를 갱신합니다.
fn update_movement_state_timer_when_landing(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    elapsed_time_sec: f32,
) {
    movement_state_timer.0 = (movement_state_timer.0 + elapsed_time_sec).min(MAX_JUMP_DURATION);
}
