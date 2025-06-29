//! 플레이어 움직임 상태와 관련된 코드를 관리합니다.
//!

use mod_network::components::{ActionState, MAX_JUMP_DURATION, MovementState};

use crate::entities::Player;

/// 플레이어의 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer(data: &mut Player, elapsed_time_ms: u16) {
    let action_state = data.player_states.action_state();
    let movement_state = data.player_states.movement_state();
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => update_movement_state_timer_when_idle(data, elapsed_time_ms),
            MovementState::Moving => update_movement_state_timer_when_moving(data, elapsed_time_ms),
            MovementState::MoveToEnd => {
                update_movement_state_timer_when_move_to_end(data, elapsed_time_ms)
            }
            MovementState::InPlaceJumping => {
                update_movement_state_timer_when_in_place_jumping(data, elapsed_time_ms)
            }
            MovementState::InPlaceLanding => {
                update_movement_state_timer_when_in_place_landing(data, elapsed_time_ms)
            }
            MovementState::MovingJumping => {
                update_movement_state_timer_when_moving_jumping(data, elapsed_time_ms);
            }
            MovementState::MovingLanding => {
                update_movement_state_timer_when_moving_landing(data, elapsed_time_ms);
            }
        },
        ActionState::Aiming
        | ActionState::AimAt
        | ActionState::AimOff
        | ActionState::Attack
        | ActionState::Reload
        | ActionState::Skill => match movement_state {
            MovementState::Idle => update_movement_state_timer_when_idle(data, elapsed_time_ms),
            MovementState::Moving => {
                update_movement_state_timer_when_walking(data, elapsed_time_ms)
            }
            MovementState::MoveToEnd => {
                update_movement_state_timer_when_move_to_end(data, elapsed_time_ms)
            }
            MovementState::InPlaceJumping => {
                update_movement_state_timer_when_in_place_jumping(data, elapsed_time_ms)
            }
            MovementState::InPlaceLanding => {
                update_movement_state_timer_when_in_place_landing(data, elapsed_time_ms)
            }
            MovementState::MovingJumping => {
                update_movement_state_timer_when_moving_jumping(data, elapsed_time_ms);
            }
            MovementState::MovingLanding => {
                update_movement_state_timer_when_moving_landing(data, elapsed_time_ms);
            }
        },
        _ => {}
    }
}

/// [`MovementState::Idle`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_idle(data: &mut Player, elapsed_time_ms: u16) {
    let duration = data.character_attributes().normal_idle_duration;
    data.movement_state_timer.0 =
        (data.movement_state_timer.0.saturating_add(elapsed_time_ms)) % duration;
}

/// [`MovementState::Moving`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_moving(data: &mut Player, elapsed_time_ms: u16) {
    let duration = data.character_attributes().move_ing_duration;
    data.movement_state_timer.0 =
        (data.movement_state_timer.0.saturating_add(elapsed_time_ms)) % duration;
}

/// [`MovementState::MoveToEnd`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_move_to_end(data: &mut Player, elapsed_time_ms: u16) {
    let duration = data.character_attributes().move_end_normal_duration;
    data.movement_state_timer.0 = data.movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        data.player_states.set_movement_state(MovementState::Idle);
        data.movement_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::Idle`]이 아니고, [`MovementState::Moving`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_walking(data: &mut Player, elapsed_time_ms: u16) {
    let duration = data.character_attributes().cafe_walk_duration;
    data.movement_state_timer.0 =
        (data.movement_state_timer.0.saturating_add(elapsed_time_ms)) % duration;
}

/// [`MovementState::InPlaceJumping`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_in_place_jumping(data: &mut Player, elapsed_time_ms: u16) {
    data.movement_state_timer.0 = data.movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.movement_state_timer.0 as i32 - MAX_JUMP_DURATION as i32;
    if diff_t >= 0 {
        data.player_states
            .set_movement_state(MovementState::InPlaceLanding);
        data.movement_state_timer.0 = 0;
    }
}

/// [`MovementState::InPlaceLanding`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_in_place_landing(data: &mut Player, elapsed_time_ms: u16) {
    data.movement_state_timer.0 =
        (data.movement_state_timer.0.saturating_add(elapsed_time_ms)).min(MAX_JUMP_DURATION);
}

/// [`MovementState::MovingJumping`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_moving_jumping(data: &mut Player, elapsed_time_ms: u16) {
    data.movement_state_timer.0 = data.movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.movement_state_timer.0 as i32 - MAX_JUMP_DURATION as i32;
    if diff_t >= 0 {
        data.player_states
            .set_movement_state(MovementState::MovingLanding);
        data.movement_state_timer.0 = 0;
    }
}

/// [`MovementState::MovingLanding`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_moving_landing(data: &mut Player, elapsed_time_ms: u16) {
    data.movement_state_timer.0 =
        (data.movement_state_timer.0.saturating_add(elapsed_time_ms)).min(MAX_JUMP_DURATION);
}
