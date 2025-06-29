use mod_network::components::{ActionState, ActionStateTimer, CharacterAttributes};

/// [`ActionState`]에 따라 [`ActionStateTimer`]를 갱신합니다.
///
/// [`ActionState`]가 변경될 경우 변경된 [`ActionState`]와 경과 시간을 반환합니다.
///
pub fn update_action_state_timer(
    prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    match action_state {
        ActionState::Idle => update_action_state_timer_when_idle(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Aiming => update_action_state_timer_when_aiming(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::AimAt => update_action_state_timer_when_aim_at(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::AimOff => update_action_state_timer_when_aim_off(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Attack => update_action_state_timer_when_attack(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Death => update_action_state_timer_when_death(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Reload => update_action_state_timer_when_reload(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Skill => update_action_state_timer_when_skill(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Callsign => update_action_state_timer_when_callsign(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::VictoryStart => update_action_state_timer_when_victory_start(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::VictoryEnd => update_action_state_timer_when_victory_end(
            prev_action_state,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
    }
}

/// [`ActionState::Idle`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_idle(
    _prev_action_state: &mut ActionState,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms) % duration;

    None
}

/// [`ActionState::Aiming`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aiming(
    _prev_action_state: &mut ActionState,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms) % duration;

    None
}

/// [`ActionState::AimAt`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aim_at(
    prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_start_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *prev_action_state = ActionState::AimAt;
        *action_state = ActionState::Aiming;
        action_state_timer.0 = diff_t as u16;

        Some((ActionState::Aiming, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::AimOff`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aim_off(
    prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_end_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *prev_action_state = ActionState::AimOff;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16;

        Some((ActionState::Idle, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Attack`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_attack(
    prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_ing_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        let state = *prev_action_state;
        *prev_action_state = ActionState::Attack;
        *action_state = state;
        action_state_timer.0 = diff_t as u16;

        Some((state, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Death`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_death(
    _prev_action_state: &mut ActionState,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.vital_death_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms).min(duration);

    None
}

/// [`ActionState::Reload`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_reload(
    prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_reload_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        let state = *prev_action_state;
        *prev_action_state = ActionState::Reload;
        *action_state = state;
        action_state_timer.0 = diff_t as u16;

        Some((ActionState::Reload, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Skill`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_skill(
    prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.skill_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        let state = *prev_action_state;
        *prev_action_state = ActionState::Skill;
        *action_state = state;
        action_state_timer.0 = diff_t as u16;

        Some((state, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Callsign`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_callsign(
    _prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_callsign_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16;
    }

    None
}

/// [`ActionState::VictoryStart`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_victory_start(
    _prev_action_state: &mut ActionState,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.victory_start_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *action_state = ActionState::VictoryEnd;
        action_state_timer.0 = diff_t as u16;
    }

    None
}

/// [`ActionState::VictoryEnd`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_victory_end(
    _prev_action_state: &mut ActionState,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    let duration = character_attributes.victory_end_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms) % duration;

    None
}
