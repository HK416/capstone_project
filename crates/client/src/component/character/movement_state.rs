use mod_network::components::{
    ActionState, CharacterAttributes, MovementState, MovementStateTimer, MAX_JUMP_DURATION,
};

/// [`MovementState`]에 따라 [`ActionStateTimer`]를 갱신합니다.
///
/// [`MovementState`]가 변경될 경우 변경된 [`MovementState`]와 경과 시간을 반환합니다.
///
pub fn update_movement_state_timer(
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle => update_movement_state_timer_when_idle(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Moving => update_movement_state_timer_when_moving(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MoveToEnd => update_movement_state_timer_when_move_to_end(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::InPlaceJumping => update_movement_state_timer_when_in_place_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::InPlaceLanding => update_movement_state_timer_when_in_place_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MovingJumping => update_movement_state_timer_when_moving_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MovingLanding => update_movement_state_timer_when_moving_landing(
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
            MovementState::Idle => update_movement_state_timer_when_idle(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::Moving => update_movement_state_timer_when_walking(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MoveToEnd => update_movement_state_timer_when_move_to_end(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::InPlaceJumping => update_movement_state_timer_when_in_place_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::InPlaceLanding => update_movement_state_timer_when_in_place_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MovingJumping => update_movement_state_timer_when_moving_jumping(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            MovementState::MovingLanding => update_movement_state_timer_when_moving_landing(
                movement_state,
                movement_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
        },
        _ => None,
    }
}

/// [`MovementState::Idle`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_idle(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    let duration = character_attributes.normal_idle_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;

    None
}

/// [`MovementState::Moving`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_moving(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    let duration = character_attributes.move_ing_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;

    None
}

/// [`MovementState::MoveToEnd`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_move_to_end(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    let duration = character_attributes.move_end_normal_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *movement_state = MovementState::Idle;
        movement_state_timer.0 = diff_t as u16;

        Some((MovementState::Idle, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Idle`]이 아니고, [`MovementState::Moving`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_walking(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    let duration = character_attributes.cafe_walk_duration;
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms) % duration;

    None
}

/// [`MovementState::InPlaceJumping`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_in_place_jumping(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - MAX_JUMP_DURATION as i32;
    if diff_t >= 0 {
        *movement_state = MovementState::InPlaceLanding;
        movement_state_timer.0 = 0;

        Some((
            MovementState::InPlaceLanding,
            elapsed_time_ms - diff_t as u16,
        ))
    } else {
        None
    }
}

/// [`MovementState::InPlaceLanding`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_in_place_landing(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    movement_state_timer.0 =
        (movement_state_timer.0.saturating_add(elapsed_time_ms)).min(MAX_JUMP_DURATION);

    None
}

/// [`MovementState::MovingJumping`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_moving_jumping(
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    movement_state_timer.0 = movement_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = movement_state_timer.0 as i32 - MAX_JUMP_DURATION as i32;
    if diff_t >= 0 {
        *movement_state = MovementState::MovingLanding;
        movement_state_timer.0 = 0;

        Some((
            MovementState::MovingLanding,
            elapsed_time_ms - diff_t as u16,
        ))
    } else {
        None
    }
}

/// [`MovementState::MovingLanding`]일 때 [`MovementStateTimer`]를 갱신합니다.
pub fn update_movement_state_timer_when_moving_landing(
    _movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(MovementState, u16)> {
    movement_state_timer.0 =
        (movement_state_timer.0.saturating_add(elapsed_time_ms)).min(MAX_JUMP_DURATION);

    None
}
