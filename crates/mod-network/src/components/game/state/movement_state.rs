//! 플레이어 움직임 상태와 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionState, CharacterAttributes, HeldInput, MovementState, MovementStateTimer,
    MAX_JUMP_DURATION,
};

pub fn update_movement_state(
    held_input: HeldInput,
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
) {
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => {
                update_state_when_idle(held_input, movement_state, movement_state_timer)
            }
            MovementState::Moving => {
                update_state_when_moving(held_input, movement_state, movement_state_timer)
            }
            MovementState::MoveToEnd => {
                update_state_when_move_to_end(held_input, movement_state, movement_state_timer)
            }
            _ => {}
        },
        ActionState::Aiming
        | ActionState::AimAt
        | ActionState::AimOff
        | ActionState::Attack
        | ActionState::Reload
        | ActionState::Skill => match movement_state {
            MovementState::Idle => {
                update_state_when_idle(held_input, movement_state, movement_state_timer)
            }
            MovementState::Moving => {
                update_state_when_walking(held_input, movement_state, movement_state_timer)
            }
            MovementState::MoveToEnd => {
                update_state_when_move_to_end(held_input, movement_state, movement_state_timer)
            }
            _ => {}
        },
        ActionState::Retreat
        | ActionState::Callsign
        | ActionState::VictoryStart
        | ActionState::VictoryEnd => {}
    }
}

/// [`MovementState::Idle`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_idle(
    held_input: HeldInput,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
) {
    if held_input.is_moved() {
        *movement_state = MovementState::Moving;
        movement_state_timer.0 = 0;
    } else if held_input.contains(HeldInput::Jump) {
        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;
    }
}

/// [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_moving(
    held_input: HeldInput,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
) {
    if !held_input.is_moved() {
        *movement_state = MovementState::MoveToEnd;
        movement_state_timer.0 = 0;
    } else if held_input.contains(HeldInput::Jump) {
        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;
    }
}

/// [`ActionState::Idle`]이 아니고, [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_walking(
    held_input: HeldInput,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
) {
    if !held_input.is_moved() {
        *movement_state = MovementState::Idle;
        movement_state_timer.0 = 0;
    } else if held_input.contains(HeldInput::Jump) {
        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;
    }
}

/// [`MovementState::MoveToEnd`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_move_to_end(
    held_input: HeldInput,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
) {
    if held_input.is_moved() {
        *movement_state = MovementState::Moving;
        movement_state_timer.0 = 0;
    } else if held_input.contains(HeldInput::Jump) {
        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;
    }
}

/// [`MovementState`]에 따라 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer(
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => update_timer_when_idle(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Moving => update_timer_when_moving(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MoveToEnd => update_timer_when_move_to_end(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Jumping => update_timer_when_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Landing => update_timer_when_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
        },
        ActionState::Aiming
        | ActionState::AimAt
        | ActionState::AimOff
        | ActionState::Attack
        | ActionState::Reload
        | ActionState::Skill => match movement_state {
            MovementState::Idle => update_timer_when_idle(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Moving => update_timer_when_walking(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MoveToEnd => update_timer_when_move_to_end(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Jumping => update_timer_when_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Landing => update_timer_when_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
        },
        ActionState::Retreat
        | ActionState::Callsign
        | ActionState::VictoryStart
        | ActionState::VictoryEnd => {}
    }
}

/// [`MovementState::Idle`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_idle(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        movement_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_moving(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.move_ing_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        movement_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`ActionState::Idle``]이 아니고, [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_walking(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.cafe_walk_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        movement_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`MovementState::MoveToEnd`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_move_to_end(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.move_end_normal_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *movement_state = MovementState::Idle;
        let duration = character_attributes.normal_idle_duration;
        movement_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`MovementState::InPlaceJumping`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_jumping(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 움직임 상태를 갱신합니다.
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - MAX_JUMP_DURATION as i32;
    if diff_t >= 0 {
        *movement_state = MovementState::Landing;
        movement_state_timer.0 = MAX_JUMP_DURATION;
    }
}

/// [`MovementState::InPlaceLanding`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_landing(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 움직임 상태를 갱신합니다.
    movement_state_timer.0 = movement_state_timer
        .0
        .saturating_add(elapsed_time_ms)
        .min(MAX_JUMP_DURATION);
}
