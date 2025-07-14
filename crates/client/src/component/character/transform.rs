use hecs::{Entity, World};
use mod_network::components::{
    update_player_rotation, ActionState, LatLon, MovementState, MovingDirection,
};

use crate::component::{
    Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8, Player9,
    PlayerArchetype, ToParentTrans,
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
    direction: MovingDirection,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
                direction,
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
    direction: MovingDirection,
    latlon: LatLon,
) {
    let mut look = local_transform.get_look_vector();
    // look = update_player_rotation(look, action_state, movement_state, direction, latlon);
    local_transform.look_to(look, glam::Vec3::Y);
}
