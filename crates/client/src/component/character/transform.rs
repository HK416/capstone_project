use hecs::{Entity, World};
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, LatLon, MovementState,
};

use crate::component::{
    MoveDirection, Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8,
    Player9, PlayerArchetype, ToParentTrans,
};

/// 플레이어 캐릭터가 바라보는 방향을 갱신합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 플레이어 상태가 갱신되어야 합니다.
///
pub fn update_character_rotation(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    action_state: ActionState,
    movement_state: MovementState,
    character_attributes: &CharacterAttributes,
    action_state_timer: ActionStateTimer,
    move_direction: MoveDirection,
    latlon: LatLon,
) {
    match archetype {
        PlayerArchetype::Player0 => {
            let mut query = world
                .query_one::<&mut (Player0, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player1 => {
            let mut query = world
                .query_one::<&mut (Player1, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player2 => {
            let mut query = world
                .query_one::<&mut (Player2, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player3 => {
            let mut query = world
                .query_one::<&mut (Player3, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player4 => {
            let mut query = world
                .query_one::<&mut (Player4, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player5 => {
            let mut query = world
                .query_one::<&mut (Player5, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player6 => {
            let mut query = world
                .query_one::<&mut (Player6, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player7 => {
            let mut query = world
                .query_one::<&mut (Player7, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player8 => {
            let mut query = world
                .query_one::<&mut (Player8, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
        PlayerArchetype::Player9 => {
            let mut query = world
                .query_one::<&mut (Player9, ToParentTrans)>(entity)
                .expect("invalid entity");
            let (_, local_transform) = query.get().expect("invalid entity component!");
            update_character_rotation_inner(
                local_transform,
                action_state,
                movement_state,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            );
        }
    }
}

/// 플레이어 캐릭터가 바라보는 방향을 갱신합니다.
///
/// # Note
/// 이 함수를 호출하기 전에 플레이어 상태가 갱신되어야 합니다.
///
fn update_character_rotation_inner(
    local_transform: &mut ToParentTrans,
    action_state: ActionState,
    movement_state: MovementState,
    character_attributes: &CharacterAttributes,
    action_state_timer: ActionStateTimer,
    move_direction: MoveDirection,
    latlon: LatLon,
) {
    match movement_state {
        MovementState::Idle
        | MovementState::MoveToEnd
        | MovementState::Jumping
        | MovementState::Landing => match action_state {
            ActionState::Aiming => set_rotation_to_camera(
                local_transform,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            ),
            ActionState::AimAt => set_rotation_to_camera_from_current(
                local_transform,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            ),
            ActionState::Attack => set_rotation_to_camera(
                local_transform,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            ),
            ActionState::Skill => set_rotation_to_camera(
                local_transform,
                character_attributes,
                action_state_timer,
                move_direction,
                latlon,
            ),
            _ => {}
        },
        MovementState::Moving | MovementState::Jumping | MovementState::Landing => {
            match action_state {
                ActionState::Idle => set_rotation_to_movement(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                ActionState::Aiming => set_rotation_to_camera(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                ActionState::AimAt => set_rotation_to_camera_from_current(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                ActionState::AimOff => set_rotation_to_current_from_camera(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                ActionState::Attack => set_rotation_to_camera(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                ActionState::Reload => set_rotation_to_camera(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                ActionState::Skill => set_rotation_to_camera(
                    local_transform,
                    character_attributes,
                    action_state_timer,
                    move_direction,
                    latlon,
                ),
                _ => {}
            }
        }
    }
}

/// 플레이어 캐릭터 방향을 이동 방향과 일치시킵니다.
fn set_rotation_to_movement(
    local_transform: &mut ToParentTrans,
    _character_attributes: &CharacterAttributes,
    _action_state_timer: ActionStateTimer,
    move_direction: MoveDirection,
    _latlon: LatLon,
) {
    // 현재 캐릭터의 방향을 가져옵니다.
    let look = local_transform.get_look_vector();

    // 두 방향을 각도에 따라 선형 보간합니다.
    let dir = look.lerp(move_direction.0, 0.5);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(dir, glam::Vec3::Y);
}

/// 캐릭터 방향을 카메라가 바라보는 방향으로 전환합니다.
fn set_rotation_to_camera_from_current(
    local_transform: &mut ToParentTrans,
    character_attributes: &CharacterAttributes,
    action_state_timer: ActionStateTimer,
    _move_direction: MoveDirection,
    latlon: LatLon,
) {
    // 삼인칭 카메라의 방향을 계산합니다.
    let rotation = glam::Mat4::from_rotation_y(latlon.lon.to_f32());
    let mat = local_transform.0 * rotation;
    let look = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let duration = character_attributes.normal_attack_start_duration;
    let s = action_state_timer.0 as f32 / duration as f32;
    let look = look.lerp(direction, s).normalize_or(look);

    // 월드 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec3::Y);
}

/// 캐릭터 방향을 자유 방향으로 전환합니다.
fn set_rotation_to_current_from_camera(
    local_transform: &mut ToParentTrans,
    character_attributes: &CharacterAttributes,
    action_state_timer: ActionStateTimer,
    move_direction: MoveDirection,
    _latlon: LatLon,
) {
    // 캐릭터의 방향을 가져옵니다.
    let look = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let duration = character_attributes.normal_attack_end_duration;
    let s = action_state_timer.0 as f32 / duration as f32;
    let look = move_direction
        .0
        .lerp(look, s)
        .normalize_or(move_direction.0);

    // 월드 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec3::Y);
}

/// 캐릭터 방향을 카메라가 바라보는 방향과 일치시킵니다.
fn set_rotation_to_camera(
    local_transform: &mut ToParentTrans,
    _character_attributes: &CharacterAttributes,
    _action_state_timer: ActionStateTimer,
    _move_direction: MoveDirection,
    latlon: LatLon,
) {
    // 삼인칭 카메라의 방향을 계산합니다.
    let mat = glam::Mat4::from_rotation_y(latlon.lon.to_f32());
    let look = glam::Vec3A::from_vec4(mat.z_axis).normalize_or(glam::Vec3A::Z);

    // 캐릭터의 방향을 가져옵니다.
    let direction = local_transform.get_look_vector();

    // 선형 보간된 방향을 계산합니다.
    let look = look.lerp(direction, 0.1).normalize_or(look);

    // 로컬 변환 행렬을 갱신합니다.
    local_transform.look_to(look, glam::Vec3::Y);
}
