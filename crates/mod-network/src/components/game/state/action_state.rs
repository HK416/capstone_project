//! 플레이어 행동 상태와 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionEvent, ActionEventDetail, ActionState, ActionStateTimer, BulletData, CharacterAttributes,
    HeldInput, SkillCostData, UserId, RESPAWN_DELAY,
};

/// [`ActionState`]에 따라 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
pub fn update_action_state(
    held_input: HeldInput,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    events: &mut Vec<ActionEvent>,
) {
    match action_state {
        ActionState::Idle => update_state_when_idle(
            held_input,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::Aiming => update_state_when_aiming(
            held_input,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::AimAt => update_state_when_aim_at(
            held_input,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::AimOff => update_state_when_aim_off(
            held_input,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            events,
        ),
        ActionState::Attack
        | ActionState::Retreat
        | ActionState::Reload
        | ActionState::Skill
        | ActionState::Callsign
        | ActionState::VictoryStart
        | ActionState::VictoryEnd => {}
    }
}

/// [`ActionState::Idle`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_idle(
    held_input: HeldInput,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    events: &mut Vec<ActionEvent>,
) {
    if held_input.contains(HeldInput::Skill)
        && skill_cost_data.remaining >= character_attributes.skill_cost
    {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::Skill;
        action_state_timer.0 = 0;

        events.push(ActionEvent::Changed(ActionState::Skill));
    } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::Attack;
        action_state_timer.0 = 0;

        events.push(ActionEvent::Changed(ActionState::Attack));
    } else if held_input.contains(HeldInput::Reload) {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::Reload;
        action_state_timer.0 = 0;

        events.push(ActionEvent::Changed(ActionState::Reload));
    } else if held_input.contains(HeldInput::Aiming) {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::AimAt;
        action_state_timer.0 = 0;
    }
}

/// [`ActionState::Aiming`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_aiming(
    held_input: HeldInput,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    events: &mut Vec<ActionEvent>,
) {
    if held_input.contains(HeldInput::Skill)
        && skill_cost_data.remaining >= character_attributes.skill_cost
    {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::Skill;
        action_state_timer.0 = 0;

        events.push(ActionEvent::Changed(ActionState::Skill));
    } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::Attack;
        action_state_timer.0 = 0;

        events.push(ActionEvent::Changed(ActionState::Attack));
    } else if held_input.contains(HeldInput::Reload) {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::Reload;
        action_state_timer.0 = 0;

        events.push(ActionEvent::Changed(ActionState::Reload));
    } else if !held_input.contains(HeldInput::Aiming) {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::AimOff;
        action_state_timer.0 = 0;
    }
}

/// [`ActionState::AimAt`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_aim_at(
    held_input: HeldInput,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    _events: &mut Vec<ActionEvent>,
) {
    if !held_input.contains(HeldInput::Aiming) {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::AimOff;
        let aim_at_duration = character_attributes.normal_attack_start_duration;
        let aim_off_duration = character_attributes.normal_attack_end_duration;
        let s = action_state_timer.0 as f32 / aim_at_duration as f32;
        let t = (1.0 - s) * aim_off_duration as f32;
        action_state_timer.0 = t.floor() as u16;
    }
}

/// [`ActionState::AimOff`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_state_when_aim_off(
    held_input: HeldInput,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    events: &mut Vec<ActionEvent>,
) {
    if held_input.contains(HeldInput::Aiming) {
        // 행동 상태를 변경합니다.
        *action_state = ActionState::AimAt;
        let aim_at_duration = character_attributes.normal_attack_start_duration;
        let aim_off_duration = character_attributes.normal_attack_end_duration;
        let s = action_state_timer.0 as f32 / aim_off_duration as f32;
        let t = (1.0 - s) * aim_at_duration as f32;
        action_state_timer.0 = t.floor() as u16;

        events.push(ActionEvent::Changed(ActionState::AimAt));
    }
}

