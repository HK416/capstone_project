use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, GameInputBits, PlayerStateData,
};

/// 행동 상태의 개수입니다.
pub const NUM_ACTION_STATES: usize = 11;

/// [`ActionState`]일 때 [`ActionStateTimer`]를 갱신합니다.
pub fn update_action_state_timer(
    input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    type Func =
        fn(GameInputBits, &mut PlayerStateData, &mut ActionStateTimer, &CharacterAttributes, u16);
    const FUNC: [Func; NUM_ACTION_STATES] = [
        update_action_state_timer_when_idle,
        update_action_state_timer_when_aiming,
        update_action_state_timer_when_aim_at,
        update_action_state_timer_when_aim_off,
        update_action_state_timer_when_attack,
        update_action_state_timer_when_death,
        update_action_state_timer_when_reload,
        update_action_state_timer_when_skill,
        update_action_state_timer_when_callsign,
        update_action_state_timer_when_victory_start,
        update_action_state_timer_when_victory_end,
    ];

    let i = player_states.action_state() as usize;
    FUNC[i](
        input_flags,
        player_states,
        action_state_timer,
        character_attributes,
        elapsed_time_ms,
    );
}

/// [`ActionState::Idle`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_idle(
    _input_flags: GameInputBits,
    _player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms) % duration;
}

/// [`ActionState::Aiming`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aiming(
    _input_flags: GameInputBits,
    _player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms) % duration;
}

/// [`ActionState::AimAt`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aim_at(
    _input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_start_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        player_states.set_action_state(ActionState::Aiming);
        action_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::AimOff`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aim_off(
    _input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_end_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        player_states.set_action_state(ActionState::Idle);
        action_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::Attack`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_attack(
    input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_ing_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        action_state_timer.0 = diff_t as u16;
        if input_flags.contains(GameInputBits::Skill) {
            player_states.set_action_state(ActionState::Skill);
        } else if input_flags.contains(GameInputBits::Attack) {
            player_states.set_action_state(ActionState::Attack);
        } else if input_flags.contains(GameInputBits::Reload) {
            player_states.set_action_state(ActionState::Reload);
        } else if input_flags.contains(GameInputBits::Aiming) {
            player_states.set_action_state(ActionState::Aiming);
        } else {
            player_states.set_action_state(ActionState::Idle);
        }
    }
}

/// [`ActionState::Death`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_death(
    _input_flags: GameInputBits,
    _player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.vital_death_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms).min(duration);
}

/// [`ActionState::Reload`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_reload(
    input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_reload_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        action_state_timer.0 = diff_t as u16;
        if input_flags.contains(GameInputBits::Skill) {
            player_states.set_action_state(ActionState::Skill);
        } else if input_flags.contains(GameInputBits::Attack) {
            player_states.set_action_state(ActionState::Attack);
        } else if input_flags.contains(GameInputBits::Reload) {
            player_states.set_action_state(ActionState::Reload);
        } else if input_flags.contains(GameInputBits::Aiming) {
            player_states.set_action_state(ActionState::Aiming);
        } else {
            player_states.set_action_state(ActionState::Idle);
        }
    }
}

/// [`ActionState::Skill`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_skill(
    input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.skill_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        action_state_timer.0 = diff_t as u16;
        if input_flags.contains(GameInputBits::Skill) {
            player_states.set_action_state(ActionState::Skill);
        } else if input_flags.contains(GameInputBits::Attack) {
            player_states.set_action_state(ActionState::Attack);
        } else if input_flags.contains(GameInputBits::Reload) {
            player_states.set_action_state(ActionState::Reload);
        } else if input_flags.contains(GameInputBits::Aiming) {
            player_states.set_action_state(ActionState::Aiming);
        } else {
            player_states.set_action_state(ActionState::Idle);
        }
    }
}

/// [`ActionState::Callsign`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_callsign(
    _input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.normal_callsign_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        player_states.set_action_state(ActionState::Idle);
        action_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::VictoryStart`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_victory_start(
    _input_flags: GameInputBits,
    player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.victory_start_duration;
    action_state_timer.0 = action_state_timer.0 + elapsed_time_ms;

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        player_states.set_action_state(ActionState::VictoryEnd);
        action_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::VictoryEnd`]일 때 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_victory_end(
    _input_flags: GameInputBits,
    _player_states: &mut PlayerStateData,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 타이머를 갱신합니다.
    let duration = character_attributes.victory_end_duration;
    action_state_timer.0 = (action_state_timer.0 + elapsed_time_ms) % duration;
}
