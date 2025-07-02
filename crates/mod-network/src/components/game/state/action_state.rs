//! 플레이어 행동 상태와 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionState, ActionStateTimer, BulletData, CharacterAttributes, GameInputBits, SkillCostData,
    StateEvent, RESPAWN_DELAY,
};

/// [`ActionState`]에 따라 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
pub fn update_action_state(
    input_bits: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    events: &mut Vec<StateEvent>,
) {
    match action_state {
        ActionState::Idle => update_state_when_idle(
            input_bits,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::Aiming => update_state_when_aiming(
            input_bits,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::AimAt => update_state_when_aim_at(
            input_bits,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::AimOff => update_state_when_aim_off(
            input_bits,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::Attack
        | ActionState::Death
        | ActionState::Reload
        | ActionState::Skill
        | ActionState::Callsign
        | ActionState::VictoryStart
        | ActionState::VictoryEnd => {}
    }
}

/// [`ActionState::Idle`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_idle(
    input_bits: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    events: &mut Vec<StateEvent>,
) {
    if input_bits.contains(GameInputBits::Skill)
        && skill_cost_data.remaining >= character_attributes.skill_cost
    {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Skill;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::Skill;
        action_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Attack) && bullet_data.remaining > 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Attack;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::Attack;
        action_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Reload) {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Reload;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::Reload;
        action_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Aiming) {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::AimAt;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::AimAt;
        action_state_timer.0 = 0;

        events.push(event);
    }
}

/// [`ActionState::Aiming`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_aiming(
    input_bits: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    events: &mut Vec<StateEvent>,
) {
    if input_bits.contains(GameInputBits::Skill)
        && skill_cost_data.remaining >= character_attributes.skill_cost
    {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Skill;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::Skill;
        action_state_timer.0 = 0;

        events.push(event);
    } else if input_bits.contains(GameInputBits::Attack) && bullet_data.remaining > 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Attack;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::Attack;
        action_state_timer.0 = 0;

        events.push(event);
    } else if !input_bits.contains(GameInputBits::Aiming) {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::AimOff;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::AimOff;
        action_state_timer.0 = 0;

        events.push(event);
    }
}

/// [`ActionState::AimAt`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_aim_at(
    input_bits: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    events: &mut Vec<StateEvent>,
) {
    if !input_bits.contains(GameInputBits::Aiming) {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::AimOff;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::AimOff;
        let aim_at_duration = character_attributes.normal_attack_start_duration;
        let aim_off_duration = character_attributes.normal_attack_end_duration;
        let s = action_state_timer.0 as f32 / aim_at_duration as f32;
        let t = (1.0 - s) * aim_off_duration as f32;
        action_state_timer.0 = t.floor() as u16;

        events.push(event);
    }
}

/// [`ActionState::AimOff`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_aim_off(
    input_bits: GameInputBits,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    events: &mut Vec<StateEvent>,
) {
    if input_bits.contains(GameInputBits::Aiming) {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::AimAt;
        let event = StateEvent::ChangeActionState {
            from,
            to,
            timing: 0,
        };

        *action_state = ActionState::AimAt;
        let aim_at_duration = character_attributes.normal_attack_start_duration;
        let aim_off_duration = character_attributes.normal_attack_end_duration;
        let s = action_state_timer.0 as f32 / aim_off_duration as f32;
        let t = (1.0 - s) * aim_at_duration as f32;
        action_state_timer.0 = t.floor() as u16;

        events.push(event);
    }
}