/// [`ActionState`]에 따라 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
pub fn update_action_state_timer(
    uid: UserId,
    held_input: HeldInput,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    match action_state {
        ActionState::Idle => update_timer_when_idle(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Aiming => update_timer_when_aiming(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::AimAt => update_timer_when_aim_at(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::AimOff => update_timer_when_aim_off(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Attack => update_timer_when_attack(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Retreat => update_timer_when_death(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Reload => update_timer_when_reload(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Skill => update_timer_when_skill(
            uid,
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            events,
        ),
        ActionState::Callsign => update_timer_when_callsign(
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::VictoryStart => update_timer_when_victory_start(
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::VictoryEnd => update_timer_when_victory_end(
            held_input,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
    }
}

/// [`ActionState::Idle`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_idle(
    _uid: UserId,
    _held_input: HeldInput,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<ActionEventDetail>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        action_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`ActionState::Aiming`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_aiming(
    _uid: UserId,
    _held_input: HeldInput,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    _action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    _events: &mut Vec<ActionEventDetail>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_idle_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        action_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`ActionState::AimAt`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_aim_at(
    uid: UserId,
    held_input: HeldInput,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_start_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        if held_input.contains(HeldInput::Skill)
            && skill_cost_data.remaining >= character_attributes.skill_cost
        {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Skill;
            let duration = character_attributes.skill_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Skill),
            });
        } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Attack;
            let duration = character_attributes.normal_attack_ing_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Attack),
            });
        } else if held_input.contains(HeldInput::Reload) {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Reload;
            let duration = character_attributes.normal_reload_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Reload),
            });
        } else {
            // 행동 상태를 변경합니다.
            *action_state = ActionState::Aiming;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        }
    }
}

/// [`ActionState::AimOff`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_aim_off(
    uid: UserId,
    held_input: HeldInput,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_end_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        if held_input.contains(HeldInput::Skill)
            && skill_cost_data.remaining >= character_attributes.skill_cost
        {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Skill;
            let duration = character_attributes.skill_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Skill),
            });
        } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Attack;
            let duration = character_attributes.normal_attack_ing_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Attack),
            });
        } else if held_input.contains(HeldInput::Reload) {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Reload;
            let duration = character_attributes.normal_reload_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Reload),
            });
        } else {
            // 행동 상태를 변경합니다.
            *action_state = ActionState::Idle;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        }
    }
}

/// [`ActionState::Attack`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_attack(
    uid: UserId,
    held_input: HeldInput,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    // 다음 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_attack_ing_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);
    let diff_t = action_state_timer.0 as i32 - duration as i32;

    let timings = &character_attributes.normal_attack_timing;
    let mut index = bullet_data.fires_per_attack as usize;
    while let Some(timing) = timings.get(index).cloned()
        && timing <= action_state_timer.0
        && bullet_data.remaining > 0
    {
        // 총알 발사 이벤트를 생성합니다.
        let timing = elapsed_time_ms - (action_state_timer.0 - timing);
        events.push(ActionEventDetail {
            uid,
            timing,
            event: ActionEvent::Attack,
        });

        bullet_data.remaining -= 1;
        bullet_data.fires_per_attack += 1;
        index = bullet_data.fires_per_attack as usize;
    }

    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        if held_input.contains(HeldInput::Skill)
            && skill_cost_data.remaining >= character_attributes.skill_cost
        {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Skill;
            let duration = character_attributes.skill_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Skill),
            });
        } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Attack;
            let duration = character_attributes.normal_attack_ing_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Attack),
            });
        } else if held_input.contains(HeldInput::Reload) {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Reload;
            let duration = character_attributes.normal_reload_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Reload),
            });
        } else if held_input.contains(HeldInput::Aiming) {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Aiming;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        } else {
            bullet_data.fires_per_attack = 0;
            *action_state = ActionState::Idle;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        }
    }
}

/// [`ActionState::Death`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_death(
    uid: UserId,
    held_input: HeldInput,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    // 행동 상태 타이머를 갱신합니다.
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - RESPAWN_DELAY as i32;
    if diff_t >= 0 {
        if held_input.contains(HeldInput::Aiming) {
            *action_state = ActionState::Aiming;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Respawn,
            });
        } else {
            *action_state = ActionState::Idle;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Respawn,
            });
        }
    }
}

