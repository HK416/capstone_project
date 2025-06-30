use mod_network::components::{
    ActionState, ActionStateTimer, BulletData, CharacterAttributes, GameInputBits, SkillCostData,
    RESPAWN_DELAY,
};

/// [`ActionState`]와 입력 상태에 따라 [`ActionState`]를 갱신합니다.
///
/// [`ActionState`]가 변경될 경우 변경된 [`ActionState`]를 반환합니다.
///
pub fn update_action_state(
    input_flags: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &BulletData,
    skill_cost_data: &SkillCostData,
) -> Option<ActionState> {
    match action_state {
        ActionState::Idle => update_state_when_idle(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
        ),
        ActionState::Aiming => update_state_when_aiming(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
        ),
        ActionState::AimAt => update_state_when_aim_at(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
        ),
        ActionState::AimOff => update_state_when_aim_off(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
        ),
        _ => None,
    }
}

/// [`ActionState::Idle`]일 때 입력 상태에 따라 [`ActionState`]를 갱신합니다.
fn update_state_when_idle(
    input_flags: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &BulletData,
    skill_cost_data: &SkillCostData,
) -> Option<ActionState> {
    if input_flags.contains(GameInputBits::Skill)
        && skill_cost_data.remaining >= character_attributes.skill_cost
    {
        Some(ActionState::Skill)
    } else if input_flags.contains(GameInputBits::Attack) && bullet_data.remaining > 0 {
        Some(ActionState::Attack)
    } else if input_flags.contains(GameInputBits::Reload) {
        *action_state = ActionState::Reload;
        action_state_timer.0 = 0;
        Some(ActionState::Reload)
    } else if input_flags.contains(GameInputBits::Aiming) {
        *action_state = ActionState::AimAt;
        action_state_timer.0 = 0;
        Some(ActionState::AimAt)
    } else {
        None
    }
}

/// [`ActionState::Aiming`]일 때 입력 상태에 따라 [`ActionState`]를 갱신합니다.
fn update_state_when_aiming(
    input_flags: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &BulletData,
    skill_cost_data: &SkillCostData,
) -> Option<ActionState> {
    if input_flags.contains(GameInputBits::Skill)
        && skill_cost_data.remaining >= character_attributes.skill_cost
    {
        Some(ActionState::Skill)
    } else if input_flags.contains(GameInputBits::Attack) && bullet_data.remaining > 0 {
        Some(ActionState::Attack)
    } else if !input_flags.contains(GameInputBits::Aiming) {
        *action_state = ActionState::AimOff;
        action_state_timer.0 = 0;
        Some(ActionState::AimOff)
    } else {
        None
    }
}

/// [`ActionState::AimAt`]일 때 입력 상태에 따라 [`ActionState`]를 갱신합니다.
fn update_state_when_aim_at(
    input_flags: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _bullet_data: &BulletData,
    _skill_cost_data: &SkillCostData,
) -> Option<ActionState> {
    if !input_flags.contains(GameInputBits::Aiming) {
        *action_state = ActionState::AimOff;

        let aim_at_duration = character_attributes.normal_attack_start_duration;
        let aim_off_duration = character_attributes.normal_attack_end_duration;
        let s = action_state_timer.0 as f32 / aim_at_duration as f32;
        let t = (1.0 - s) * aim_off_duration as f32;
        action_state_timer.0 = t.floor() as u16;
        Some(ActionState::AimAt)
    } else {
        None
    }
}

/// [`ActionState::AimOff`]일 때 입력 상태에 따라 [`ActionState`]를 갱신합니다.
fn update_state_when_aim_off(
    input_flags: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _bullet_data: &BulletData,
    _skill_cost_data: &SkillCostData,
) -> Option<ActionState> {
    if input_flags.contains(GameInputBits::Aiming) {
        *action_state = ActionState::AimAt;

        let aim_at_duration = character_attributes.normal_attack_start_duration;
        let aim_off_duration = character_attributes.normal_attack_end_duration;
        let s = action_state_timer.0 as f32 / aim_off_duration as f32;
        let t = (1.0 - s) * aim_at_duration as f32;
        action_state_timer.0 = t.floor() as u16;
        Some(ActionState::AimAt)
    } else {
        None
    }
}