/// [`ActionState`]에 따라 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
pub fn update_action_state_timer(
    input_bits: GameInputBits,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<StateEvent>,
) {
    match action_state {
        ActionState::Idle => update_timer_when_idle(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Aiming => update_timer_when_aiming(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::AimAt => update_timer_when_aim_at(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::AimOff => update_timer_when_aim_off(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::Attack => update_timer_when_attack(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::Death => update_timer_when_death(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::Reload => update_timer_when_reload(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::Skill => update_timer_when_skill(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::Callsign => update_timer_when_callsign(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::VictoryStart => update_timer_when_victory_start(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
        ActionState::VictoryEnd => update_timer_when_victory_end(
            input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            events,
            elapsed_time_ms,
        ),
    }
}

/// [`ActionState::Idle`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_idle(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<StateEvent>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms) % duration;
}

/// [`ActionState::Aiming`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_aiming(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<StateEvent>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms) % duration;
}

/// [`ActionState::AimAt`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_aim_at(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_start_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Aiming;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeActionState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Aiming;
        action_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`ActionState::AimOff`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_aim_off(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_end_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Idle;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeActionState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`ActionState::Attack`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_attack(
    input_bits: GameInputBits,
    bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 다음 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_ing_duration;
    let next_timer = action_state_timer.0.saturating_add(elapsed_time_ms);
    let diff_t = next_timer as i32 - duration as i32;

    let timings = &character_attributes.normal_attack_timing;
    let mut index = bullet_data.fires_per_attack as usize;
    while let Some(timing) = timings.get(index).cloned()
        && timing <= next_timer
        && bullet_data.remaining > 0
    {
        // 총알 발사 이벤트를 생성합니다.
        let timing = timing.saturating_sub(action_state_timer.0);
        events.push(StateEvent::BulletFired { timing });

        bullet_data.fires_per_attack += 1;
        bullet_data.remaining -= 1;
        index = bullet_data.fires_per_attack as usize;
    }

    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        if input_bits.contains(GameInputBits::Aiming) {
            let from = action_state.clone();
            let to = ActionState::Aiming;
            let timing = elapsed_time_ms - diff_t as u16;
            let event = StateEvent::ChangeActionState { from, to, timing };

            let duration = character_attributes.normal_idle_duration;
            *action_state = ActionState::Aiming;
            action_state_timer.0 = diff_t as u16 % duration;

            events.push(event);
        } else {
            let from = action_state.clone();
            let to = ActionState::Idle;
            let timing = elapsed_time_ms - diff_t as u16;
            let event = StateEvent::ChangeActionState { from, to, timing };

            let duration = character_attributes.normal_idle_duration;
            *action_state = ActionState::Idle;
            action_state_timer.0 = diff_t as u16 % duration;

            events.push(event);
        }
    }

    action_state_timer.0 = next_timer;
}

/// [`ActionState::Death`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_death(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - RESPAWN_DELAY as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Idle;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeActionState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`ActionState::Reload`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_reload(
    input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_reload_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        if input_bits.contains(GameInputBits::Aiming) {
            let from = action_state.clone();
            let to = ActionState::Aiming;
            let timing = elapsed_time_ms - diff_t as u16;
            let event = StateEvent::ChangeActionState { from, to, timing };

            let duration = character_attributes.normal_idle_duration;
            *action_state = ActionState::Aiming;
            action_state_timer.0 = diff_t as u16 % duration;

            events.push(event);
        } else {
            let from = action_state.clone();
            let to = ActionState::Idle;
            let timing = elapsed_time_ms - diff_t as u16;
            let event = StateEvent::ChangeActionState { from, to, timing };

            let duration = character_attributes.normal_idle_duration;
            *action_state = ActionState::Idle;
            action_state_timer.0 = diff_t as u16 % duration;

            events.push(event);
        }
    }
}

/// [`ActionState::Skill`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_skill(
    input_bits: GameInputBits,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
}

/// [`ActionState::Callsign`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_callsign(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_callsign_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::Idle;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeActionState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::Idle;
        action_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`ActionState::VictoryStart`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_victory_start(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.victory_start_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        let from = action_state.clone();
        let to = ActionState::VictoryEnd;
        let timing = elapsed_time_ms - diff_t as u16;
        let event = StateEvent::ChangeActionState { from, to, timing };

        let duration = character_attributes.normal_idle_duration;
        *action_state = ActionState::VictoryEnd;
        action_state_timer.0 = diff_t as u16 % duration;

        events.push(event);
    }
}

/// [`ActionState::VictoryEnd`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_victory_end(
    _input_bits: GameInputBits,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _events: &mut Vec<StateEvent>,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.victory_end_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms) % duration;
}