/// [`ActionState::Reload`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_reload(
    uid: UserId,
    held_input: HeldInput,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_reload_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    if bullet_data.remaining != bullet_data.num_maximum_bullets()
        && action_state_timer.0 >= duration / 2
    {
        let diff_t = duration / 2 - action_state_timer.0;
        let timing = elapsed_time_ms - diff_t as u16;
        events.push(ActionEventDetail {
            uid,
            timing,
            event: ActionEvent::Reload,
        });
    }

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        // 행동 상태를 변경합니다.
        if held_input.contains(HeldInput::Skill)
            && skill_cost_data.remaining >= character_attributes.skill_cost
        {
            *action_state = ActionState::Skill;
            let duration = character_attributes.skill_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Skill),
            });
        } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
            *action_state = ActionState::Attack;
            let duration = character_attributes.normal_attack_ing_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Attack),
            });
        } else if held_input.contains(HeldInput::Reload) {
            *action_state = ActionState::Reload;
            let duration = character_attributes.normal_reload_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Reload),
            });
        } else if held_input.contains(HeldInput::Aiming) {
            *action_state = ActionState::Aiming;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        } else {
            *action_state = ActionState::Idle;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        }
    }
}

/// [`ActionState::Skill`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_skill(
    uid: UserId,
    held_input: HeldInput,
    bullet_data: &mut BulletData,
    skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
    events: &mut Vec<ActionEventDetail>,
) {
    // 다음 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.skill_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);
    let diff_t = action_state_timer.0 as i32 - duration as i32;

    let timings = &character_attributes.skill_timing;
    let mut index = skill_cost_data.count as usize;
    while let Some(timing) = timings.get(index).cloned()
        && timing <= action_state_timer.0
    {
        // 스킬 사용 이벤트를 생성합니다.
        let timing = elapsed_time_ms - (action_state_timer.0 - timing);
        events.push(ActionEventDetail {
            uid,
            timing,
            event: ActionEvent::Skill,
        });

        skill_cost_data.count += 1;
        index = skill_cost_data.count as usize;
    }

    if diff_t >= 0 {
        skill_cost_data.count = 0;

        // 행동 상태를 변경합니다.
        if held_input.contains(HeldInput::Skill)
            && skill_cost_data.remaining >= character_attributes.skill_cost
        {
            *action_state = ActionState::Skill;
            let duration = character_attributes.skill_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Skill),
            });
        } else if held_input.contains(HeldInput::Attack) && bullet_data.remaining > 0 {
            *action_state = ActionState::Attack;
            let duration = character_attributes.normal_attack_ing_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Attack),
            });
        } else if held_input.contains(HeldInput::Reload) {
            *action_state = ActionState::Reload;
            let duration = character_attributes.normal_reload_duration;
            action_state_timer.0 = (diff_t as u16).min(duration);

            let timing = elapsed_time_ms - diff_t as u16;
            events.push(ActionEventDetail {
                uid,
                timing,
                event: ActionEvent::Changed(ActionState::Reload),
            });
        } else if held_input.contains(HeldInput::Aiming) {
            *action_state = ActionState::Aiming;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        } else {
            *action_state = ActionState::Idle;
            let duration = character_attributes.normal_idle_duration;
            action_state_timer.0 = diff_t as u16 % duration;
        }
    }
}

/// [`ActionState::Callsign`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_callsign(
    _held_input: HeldInput,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.normal_callsign_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *action_state = ActionState::Idle;
        let duration = character_attributes.normal_idle_duration;
        action_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`ActionState::VictoryStart`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_victory_start(
    _held_input: HeldInput,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.victory_start_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *action_state = ActionState::VictoryEnd;
        let duration = character_attributes.normal_idle_duration;
        action_state_timer.0 = diff_t as u16 % duration;
    }
}

/// [`ActionState::VictoryEnd`]일 떄 플레이어의 [`ActionState`]와 [`ActionStateTimer`]를 변경합니다.
fn update_timer_when_victory_end(
    _held_input: HeldInput,
    _bullet_data: &mut BulletData,
    _skill_cost_data: &mut SkillCostData,
    action_state: &mut ActionState,
    action_state_timer: &mut ActionStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    // 행동 상태 타이머를 갱신합니다.
    let duration = character_attributes.victory_end_duration;
    action_state_timer.0 = action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        *action_state = ActionState::VictoryEnd;
        let duration = character_attributes.normal_idle_duration;
        action_state_timer.0 = diff_t as u16 % duration;
    }
}
