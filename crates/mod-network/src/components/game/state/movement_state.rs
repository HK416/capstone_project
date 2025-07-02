//! 플레이어 움직임 상태와 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionState, CharacterAttributes, GameInputBits, MovementState, MovementStateTimer, StateEvent,
    MAX_JUMP_DURATION,
};

pub fn update_movement_state(
    input_bits: GameInputBits,
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    events: &mut Vec<StateEvent>,
) {
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => {
                update_state_when_idle(input_bits, movement_state, movement_state_timer, events)
            }
            MovementState::Moving => {
                update_state_when_moving(input_bits, movement_state, movement_state_timer, events)
            }
            MovementState::MoveToEnd => update_state_when_move_to_end(
                input_bits,
                movement_state,
                movement_state_timer,
                events,
            ),
            _ => {}
        },
        ActionState::Aiming
        | ActionState::AimAt
        | ActionState::AimOff
        | ActionState::Attack
        | ActionState::Reload
        | ActionState::Skill => match movement_state {
            MovementState::Idle => {
                update_state_when_idle(input_bits, movement_state, movement_state_timer, events)
            }
            MovementState::Moving => {
                update_state_when_walking(input_bits, movement_state, movement_state_timer, events)
            }
            MovementState::MoveToEnd => update_state_when_move_to_end(
                input_bits,
                movement_state,
                movement_state_timer,
                events,
            ),
            _ => {}
        },
        ActionState::Death
        | ActionState::Callsign
        | ActionState::VictoryStart
        | ActionState::VictoryEnd => {}
    }
}

/// [`MovementState::Idle`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_idle(
    input_bits: GameInputBits,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    events: &mut Vec<StateEvent>,
) {
    if input_bits.is_moved() {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Moving;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Moving;
        movement_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Jump) {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Jumping;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;

        events.push(event);
    }
}

/// [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_moving(
    input_bits: GameInputBits,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    events: &mut Vec<StateEvent>,
) {
    if !input_bits.is_moved() {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::MoveToEnd;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::MoveToEnd;
        movement_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Jump) {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Jumping;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;

        events.push(event);
    }
}

/// [`ActionState::Idle`]이 아니고, [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_walking(
    input_bits: GameInputBits,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    events: &mut Vec<StateEvent>,
) {
    if !input_bits.is_moved() {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Idle;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Idle;
        movement_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Jump) {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Jumping;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;

        events.push(event);
    }
}

/// [`MovementState::MoveToEnd`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_state_when_move_to_end(
    input_bits: GameInputBits,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    events: &mut Vec<StateEvent>,
) {
    if input_bits.is_moved() {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Moving;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Moving;
        movement_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Jump) {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Jumping;
        let event = StateEvent::ChangeMovementState {
            from,
            to,
            timing: 0,
        };

        *movement_state = MovementState::Jumping;
        movement_state_timer.0 = 0;

        events.push(event);
    }
}

/// [`MovementState`]에 따라 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer(
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<StateEvent>,
) {
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => update_timer_when_idle(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::Moving => update_timer_when_moving(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::MoveToEnd => update_timer_when_move_to_end(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::Jumping => update_timer_when_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::Landing => update_timer_when_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
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
                events,
            ),
            MovementState::Moving => update_timer_when_walking(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::MoveToEnd => update_timer_when_move_to_end(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::Jumping => update_timer_when_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
            MovementState::Landing => update_timer_when_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
                events,
            ),
        },
        ActionState::Death
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
    _events: &mut Vec<StateEvent>,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;
}

/// [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_moving(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<StateEvent>,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.move_ing_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;
}

/// [`ActionState::Idle``]이 아니고, [`MovementState::Moving`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_walking(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<StateEvent>,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.cafe_walk_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;
}

/// [`MovementState::MoveToEnd`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_move_to_end(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<StateEvent>,
) {
    // 움직임 상태를 갱신합니다.
    let duration = character_attributes.move_ing_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Idle;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeMovementState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *movement_state = MovementState::Idle;
        movement_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`MovementState::InPlaceJumping`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_jumping(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<StateEvent>,
) {
    // 움직임 상태를 갱신합니다.
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - MAX_JUMP_DURATION as i32;
    if diff_t >= 0 {
        // 움직임 상태를 변경합니다.
        let from = movement_state.clone();
        let to = MovementState::Landing;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeMovementState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *movement_state = MovementState::Landing;
        movement_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`MovementState::InPlaceLanding`]일 때 [`MovementState`]와 [`MovementStateTimer`]를 갱신합니다.
fn update_timer_when_landing(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<StateEvent>,
) {
    // 움직임 상태를 갱신합니다.
    movement_state_timer.0 = movement_state_timer
        .0
        .saturating_add(elapsed_time_ms)
        .min(MAX_JUMP_DURATION);
}
