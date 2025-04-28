mod animation;
mod aris_original;
mod midori_original;
mod momoi_original;
mod pipeline;
mod yuuka_original;

use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, GameInputBits, LatLon, MovementState,
    MovementStateTimer, PlayPhasePlayer, ViewState, ViewStateTimer, NUM_ACTION_STATES,
    NUM_MOVEMENT_STATES,
};

use crate::{
    asset::{ModelPool, ModelRoot, MotionPool, TextureDataPool, CHARACTER_URIS},
    component::{Child, Sibling, ToParentTrans, WorldTransform},
};

pub use self::{animation::*, pipeline::*};

use super::{MoveDirection, ThirdPersonCamera};

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

/// 플레이어 캐릭터를 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다
/// - 자식 엔터티(`Child`)
/// - 캐릭터 종류(`CharacterKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - 스키닝 애니메이션(`SkinningAnimation`)
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
    world: &World,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    player: &PlayPhasePlayer,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    type Func = fn(
        Option<&str>,
        &TextureDataPool,
        &wgpu::Device,
        &mut wgpu::CommandEncoder,
        &mut Vec<wgpu::Buffer>,
        &World,
        Entity,
        &ModelRoot,
    ) -> (Entity, SkinningAnimation, Vec<(Entity, EntityBuilder)>);
    const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
        aris_original::spawn_character_model,
        momoi_original::spawn_character_model,
        midori_original::spawn_character_model,
        yuuka_original::spawn_character_model,
    ];

    // 모델 풀 객체에서 캐릭터 모델 노드를 가져옵니다.
    let i = player.character_kind as usize;
    let root = model_pool
        .get(CHARACTER_URIS[i])
        .expect("the character model must exist!");

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
        glam::Quat::from_array(player.rotation),
        glam::Vec3::from_array(player.translation),
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add_bundle((
        player.account,
        player.character_kind,
        (player.team(), player.team_index()),
        player.play_data,
        player.health_point,
        player.remaining_bullet,
        player.ex_skill_cost,
    ));
    builder.add_bundle((
        local_transform,
        world_transform,
        player.action_state(),
        player.action_state_timer,
        player.movement_state(),
        player.movement_state_timer,
        player.view_state(),
        player.view_state_timer,
        player.view_rotation,
    ));

    // 캐릭터 종류에 따른 캐릭터 모델을 구성하는 엔터티를 생성합니다.
    let (child, skinning_animation, mut batch_commands) = FUNC_TABLE[i](
        Some(&format!("Player({})", player.account.uid)),
        texture_data_pool,
        device,
        encoder,
        staging_buffers,
        world,
        entity,
        &root,
    );

    // 캐릭터 모델 루트 노드와 스키닝 애니메이션 컴포넌트를 추가합니다.
    builder.add_bundle((Child(child), skinning_animation));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    (entity, batch_commands)
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
    const FUNC_TABLE: [[Func; NUM_ACTION_STATES]; NUM_MOVEMENT_STATES] = [
        // `MovementState::Idle`
        [
            set_character_direction_to_none,                // ActionState::Idle
            set_character_direction_to_camera,              // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_none,                // ActionState::AimOff
            set_character_direction_to_camera,              // ActionState::Attack
            set_character_direction_to_none,                // ActionState::Dead
            set_character_direction_to_none,                // ActionState::Reload
            set_character_direction_to_camera,              // ActionState::Skill
            set_character_direction_to_camera,              // ActionState::ExSkill
            set_character_direction_to_none,                // ActionState::Callsign
        ],
        // `MovementState::Moving`
        [
            set_character_direction_to_movement, // ActionState::Idle
            set_character_direction_to_camera,   // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_current_from_camera, // ActionState::AimOff
            set_character_direction_to_camera,   // ActionState::Attack
            set_character_direction_to_none,     // ActionState::Dead
            set_character_direction_to_camera,   // ActionState::Reload
            set_character_direction_to_camera,   // ActionState::Skill
            set_character_direction_to_camera,   // ActionState::ExSkill
            set_character_direction_to_none,     // ActionState::Callsign
        ],
        // `MovementState::MoveToEnd`
        [
            set_character_direction_to_none,                // ActionState::Idle
            set_character_direction_to_camera,              // ActionState::Aiminig
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_none,                // ActionState::AimOff
            set_character_direction_to_camera,              // ActionState::Attack
            set_character_direction_to_none,                // ActionState::Dead
            set_character_direction_to_none,                // ActionState::Reload
            set_character_direction_to_camera,              // ActionState::Skill
            set_character_direction_to_camera,              // ActionState::ExSkill
            set_character_direction_to_none,                // ActionState::Callsign
        ],
        // `MovementState::InPlaceJumping`
        [
            set_character_direction_to_none,                // ActionState::Idle
            set_character_direction_to_camera,              // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_none,                // ActionState::AimOff
            set_character_direction_to_camera,              // ActionState::Attack
            set_character_direction_to_none,                // ActionState::Dead
            set_character_direction_to_camera,              // ActionState::Reload
            set_character_direction_to_camera,              // ActionState::Skill
            set_character_direction_to_camera,              // ActionState::ExSkill
            set_character_direction_to_none,                // ActionState::Callsign
        ],
        // `MovementState::InPlaceLanding`
        [
            set_character_direction_to_none,                // ActionState::Idle
            set_character_direction_to_camera,              // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_none,                // ActionState::AimOff
            set_character_direction_to_camera,              // ActionState::Attack
            set_character_direction_to_none,                // ActionState::Dead
            set_character_direction_to_camera,              // ActionState::Reload
            set_character_direction_to_camera,              // ActionState::Skill
            set_character_direction_to_camera,              // ActionState::ExSkill
            set_character_direction_to_none,                // ActionState::Callsign
        ],
        // `MovementState::MovingJumping`
        [
            set_character_direction_to_movement, // ActionState::Idle
            set_character_direction_to_camera,   // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_current_from_camera, // ActionState::AimOff
            set_character_direction_to_camera,   // ActionState::Attack
            set_character_direction_to_none,     // ActionState::Dead
            set_character_direction_to_camera,   // ActionState::Reload
            set_character_direction_to_camera,   // ActionState::Skill
            set_character_direction_to_camera,   // ActionState::ExSkill
            set_character_direction_to_none,     // ActionState::Callsign
        ],
        // `MovementState::MovingLanding`
        [
            set_character_direction_to_movement, // ActionState::Idle
            set_character_direction_to_camera,   // ActionState::Aiming
            set_character_direction_to_camera_from_current, // ActionState::AimAt
            set_character_direction_to_current_from_camera, // ActionState::AimOff
            set_character_direction_to_camera,   // ActionState::Attack
            set_character_direction_to_none,     // ActionState::Dead
            set_character_direction_to_camera,   // ActionState::Reload
            set_character_direction_to_camera,   // ActionState::Skill
            set_character_direction_to_camera,   // ActionState::ExSkill
            set_character_direction_to_none,     // ActionState::Callsign
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
    local_transform.look_to(dir, glam::Vec3::Y);
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
    let look = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let i = character_kind as usize;
    let s = view_state_timer.0 / ZOOM_IN_LEN[i];
    let look = look.lerp(direction, s).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec3::Y);
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
    local_transform.look_to(look, glam::Vec3::Y);
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
    let look = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let look = look.lerp(direction, 0.1).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec3::Y);
}

/// `InGameInputFlags`에 따라 `ViewState`를 갱신합니다.
pub fn update_view_state_by_controller_input_flags(
    character_kind: CharacterKind,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    controller_input_flags: GameInputBits,
) {
    type Func = fn(&mut ViewState, &mut ViewStateTimer, GameInputBits);
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
    motion_pool: &MotionPool,
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
        &MotionPool,
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
        motion_pool,
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
    FUNC_TABLE[i](
        action_state,
        skinning_animation,
        child_view,
        sibling_view,
        transform_view,
    );
}

/// 캐릭터의 삼인칭 카메라를 생성합니다.
pub fn create_third_person_camera_of_character(
    character_kind: CharacterKind,
    rotation: LatLon,
) -> ThirdPersonCamera {
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
        rotation,
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
    let j = match action_state {
        _ => view_state as usize,
    };

    FUNC_TABLE[i][j](
        third_person_camera,
        action_state,
        action_state_timer,
        view_state_timer,
    );
}