/// [`ActionState`]에 따라 [`ActionStateTimer`]를 갱신합니다.
///
/// [`ActionState`]가 변경될 경우 변경된 [`ActionState`]와 경과 시간을 반환합니다.
///
pub fn update_action_state_timer(
    input_flags: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    match action_state {
        ActionState::Idle => update_timer_when_idle(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Aiming => update_timer_when_aiming(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::AimAt => update_timer_when_aim_at(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::AimOff => update_timer_when_aim_off(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Attack => update_timer_when_attack(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Death => update_timer_when_death(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Reload => update_timer_when_reload(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Skill => update_timer_when_skill(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Callsign => update_timer_when_callsign(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::VictoryStart => update_timer_when_victory_start(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::VictoryEnd => update_timer_when_victory_end(
            input_flags,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
    }
}

/// [`ActionState::Idle`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_idle(
    _input_flags: GameInputBits,
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
fn update_timer_when_aiming(
    _input_flags: GameInputBits,
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
fn update_timer_when_aim_at(
    _input_flags: GameInputBits,
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
        *action_state = ActionState::Aiming;
        action_state_timer.0 = diff_t as u16;

        Some((ActionState::Aiming, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::AimOff`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_aim_off(
    _input_flags: GameInputBits,
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
        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16 % duration;

        Some((ActionState::Idle, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Attack`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_attack(
    input_flags: GameInputBits,
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
        let duration = character_attributes.normal_idle_duration;
        if input_flags.contains(GameInputBits::Aiming) {
            *action_state = ActionState::Aiming;
            action_state_timer.0 = diff_t as u16 % duration;
            Some((ActionState::Aiming, elapsed_time_ms - diff_t as u16))
        } else {
            *action_state = ActionState::Idle;
            action_state_timer.0 = diff_t as u16 % duration;
            Some((ActionState::Idle, elapsed_time_ms - diff_t as u16))
        }
    } else {
        None
    }
}

/// [`ActionState::Death`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_death(
    _input_flags: GameInputBits,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    _character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) -> Option<(ActionState, u16)> {
    // 타이머를 갱신합니다.
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms).min(RESPAWN_DELAY);
    None
}

/// [`ActionState::Reload`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_reload(
    _input_flags: GameInputBits,
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
        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16 % duration;

        Some((ActionState::Idle, elapsed_time_ms - diff_t as u16))
    } else {
        None
    }
}

/// [`ActionState::Skill`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_skill(
    input_flags: GameInputBits,
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
        let duration = character_attributes.normal_idle_duration;
        if input_flags.contains(GameInputBits::Aiming) {
            *action_state = ActionState::Aiming;
            action_state_timer.0 = diff_t as u16 % duration;
            Some((ActionState::Aiming, elapsed_time_ms - diff_t as u16))
        } else {
            *action_state = ActionState::Idle;
            action_state_timer.0 = diff_t as u16 % duration;
            Some((ActionState::Idle, elapsed_time_ms - diff_t as u16))
        }
    } else {
        None
    }
}

/// [`ActionState::Callsign`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_callsign(
    _input_flags: GameInputBits,
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
        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16 % duration;
    }

    None
}

/// [`ActionState::VictoryStart`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_victory_start(
    _input_flags: GameInputBits,
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
        let duration = character_attributes.victory_end_duration;
        *action_state = ActionState::VictoryEnd;
        action_state_timer.0 = (diff_t as u16) % duration;
    }

    None
}

/// [`ActionState::VictoryEnd`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_timer_when_victory_end(
    _input_flags: GameInputBits,
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
