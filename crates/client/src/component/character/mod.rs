mod animation;
// mod aris_original;
// mod midori_original;
// mod momoi_original;
mod pipeline;
// mod yuuka_original;

// use ahash::HashMap;
// use hecs::{Entity, EntityBuilder, ViewBorrow, World};
// use mod_network::components::{
//     ActionState, ActionStateTimer, CharacterKind, GameInputBits, LatLon, MovementState,
//     MovementStateTimer, PlayPhasePlayer, ViewState, ViewStateTimer, NUM_ACTION_STATES,
//     NUM_MOVEMENT_STATES,
// };

// use crate::{
//     asset::{ModelPool, ModelRoot, MotionPool, TextureDataPool, CHARACTER_URIS},
//     component::{Child, Sibling, ToParentTrans, WorldTransform},
// };

pub use self::{animation::*, pipeline::*};

// use super::{
//     AttributeKind, CameraResource, LightSetResource, MaterialKind, MaterialResource, Mesh,
//     MeshFilter, MeshRenderer, MoveDirection, OpaqueMap, ShadowMap, ShadowResource,
//     SkinnedMeshRenderer, ThirdPersonCamera, TransformDataLayout,
// };

// /// 캐릭터의 수
// const NUM_CHARACTERS: usize = 4;

// /// 캐릭터 헤일로의 종류입니다.
// #[repr(u8)]
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub enum CharacterHaloKind {
//     ArisOriginalHalo = 0,
//     MomoiOriginalHalo = 1,
//     MidoriOriginalHalo = 2,
//     YuukaOriginalHalo = 3,
// }

// impl From<CharacterKind> for CharacterHaloKind {
//     fn from(value: CharacterKind) -> Self {
//         match value {
//             CharacterKind::ArisOriginal => CharacterHaloKind::ArisOriginalHalo,
//             CharacterKind::MomoiOriginal => CharacterHaloKind::MomoiOriginalHalo,
//             CharacterKind::MidoriOriginal => CharacterHaloKind::MidoriOriginalHalo,
//             CharacterKind::YuukaOriginal => CharacterHaloKind::YuukaOriginalHalo,
//         }
//     }
// }

// impl ToString for CharacterHaloKind {
//     fn to_string(&self) -> String {
//         match self {
//             CharacterHaloKind::ArisOriginalHalo => "Aris Original Halo",
//             CharacterHaloKind::MomoiOriginalHalo => "Momoi Original Halo",
//             CharacterHaloKind::MidoriOriginalHalo => "Midori Original Halo",
//             CharacterHaloKind::YuukaOriginalHalo => "Yuuka Original Halo",
//         }
//         .to_string()
//     }
// }

// /// 플레이어 캐릭터를 구성하는 엔터티를 생성합니다.
// ///
// /// 생성된 엔터티는 아래 컴포넌트를 가집니다
// /// - 자식 엔터티(`Child`)
// /// - 캐릭터 종류(`CharacterKind`)
// /// - 로컬 변환 행렬(`ToParentTrans`)
// /// - 월드 변환 행렬(`WorldTransform`)
// /// - 스키닝 애니메이션(`SkinningAnimation`)
// /// - 체력(`HealthPoint`)
// /// - 행동 상태(`ActionState`)
// /// - 행동 상태 지속 시간 타이머(`ActionStateTimer`)
// /// - 움직임 상태(`MovementState`)
// /// - 움직임 상태 지속 시간 타이머(`MovementStateTimer`)
// /// - 시야 상태(`ViewState`)
// /// - 시야 상태 지속 시간 타이머(`ViewStateTimer`)
// /// - 시야 방향(`Latlon`)
// ///
// pub fn spawn_player_character(
//     world: &World,
//     model_pool: &ModelPool,
//     texture_data_pool: &TextureDataPool,
//     player: &PlayPhasePlayer,
//     device: &wgpu::Device,
//     encoder: &mut wgpu::CommandEncoder,
//     staging_buffers: &mut Vec<wgpu::Buffer>,
// ) -> (Entity, Vec<(Entity, EntityBuilder)>) {
//     type Func = fn(
//         Option<&str>,
//         &TextureDataPool,
//         &wgpu::Device,
//         &mut wgpu::CommandEncoder,
//         &mut Vec<wgpu::Buffer>,
//         &World,
//         Entity,
//         &ModelRoot,
//     ) -> (Entity, SkinningAnimation, Vec<(Entity, EntityBuilder)>);
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::spawn_character_model,
//         momoi_original::spawn_character_model,
//         midori_original::spawn_character_model,
//         yuuka_original::spawn_character_model,
//     ];

//     // 모델 풀 객체에서 캐릭터 모델 노드를 가져옵니다.
//     let i = player.character_kind as usize;
//     let root = model_pool
//         .get(CHARACTER_URIS[i])
//         .expect("the character model must exist!");

//     // 엔터티를 하나 할당받습니다.
//     let entity = world.reserve_entity();
//     let mut builder = EntityBuilder::new();

//     // 컴포넌트 데이터를 준비합니다.
//     let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
//         glam::Quat::from_array(player.rotation),
//         glam::Vec3::from_array(player.translation),
//     ));
//     let world_transform = WorldTransform::default();

//     // 컴포넌트를 추가합니다.
//     builder.add_bundle((
//         player.account,
//         player.character_kind,
//         (player.team(), player.team_index()),
//         player.play_data,
//         player.health_point,
//         player.remaining_bullet,
//         player.ex_skill_cost,
//     ));
//     builder.add_bundle((
//         local_transform,
//         world_transform,
//         player.action_state(),
//         player.action_state_timer,
//         player.movement_state(),
//         player.movement_state_timer,
//         player.view_state(),
//         player.view_state_timer,
//         player.view_rotation,
//     ));

//     // 캐릭터 종류에 따른 캐릭터 모델을 구성하는 엔터티를 생성합니다.
//     let (child, skinning_animation, mut batch_commands) = FUNC_TABLE[i](
//         Some(&format!("Player({})", player.account.uid)),
//         texture_data_pool,
//         device,
//         encoder,
//         staging_buffers,
//         world,
//         entity,
//         &root,
//     );

//     // 캐릭터 모델 루트 노드와 스키닝 애니메이션 컴포넌트를 추가합니다.
//     builder.add_bundle((Child(child), skinning_animation));

//     // 엔터티 생성 명령어를 추가합니다.
//     batch_commands.push((entity, builder));

//     (entity, batch_commands)
// }

// /// 플레이어 캐릭터의 방향을 갱신합니다.
// /// 이 함수는 캐릭터가 바라보는 방향을 변경합니다. (플레이어 움직임 방향과 다름)
// ///
// /// # Note
// /// 이 함수를 호출하기 전에 `MovementState`, `ViewState`, `ViewStateTimer`, `MoveDirection`, `ThirdPersonCamera`가
// /// 갱신되어야 합니다.
// ///
// pub fn update_character_direction(
//     character_kind: CharacterKind,
//     movement_state: MovementState,
//     action_state: ActionState,
//     action_state_timer: ActionStateTimer,
//     move_direction: &MoveDirection,
//     third_person_camera: &ThirdPersonCamera,
//     local_transform: &mut ToParentTrans,
// ) {
//     type Func =
//         fn(CharacterKind, ActionStateTimer, &MoveDirection, &ThirdPersonCamera, &mut ToParentTrans);
//     const FUNC_TABLE: [[Func; NUM_ACTION_STATES]; NUM_MOVEMENT_STATES] = [
//         // `MovementState::Idle`
//         [
//             set_character_direction_to_none,                // ActionState::Idle
//             set_character_direction_to_camera,              // ActionState::Aiming
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_none,                // ActionState::AimOff
//             set_character_direction_to_camera,              // ActionState::Attack
//             set_character_direction_to_none,                // ActionState::Dead
//             set_character_direction_to_none,                // ActionState::Reload
//             set_character_direction_to_camera,              // ActionState::Skill
//             set_character_direction_to_camera,              // ActionState::ExSkill
//             set_character_direction_to_none,                // ActionState::Callsign
//             set_character_direction_to_none,                // ActionState::VictoryStart
//             set_character_direction_to_none,                // ActionState::VictoryEnd
//         ],
//         // `MovementState::Moving`
//         [
//             set_character_direction_to_movement, // ActionState::Idle
//             set_character_direction_to_camera,   // ActionState::Aiming
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_current_from_camera, // ActionState::AimOff
//             set_character_direction_to_camera,   // ActionState::Attack
//             set_character_direction_to_none,     // ActionState::Dead
//             set_character_direction_to_camera,   // ActionState::Reload
//             set_character_direction_to_camera,   // ActionState::Skill
//             set_character_direction_to_camera,   // ActionState::ExSkill
//             set_character_direction_to_none,     // ActionState::Callsign
//             set_character_direction_to_none,     // ActionState::VictoryStart
//             set_character_direction_to_none,     // ActionState::VictoryEnd
//         ],
//         // `MovementState::MoveToEnd`
//         [
//             set_character_direction_to_none,                // ActionState::Idle
//             set_character_direction_to_camera,              // ActionState::Aiminig
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_none,                // ActionState::AimOff
//             set_character_direction_to_camera,              // ActionState::Attack
//             set_character_direction_to_none,                // ActionState::Dead
//             set_character_direction_to_none,                // ActionState::Reload
//             set_character_direction_to_camera,              // ActionState::Skill
//             set_character_direction_to_camera,              // ActionState::ExSkill
//             set_character_direction_to_none,                // ActionState::Callsign
//             set_character_direction_to_none,                // ActionState::VictoryStart
//             set_character_direction_to_none,                // ActionState::VictoryEnd
//         ],
//         // `MovementState::InPlaceJumping`
//         [
//             set_character_direction_to_none,                // ActionState::Idle
//             set_character_direction_to_camera,              // ActionState::Aiming
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_none,                // ActionState::AimOff
//             set_character_direction_to_camera,              // ActionState::Attack
//             set_character_direction_to_none,                // ActionState::Dead
//             set_character_direction_to_camera,              // ActionState::Reload
//             set_character_direction_to_camera,              // ActionState::Skill
//             set_character_direction_to_camera,              // ActionState::ExSkill
//             set_character_direction_to_none,                // ActionState::Callsign
//             set_character_direction_to_none,                // ActionState::VictoryStart
//             set_character_direction_to_none,                // ActionState::VictoryEnd
//         ],
//         // `MovementState::InPlaceLanding`
//         [
//             set_character_direction_to_none,                // ActionState::Idle
//             set_character_direction_to_camera,              // ActionState::Aiming
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_none,                // ActionState::AimOff
//             set_character_direction_to_camera,              // ActionState::Attack
//             set_character_direction_to_none,                // ActionState::Dead
//             set_character_direction_to_camera,              // ActionState::Reload
//             set_character_direction_to_camera,              // ActionState::Skill
//             set_character_direction_to_camera,              // ActionState::ExSkill
//             set_character_direction_to_none,                // ActionState::Callsign
//             set_character_direction_to_none,                // ActionState::VictoryStart
//             set_character_direction_to_none,                // ActionState::VictoryEnd
//         ],
//         // `MovementState::MovingJumping`
//         [
//             set_character_direction_to_movement, // ActionState::Idle
//             set_character_direction_to_camera,   // ActionState::Aiming
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_current_from_camera, // ActionState::AimOff
//             set_character_direction_to_camera,   // ActionState::Attack
//             set_character_direction_to_none,     // ActionState::Dead
//             set_character_direction_to_camera,   // ActionState::Reload
//             set_character_direction_to_camera,   // ActionState::Skill
//             set_character_direction_to_camera,   // ActionState::ExSkill
//             set_character_direction_to_none,     // ActionState::Callsign
//             set_character_direction_to_none,     // ActionState::VictoryStart
//             set_character_direction_to_none,     // ActionState::VictoryEnd
//         ],
//         // `MovementState::MovingLanding`
//         [
//             set_character_direction_to_movement, // ActionState::Idle
//             set_character_direction_to_camera,   // ActionState::Aiming
//             set_character_direction_to_camera_from_current, // ActionState::AimAt
//             set_character_direction_to_current_from_camera, // ActionState::AimOff
//             set_character_direction_to_camera,   // ActionState::Attack
//             set_character_direction_to_none,     // ActionState::Dead
//             set_character_direction_to_camera,   // ActionState::Reload
//             set_character_direction_to_camera,   // ActionState::Skill
//             set_character_direction_to_camera,   // ActionState::ExSkill
//             set_character_direction_to_none,     // ActionState::Callsign
//             set_character_direction_to_none,     // ActionState::VictoryStart
//             set_character_direction_to_none,     // ActionState::VictoryEnd
//         ],
//     ];

//     let i = movement_state as usize;
//     let j = action_state as usize;
//     FUNC_TABLE[i][j](
//         character_kind,
//         action_state_timer,
//         move_direction,
//         third_person_camera,
//         local_transform,
//     );
// }

// /// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ActionState::Idle`일 때 캐릭터의 방향을 갱신합니다.
// fn set_character_direction_to_none(
//     _character_kind: CharacterKind,
//     _view_state_timer: ActionStateTimer,
//     _move_direction: &MoveDirection,
//     _third_person_camera: &ThirdPersonCamera,
//     _local_transform: &mut ToParentTrans,
// ) {
//     /* empty */
// }

// /// `MovementState::Moving`, `ActionState::Idle`일 때 캐릭터의 방향을 갱신합니다.
// fn set_character_direction_to_movement(
//     _character_kind: CharacterKind,
//     _view_state_timer: ActionStateTimer,
//     move_direction: &MoveDirection,
//     _third_person_camera: &ThirdPersonCamera,
//     local_transform: &mut ToParentTrans,
// ) {
//     // 현재 캐릭터의 방향을 가져옵니다.
//     let look = local_transform.get_look_vector();

//     // 플레이어 이동 방향을 가져옵니다.
//     let direction = move_direction.0;

//     // 두 방향을 각도에 따라 선형 보간합니다.
//     let dir = look.lerp(direction, 0.5);

//     // 로컬 변환 행렬을 갱신합니다.
//     local_transform.look_to(dir, glam::Vec3::Y);
// }

// /// `MovementState::Idle` 또는 `MovementState::MoveToEnd`, `ViewState::ZoomIn`일 때 캐릭터의 방향을 갱신합니다.
// fn set_character_direction_to_camera_from_current(
//     character_kind: CharacterKind,
//     view_state_timer: ActionStateTimer,
//     _move_direction: &MoveDirection,
//     third_person_camera: &ThirdPersonCamera,
//     local_transform: &mut ToParentTrans,
// ) {
//     const ZOOM_IN_LEN: [f32; NUM_CHARACTERS] = [
//         aris_original::NORMAL_ATTACK_START_DURATION,
//         momoi_original::NORMAL_ATTACK_START_DURATION,
//         midori_original::NORMAL_ATTACK_START_DURATION,
//         yuuka_original::NORMAL_ATTACK_START_DURATION,
//     ];

//     // 삼인칭 카메라의 방향을 계산합니다.
//     let mat = glam::Mat4::from_rotation_y(third_person_camera.rotation.lon);
//     let look = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

//     // 캐릭터의 방향을 가져옵니다.
//     let direction = local_transform.get_look_vector();

//     // 선형 보간된 방향을 계산합니다.
//     let i = character_kind as usize;
//     let s = view_state_timer.0 / ZOOM_IN_LEN[i];
//     let look = look.lerp(direction, s).normalize_or(look);

//     // 로컬 변환 행렬을 갱신합니다.
//     local_transform.look_to(look, glam::Vec3::Y);
// }

// /// `MovementState::Moving`, `ViewState::ZoomOut`일 때 캐릭터의 방향을 갱신합니다.
// fn set_character_direction_to_current_from_camera(
//     character_kind: CharacterKind,
//     view_state_timer: ActionStateTimer,
//     move_direction: &MoveDirection,
//     _third_person_camera: &ThirdPersonCamera,
//     local_transform: &mut ToParentTrans,
// ) {
//     const ZOOM_OUT_LEN: [f32; NUM_CHARACTERS] = [
//         aris_original::NORMAL_ATTACK_END_DURATION,
//         momoi_original::NORMAL_ATTACK_END_DURATION,
//         midori_original::NORMAL_ATTACK_END_DURATION,
//         yuuka_original::NORMAL_ATTACK_END_DURATION,
//     ];

//     // 캐릭터의 방향을 가져옵니다.
//     let look = local_transform.get_look_vector();

//     // 선형 보간된 방향을 계산합니다.
//     let i = character_kind as usize;
//     let s = view_state_timer.0 / ZOOM_OUT_LEN[i];
//     let look = move_direction
//         .0
//         .lerp(look, s)
//         .normalize_or(move_direction.0);

//     // 로컬 변환 행렬을 갱신합니다.
//     local_transform.look_to(look, glam::Vec3::Y);
// }

// fn set_character_direction_to_camera(
//     _character_kind: CharacterKind,
//     _view_state_timer: ActionStateTimer,
//     _move_direction: &MoveDirection,
//     third_person_camera: &ThirdPersonCamera,
//     local_transform: &mut ToParentTrans,
// ) {
//     // 삼인칭 카메라의 방향을 계산합니다.
//     let mat = glam::Mat4::from_rotation_y(third_person_camera.rotation.lon);
//     let look = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

//     // 캐릭터의 방향을 가져옵니다.
//     let direction = local_transform.get_look_vector();

//     // 선형 보간된 방향을 계산합니다.
//     let look = look.lerp(direction, 0.1).normalize_or(look);

//     // 로컬 변환 행렬을 갱신합니다.
//     local_transform.look_to(look, glam::Vec3::Y);
// }

// /// `InGameInputFlags`에 따라 `ViewState`를 갱신합니다.
// pub fn update_view_state_by_controller_input_flags(
//     character_kind: CharacterKind,
//     view_state: &mut ViewState,
//     view_state_timer: &mut ViewStateTimer,
//     controller_input_flags: GameInputBits,
// ) {
//     type Func = fn(&mut ViewState, &mut ViewStateTimer, GameInputBits);
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::update_character_view_state,
//         momoi_original::update_character_view_state,
//         midori_original::update_character_view_state,
//         yuuka_original::update_character_view_state,
//     ];

//     let i = character_kind as usize;
//     FUNC_TABLE[i](view_state, view_state_timer, controller_input_flags);
// }

// /// 주어진 경과 시간 만큼 `ViewStateTimer`를 갱신합니다.
// pub fn update_view_state_timer(
//     character_kind: CharacterKind,
//     view_state: &mut ViewState,
//     view_state_timer: &mut ViewStateTimer,
//     elapsed_time_sec: f32,
// ) {
//     type Func = fn(&mut ViewState, &mut ViewStateTimer, f32);
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::update_character_view_state_timer,
//         momoi_original::update_character_view_state_timer,
//         midori_original::update_character_view_state_timer,
//         yuuka_original::update_character_view_state_timer,
//     ];

//     let i = character_kind as usize;
//     FUNC_TABLE[i](view_state, view_state_timer, elapsed_time_sec);
// }

// /// 주어진 경과 시간 만큼 `MovementStateTimer`를 갱신합니다.
// pub fn update_movement_state_timer(
//     character_kind: CharacterKind,
//     action_state: ActionState,
//     movement_state: &mut MovementState,
//     movement_state_timer: &mut MovementStateTimer,
//     elapsed_time_sec: f32,
// ) {
//     type Func = fn(ActionState, &mut MovementState, &mut MovementStateTimer, f32);
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::update_movement_state_timer,
//         momoi_original::update_movement_state_timer,
//         midori_original::update_movement_state_timer,
//         yuuka_original::update_movement_state_timer,
//     ];

//     let i = character_kind as usize;
//     FUNC_TABLE[i](
//         action_state,
//         movement_state,
//         movement_state_timer,
//         elapsed_time_sec,
//     );
// }

// /// 주어진 경과 시간 만큼 `ActionStateTimer`를 갱신합니다.
// pub fn update_action_state_timer(
//     character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     elapsed_time_sec: f32,
// ) {
//     type Func = fn(&mut ActionState, &mut ActionStateTimer, f32);
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::update_action_state_timer,
//         momoi_original::update_action_state_timer,
//         midori_original::update_action_state_timer,
//         yuuka_original::update_action_state_timer,
//     ];

//     let i = character_kind as usize;
//     FUNC_TABLE[i](action_state, action_state_timer, elapsed_time_sec);
// }

// /// `ActionState` 변경을 시도합니다.
// /// 해당 `ActionState`로 변경이 불가능할 경우 무시됩니다.
// pub fn try_change_action_state(
//     character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(CharacterKind, &mut ActionState, &mut ActionStateTimer, ActionState);
//     const FUNC_TABLE: [Func; NUM_ACTION_STATES] = [
//         try_change_action_state_when_idle,
//         try_change_action_state_when_aiming,
//         try_change_action_state_when_aim_at,
//         try_change_action_state_when_aim_off,
//         try_change_action_state_when_attack,
//         try_change_action_state_when_dead,
//         try_change_action_state_when_reload,
//         try_change_action_state_when_skill,
//         try_change_action_state_when_ex_skill,
//         try_change_action_state_when_special,
//         try_change_action_state_when_special,
//         try_change_action_state_when_special,
//     ];

//     let i = *action_state as usize;
//     FUNC_TABLE[i](character_kind, action_state, action_state_timer, new);
// }

// /// `ActionState::Idle`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_idle(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Idle, |_| {}),  // ActionState::Idle
//         (ActionState::Idle, |_| {}),  // ActionState::Aiming
//         (ActionState::AimAt, |_| {}), // ActionState::AimAt
//         (ActionState::Idle, |_| {}),  // ActionState::AimOff
//         (ActionState::Attack, |t| {
//             t.reset();
//         }), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::Reload, |t| {
//             t.reset();
//         }), // ActionState::Reload
//         (ActionState::Skill, |t| {
//             t.reset();
//         }), // ActionState::Skill
//         (ActionState::ExSkill, |t| {
//             t.reset();
//         }), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `ActionState::Aiming`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_aiming(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Aiming, |_| {}), // ActionState::Idle
//         (ActionState::Aiming, |_| {}), // ActionState::Aiming
//         (ActionState::Aiming, |_| {}), // ActionState::AimAt
//         (ActionState::AimOff, |t| {
//             t.reset();
//         }), // ActionState::AimOff
//         (ActionState::Attack, |t| {
//             t.reset();
//         }), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::Aiming, |_| {}), // ActionState::Reload
//         (ActionState::Skill, |t| {
//             t.reset();
//         }), // ActionState::Skill
//         (ActionState::ExSkill, |t| {
//             t.reset();
//         }), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `ActionState::AimAt`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_aim_at(
//     character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     fn switch_timer(kind: CharacterKind, t: &mut ActionStateTimer) {
//         const DURATIONS: [(f32, f32); NUM_CHARACTERS] = [
//             (
//                 aris_original::NORMAL_ATTACK_START_DURATION,
//                 aris_original::NORMAL_ATTACK_END_DURATION,
//             ),
//             (
//                 momoi_original::NORMAL_ATTACK_START_DURATION,
//                 momoi_original::NORMAL_ATTACK_END_DURATION,
//             ),
//             (
//                 midori_original::NORMAL_ATTACK_START_DURATION,
//                 midori_original::NORMAL_ATTACK_END_DURATION,
//             ),
//             (
//                 yuuka_original::NORMAL_ATTACK_START_DURATION,
//                 yuuka_original::NORMAL_ATTACK_END_DURATION,
//             ),
//         ];

//         let (start, end) = DURATIONS[kind as usize];
//         let p = (start - t.0) / start;
//         t.0 = end * p;
//     }

//     type Func = fn(CharacterKind, &mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::AimAt, |_, _| {}), // ActionState::Idle
//         (ActionState::Aiming, |_, t| {
//             t.reset();
//         }), // ActionState::Aiming
//         (ActionState::AimAt, |_, _| {}), // ActionState::AimAt
//         (ActionState::AimOff, switch_timer), // ActionState::AimOff
//         (ActionState::AimAt, |_, _| {}), // ActionState::Attack
//         (ActionState::Dead, |_, t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::AimAt, |_, _| {}), // ActionState::Reload
//         (ActionState::AimAt, |_, _| {}), // ActionState::Skill
//         (ActionState::AimAt, |_, _| {}), // ActionState::ExSkill
//         (ActionState::Callsign, |_, t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |_, t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryEnd, |_, t| {
//             t.reset();
//         }), // ActionState::Callsign
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(character_kind, action_state_timer);
// }

// /// `ActionState::AimOff`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_aim_off(
//     character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     fn switch_timer(kind: CharacterKind, t: &mut ActionStateTimer) {
//         const DURATIONS: [(f32, f32); NUM_CHARACTERS] = [
//             (
//                 aris_original::NORMAL_ATTACK_START_DURATION,
//                 aris_original::NORMAL_ATTACK_END_DURATION,
//             ),
//             (
//                 momoi_original::NORMAL_ATTACK_START_DURATION,
//                 momoi_original::NORMAL_ATTACK_END_DURATION,
//             ),
//             (
//                 midori_original::NORMAL_ATTACK_START_DURATION,
//                 midori_original::NORMAL_ATTACK_END_DURATION,
//             ),
//             (
//                 yuuka_original::NORMAL_ATTACK_START_DURATION,
//                 yuuka_original::NORMAL_ATTACK_END_DURATION,
//             ),
//         ];

//         let (start, end) = DURATIONS[kind as usize];
//         let p = (end - t.0) / end;
//         t.0 = start * p;
//     }

//     type Func = fn(CharacterKind, &mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Idle, |_, t| {
//             t.reset();
//         }), // ActionState::Idle
//         (ActionState::AimOff, |_, _| {}),   // ActionState::Aiming
//         (ActionState::AimAt, switch_timer), // ActionState::AimAt
//         (ActionState::AimOff, |_, _| {}),   // ActionState::AimOff
//         (ActionState::AimOff, |_, _| {}),   // ActionState::Attack
//         (ActionState::Dead, |_, t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::AimOff, |_, _| {}),   // ActionState::Reload
//         (ActionState::AimOff, |_, _| {}),   // ActionState::Skill
//         (ActionState::AimOff, |_, _| {}),   // ActionState::ExSkill
//         (ActionState::Callsign, |_, t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |_, t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |_, t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(character_kind, action_state_timer);
// }

// /// `ActionState::Attack`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_attack(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Attack, |_| {}), // ActionState::Idle
//         (ActionState::Attack, |_| {}), // ActionState::Aiming
//         (ActionState::Attack, |_| {}), // ActionState::AimAt
//         (ActionState::Attack, |_| {}), // ActionState::AimOff
//         (ActionState::Attack, |_| {}), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::Attack, |_| {}), // ActionState::Reload
//         (ActionState::Attack, |_| {}), // ActionState::Skill
//         (ActionState::Attack, |_| {}), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `ActionState::Dead`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_dead(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Dead, |_| {}), // ActionState::Idle
//         (ActionState::Dead, |_| {}), // ActionState::Aiming
//         (ActionState::Dead, |_| {}), // ActionState::AimAt
//         (ActionState::Dead, |_| {}), // ActionState::AimOff
//         (ActionState::Dead, |_| {}), // ActionState::Attack
//         (ActionState::Dead, |_| {}), // ActionState::Dead
//         (ActionState::Dead, |_| {}), // ActionState::Reload
//         (ActionState::Dead, |_| {}), // ActionState::Skill
//         (ActionState::Dead, |_| {}), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `ActionState::Reload`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_reload(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Reload, |_| {}), // ActionState::Idle
//         (ActionState::Reload, |_| {}), // ActionState::Aiming
//         (ActionState::Reload, |_| {}), // ActionState::AimAt
//         (ActionState::Reload, |_| {}), // ActionState::AimOff
//         (ActionState::Reload, |_| {}), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::Reload, |_| {}), // ActionState::Reload
//         (ActionState::Reload, |_| {}), // ActionState::Skill
//         (ActionState::Reload, |_| {}), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `ActionState::Skill`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_skill(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Skill, |_| {}), // ActionState::Idle
//         (ActionState::Skill, |_| {}), // ActionState::Aiming
//         (ActionState::Skill, |_| {}), // ActionState::AimAt
//         (ActionState::Skill, |_| {}), // ActionState::AimOff
//         (ActionState::Skill, |_| {}), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::Skill, |_| {}), // ActionState::Reload
//         (ActionState::Skill, |_| {}), // ActionState::Skill
//         (ActionState::Skill, |_| {}), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `ActionState::ExSkill`일 때 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_ex_skill(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::ExSkill, |_| {}), // ActionState::Idle
//         (ActionState::ExSkill, |_| {}), // ActionState::Aiming
//         (ActionState::ExSkill, |_| {}), // ActionState::AimAt
//         (ActionState::ExSkill, |_| {}), // ActionState::AimOff
//         (ActionState::ExSkill, |_| {}), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::ExSkill, |_| {}), // ActionState::Reload
//         (ActionState::ExSkill, |_| {}), // ActionState::Skill
//         (ActionState::ExSkill, |_| {}), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// 주어진 상태로 변경을 시도합니다.
// fn try_change_action_state_when_special(
//     _character_kind: CharacterKind,
//     action_state: &mut ActionState,
//     action_state_timer: &mut ActionStateTimer,
//     new: ActionState,
// ) {
//     type Func = fn(&mut ActionStateTimer);
//     const TABLE: [(ActionState, Func); NUM_ACTION_STATES] = [
//         (ActionState::Idle, |t| {
//             t.reset();
//         }), // ActionState::Idle
//         (ActionState::Aiming, |t| {
//             t.reset();
//         }), // ActionState::Aiming
//         (ActionState::AimAt, |t| {
//             t.reset();
//         }), // ActionState::AimAt
//         (ActionState::AimOff, |t| {
//             t.reset();
//         }), // ActionState::AimOff
//         (ActionState::Attack, |t| {
//             t.reset();
//         }), // ActionState::Attack
//         (ActionState::Dead, |t| {
//             t.reset();
//         }), // ActionState::Dead
//         (ActionState::Reload, |t| {
//             t.reset();
//         }), // ActionState::Reload
//         (ActionState::Skill, |t| {
//             t.reset();
//         }), // ActionState::Skill
//         (ActionState::ExSkill, |t| {
//             t.reset();
//         }), // ActionState::ExSkill
//         (ActionState::Callsign, |t| {
//             t.reset();
//         }), // ActionState::Callsign
//         (ActionState::VictoryStart, |t| {
//             t.reset();
//         }), // ActionState::VictoryStart
//         (ActionState::VictoryEnd, |t| {
//             t.reset();
//         }), // ActionState::VictoryEnd
//     ];

//     let i = new as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *action_state = next_state;
//     timer_func(action_state_timer);
// }

// /// `MovementState` 변경을 시도합니다.
// /// `MovementState::Idle`로 변경이 불가능할 경우 무시됩니다.
// pub fn try_reset_movement_state(
//     movement_state: &mut MovementState,
//     movement_state_timer: &mut MovementStateTimer,
// ) {
//     type Func = fn(&mut MovementStateTimer);
//     const TABLE: [(MovementState, Func); NUM_MOVEMENT_STATES] = [
//         (MovementState::Idle, |_| {}), // MovementState::Idle
//         (MovementState::Idle, |t| {
//             t.reset();
//         }), // MovementState::Moving
//         (MovementState::MoveToEnd, |_| {}), // MovementState::MoveToEnd
//         (MovementState::InPlaceJumping, |_| {}), // MovementState::InPlaceJumping
//         (MovementState::InPlaceLanding, |_| {}), // MovementState::InPlaceLanding
//         (MovementState::MovingJumping, |_| {}), // MovementState::MovingJumping
//         (MovementState::MovingLanding, |_| {}), // MovementState::MovingLanding
//     ];

//     let i = *movement_state as usize;
//     let (next_state, timer_func) = TABLE[i];

//     *movement_state = next_state;
//     timer_func(movement_state_timer);
// }

// pub fn animate_character(
//     motion_pool: &MotionPool,
//     character_kind: CharacterKind,
//     view_rotation: LatLon,
//     action_state: ActionState,
//     action_state_timer: ActionStateTimer,
//     movement_state: MovementState,
//     movement_state_timer: MovementStateTimer,
//     skinning_animation: &SkinningAnimation,
//     collection_view: &ViewBorrow<&BoneCollection>,
//     transform_view: &mut ViewBorrow<&mut ToParentTrans>,
// ) {
//     type Func = fn(
//         &MotionPool,
//         LatLon,
//         ActionState,
//         ActionStateTimer,
//         MovementState,
//         MovementStateTimer,
//         &SkinningAnimation,
//         &ViewBorrow<&BoneCollection>,
//         &mut ViewBorrow<&mut ToParentTrans>,
//     );
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::animate_character,
//         momoi_original::animate_character,
//         midori_original::animate_character,
//         yuuka_original::animate_character,
//     ];

//     let i = character_kind as usize;
//     FUNC_TABLE[i](
//         motion_pool,
//         view_rotation,
//         action_state,
//         action_state_timer,
//         movement_state,
//         movement_state_timer,
//         skinning_animation,
//         collection_view,
//         transform_view,
//     );
// }

// /// 무기의 위치를 설정합니다.
// ///
// /// # NOTE
// /// 이 함수는 캐릭터의 월드 변환 행렬이 계산된 후 호출해야 합니다.
// ///
// pub fn set_weapon_position(
//     character_kind: CharacterKind,
//     action_state: ActionState,
//     skinning_animation: &SkinningAnimation,
//     child_view: &ViewBorrow<&Child>,
//     sibling_view: &ViewBorrow<&Sibling>,
//     transform_view: &mut ViewBorrow<(&ToParentTrans, &mut WorldTransform)>,
// ) {
//     type Func = fn(
//         ActionState,
//         &SkinningAnimation,
//         &ViewBorrow<&Child>,
//         &ViewBorrow<&Sibling>,
//         &mut ViewBorrow<(&ToParentTrans, &mut WorldTransform)>,
//     );
//     const FUNC_TABLE: [Func; NUM_CHARACTERS] = [
//         aris_original::set_weapon_position,
//         momoi_original::set_weapon_position,
//         midori_original::set_weapon_position,
//         yuuka_original::set_weapon_position,
//     ];

//     let i = character_kind as usize;
//     FUNC_TABLE[i](
//         action_state,
//         skinning_animation,
//         child_view,
//         sibling_view,
//         transform_view,
//     );
// }

// /// 캐릭터의 삼인칭 카메라를 생성합니다.
// pub fn create_third_person_camera_of_character(
//     character_kind: CharacterKind,
//     rotation: LatLon,
// ) -> ThirdPersonCamera {
//     const CAMERA_FOV_Y: [f32; NUM_CHARACTERS] = [
//         aris_original::CAMERA_IDLE_FOV_Y,
//         momoi_original::CAMERA_IDLE_FOV_Y,
//         midori_original::CAMERA_IDLE_FOV_Y,
//         yuuka_original::CAMERA_IDLE_FOV_Y,
//     ];
//     const CAMERA_POSITION: [glam::Vec3A; NUM_CHARACTERS] = [
//         aris_original::CAMERA_IDLE_POSITION,
//         momoi_original::CAMERA_IDLE_POSITION,
//         midori_original::CAMERA_IDLE_POSITION,
//         yuuka_original::CAMERA_IDLE_POSITION,
//     ];

//     let i = character_kind as usize;
//     ThirdPersonCamera {
//         fov_y: CAMERA_FOV_Y[i],
//         rotation,
//         position: CAMERA_POSITION[i],
//     }
// }

// /// 삼인칭 카메라를 갱신합니다.
// ///
// /// # Note
// /// 이 함수를 호출하기 전에 `ViewState`가 먼저 갱신되어야합니다.
// ///
// pub fn update_third_person_camera(
//     third_person_camera: &mut ThirdPersonCamera,
//     character_kind: CharacterKind,
//     action_state: ActionState,
//     action_state_timer: ActionStateTimer,
//     view_state: ViewState,
//     view_state_timer: ViewStateTimer,
// ) {
//     type Func = fn(&mut ThirdPersonCamera, ActionState, ActionStateTimer, ViewStateTimer);
//     const FUNC_TABLE: [[Func; 4]; NUM_CHARACTERS] = [
//         [
//             aris_original::update_third_person_camera_when_idle,
//             aris_original::update_third_person_camera_when_zoom_in,
//             aris_original::update_third_person_camera_when_zoom_out,
//             aris_original::update_third_person_camera_when_aiming,
//         ],
//         [
//             momoi_original::update_third_person_camera_when_idle,
//             momoi_original::update_third_person_camera_when_zoom_in,
//             momoi_original::update_third_person_camera_when_zoom_out,
//             momoi_original::update_third_person_camera_when_aiming,
//         ],
//         [
//             midori_original::update_third_person_camera_when_idle,
//             midori_original::update_third_person_camera_when_zoom_in,
//             midori_original::update_third_person_camera_when_zoom_out,
//             midori_original::update_third_person_camera_when_aiming,
//         ],
//         [
//             yuuka_original::update_third_person_camera_when_idle,
//             yuuka_original::update_third_person_camera_when_zoom_in,
//             yuuka_original::update_third_person_camera_when_zoom_out,
//             yuuka_original::update_third_person_camera_when_aiming,
//         ],
//     ];

//     let i = character_kind as usize;
//     let j = match action_state {
//         _ => view_state as usize,
//     };

//     FUNC_TABLE[i][j](
//         third_person_camera,
//         action_state,
//         action_state_timer,
//         view_state_timer,
//     );
// }

// /// 캐릭터의 쉐이더 리소스를 갱신합니다.
// pub fn update_character_resource(
//     entity: Entity,
//     device: &wgpu::Device,
//     encoder: &mut wgpu::CommandEncoder,
//     staging_buffers: &mut Vec<wgpu::Buffer>,
//     shadow_map: &mut ShadowMap,
//     opaque_map: &mut OpaqueMap,
//     child_view: &ViewBorrow<'_, &Child>,
//     sibling_view: &ViewBorrow<'_, &Sibling>,
//     transform_view: &ViewBorrow<'_, &WorldTransform>,
//     mesh_filter_view: &mut ViewBorrow<'_, MeshRenderer>,
//     skinned_mesh_filter_view: &mut ViewBorrow<'_, SkinnedMeshRenderer>,
// ) {
//     // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
//     if let Some(child_entity) = child_view.get(entity).cloned() {
//         update_character_resource(
//             *child_entity,
//             device,
//             encoder,
//             staging_buffers,
//             shadow_map,
//             opaque_map,
//             child_view,
//             sibling_view,
//             transform_view,
//             mesh_filter_view,
//             skinned_mesh_filter_view,
//         );
//     }

//     // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
//     if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
//         update_character_resource(
//             *sibling_entity,
//             device,
//             encoder,
//             staging_buffers,
//             shadow_map,
//             opaque_map,
//             child_view,
//             sibling_view,
//             transform_view,
//             mesh_filter_view,
//             skinned_mesh_filter_view,
//         );
//     }

//     let result = mesh_filter_view.get_mut(entity);
//     if let Some((mesh, mesh_resource, uniform, _, materials)) = result {
//         // 유니폼 버퍼를 갱신합니다.
//         let transform = transform_view
//             .get(entity)
//             .expect("invalid entity component");
//         uniform.update(
//             device,
//             encoder,
//             staging_buffers,
//             TransformDataLayout {
//                 trans: transform.0.to_cols_array(),
//             },
//         );

//         // 렌더 집합에 추가합니다.
//         for (index, material) in materials.iter().enumerate() {
//             let key = (mesh.clone(), material.kind());
//             let sub_key = (index, material.clone());
//             let val = MeshFilter::Mesh(mesh_resource.clone());
//             if let Some(res_map) = opaque_map.get_mut(&key) {
//                 match res_map.get_mut(&sub_key) {
//                     Some(filters) => {
//                         filters.push(val);
//                     }
//                     None => {
//                         res_map.insert(sub_key, vec![val]);
//                     }
//                 }
//             } else {
//                 opaque_map.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
//             }
//         }

//         // 그림자 집합에 추가합니다.
//         for (index, material) in materials.iter().enumerate() {
//             if material.kind() == MaterialKind::Character
//                 || material.kind() == MaterialKind::CharacterEyeMouth
//             {
//                 let key = (mesh.clone(), material.kind());
//                 let val = MeshFilter::Mesh(mesh_resource.clone());
//                 if let Some(res_map) = shadow_map.get_mut(&key) {
//                     match res_map.get_mut(&index) {
//                         Some(filters) => {
//                             filters.push(val);
//                         }
//                         None => {
//                             res_map.insert(index, vec![val]);
//                         }
//                     }
//                 } else {
//                     shadow_map.insert(key, HashMap::from_iter([(index, vec![val])]));
//                 }
//             }
//         }

//         return;
//     }

//     let result = skinned_mesh_filter_view.get_mut(entity);
//     if let Some((mesh, mesh_resource, collection, uniform, _, materials)) = result {
//         // 유니폼 버퍼를 갱신합니다.
//         let data = collection
//             .bones
//             .iter()
//             .map(|&entity| {
//                 transform_view
//                     .get(entity)
//                     .expect("invalid entity or invalid entity component")
//             })
//             .map(|transform| transform.0.to_cols_array())
//             .collect();
//         uniform.update(device, encoder, staging_buffers, data);

//         // 렌더 집합에 추가합니다.
//         for (index, material) in materials.iter().enumerate() {
//             let key = (mesh.clone(), material.kind());
//             let sub_key = (index, material.clone());
//             let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
//             if let Some(res_map) = opaque_map.get_mut(&key) {
//                 match res_map.get_mut(&sub_key) {
//                     Some(filters) => {
//                         filters.push(val);
//                     }
//                     None => {
//                         res_map.insert(sub_key, vec![val]);
//                     }
//                 }
//             } else {
//                 opaque_map.insert(key, HashMap::from_iter([(sub_key, vec![val])]));
//             }
//         }

//         // 그림자 집합에 추가합니다.
//         for (index, material) in materials.iter().enumerate() {
//             if material.kind() == MaterialKind::Character
//                 || material.kind() == MaterialKind::CharacterEyeMouth
//             {
//                 let key = (mesh.clone(), material.kind());
//                 let val = MeshFilter::SkinnedMesh(mesh_resource.clone());
//                 if let Some(res_map) = shadow_map.get_mut(&key) {
//                     match res_map.get_mut(&index) {
//                         Some(filters) => {
//                             filters.push(val);
//                         }
//                         None => {
//                             res_map.insert(index, vec![val]);
//                         }
//                     }
//                 } else {
//                     shadow_map.insert(key, HashMap::from_iter([(index, vec![val])]));
//                 }
//             }
//         }

//         return;
//     }
// }

// /// 캐릭터를 그립니다.
// pub fn draw_character<'a>(
//     mesh: &'a Mesh,
//     pipeline: &'a wgpu::RenderPipeline,
//     camera_resource: &'a CameraResource,
//     light_set_resource: &'a LightSetResource,
//     material_resources: &'a HashMap<(usize, MaterialResource), Vec<MeshFilter>>,
//     rpass: &mut wgpu::RenderPass<'a>,
// ) {
//     rpass.set_pipeline(&pipeline);

//     rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
//     rpass.set_bind_group(3, light_set_resource.bind_group(), &[]);

//     rpass.set_vertex_buffer(0, mesh.vertex(..));
//     rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
//     rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
//     rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
//     rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

//     for ((index, material), filters) in material_resources {
//         let index_buffer = mesh.submeshes().get(*index).unwrap();
//         rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
//         rpass.set_bind_group(2, material.bind_group(), &[]);

//         for resource in filters {
//             rpass.set_bind_group(1, resource.bind_group(), &[]);
//             rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
//         }
//     }
// }

// /// 캐릭터의 그림자를 생성합니다.
// pub fn bake_character<'a>(
//     mesh: &'a Mesh,
//     pipeline: &'a wgpu::RenderPipeline,
//     shadow_resource: &'a ShadowResource,
//     submesh_resources: &'a HashMap<usize, Vec<MeshFilter>>,
//     rpass: &mut wgpu::RenderPass<'a>,
// ) {
//     rpass.set_pipeline(&pipeline);

//     rpass.set_bind_group(0, &shadow_resource.bind_group, &[]);

//     rpass.set_vertex_buffer(0, mesh.vertex(..));
//     rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
//     rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

//     for (index, filters) in submesh_resources {
//         let index_buffer = mesh.submeshes().get(*index).unwrap();
//         rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());

//         for resource in filters {
//             rpass.set_bind_group(1, resource.bind_group(), &[]);
//             rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
//         }
//     }
// }

// /// 캐릭터의 눈과 입을 그립니다.
// pub fn draw_character_eye_mouth<'a>(
//     mesh: &'a Mesh,
//     pipeline: &'a wgpu::RenderPipeline,
//     camera_resource: &'a CameraResource,
//     light_set_resource: &'a LightSetResource,
//     material_resources: &'a HashMap<(usize, MaterialResource), Vec<MeshFilter>>,
//     rpass: &mut wgpu::RenderPass<'a>,
// ) {
//     rpass.set_pipeline(&pipeline);

//     rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
//     rpass.set_bind_group(3, light_set_resource.bind_group(), &[]);

//     rpass.set_vertex_buffer(0, mesh.vertex(..));
//     rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
//     rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
//     rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
//     rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

//     for ((index, material), filters) in material_resources {
//         let index_buffer = mesh.submeshes().get(*index).unwrap();
//         rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
//         rpass.set_bind_group(2, material.bind_group(), &[]);

//         for resource in filters {
//             rpass.set_bind_group(1, resource.bind_group(), &[]);
//             rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
//         }
//     }
// }

// /// 캐릭터의 눈과 입의 그림자를 생성합니다.
// pub fn bake_character_eye_mouth<'a>(
//     mesh: &'a Mesh,
//     pipeline: &'a wgpu::RenderPipeline,
//     shadow_resource: &'a ShadowResource,
//     submesh_resources: &'a HashMap<usize, Vec<MeshFilter>>,
//     rpass: &mut wgpu::RenderPass<'a>,
// ) {
//     rpass.set_pipeline(&pipeline);

//     rpass.set_bind_group(0, &shadow_resource.bind_group, &[]);

//     rpass.set_vertex_buffer(0, mesh.vertex(..));
//     rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
//     rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

//     for (index, filters) in submesh_resources {
//         let index_buffer = mesh.submeshes().get(*index).unwrap();
//         rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());

//         for resource in filters {
//             rpass.set_bind_group(1, resource.bind_group(), &[]);
//             rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
//         }
//     }
// }

// /// 캐릭터의 헤일로를 그립니다.
// pub fn draw_character_halo<'a>(
//     mesh: &'a Mesh,
//     pipeline: &'a wgpu::RenderPipeline,
//     camera_resource: &'a CameraResource,
//     material_resources: &'a HashMap<(usize, MaterialResource), Vec<MeshFilter>>,
//     rpass: &mut wgpu::RenderPass<'a>,
// ) {
//     rpass.set_pipeline(&pipeline);

//     rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

//     rpass.set_vertex_buffer(0, mesh.vertex(..));
//     rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

//     for ((index, material), filters) in material_resources {
//         let index_buffer = mesh.submeshes().get(*index).unwrap();
//         rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
//         rpass.set_bind_group(2, material.bind_group(), &[]);

//         for resource in filters {
//             rpass.set_bind_group(1, resource.bind_group(), &[]);
//             rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
//         }
//     }
// }
