//! 플레이어 행동 상태와 관련된 코드를 관리합니다.
//!

use mod_network::components::{ActionState, CharacterKind, RESPAWN_DELAY, UserId};
use mod_parallelism::collections::Queue;

use crate::{
    entities::Player,
    world::{GameWorldEvent, GameWorldInGameRunStateEvent},
};

/// 플레이어의 [`ActionStateTimer`]를 갱신합니다.
pub fn update_action_state_timer(
    uid: UserId,
    data: &mut Player,
    events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let action_state = data.player_states.action_state();
    match action_state {
        ActionState::Idle => {
            update_action_state_timer_when_idle(uid, data, events, elapsed_time_ms)
        }
        ActionState::Aiming => {
            update_action_state_timer_when_aiming(uid, data, events, elapsed_time_ms)
        }
        ActionState::AimAt => {
            update_action_state_timer_when_aim_at(uid, data, events, elapsed_time_ms)
        }
        ActionState::AimOff => {
            update_action_state_timer_when_aim_off(uid, data, events, elapsed_time_ms)
        }
        ActionState::Attack => {
            update_action_state_timer_when_attack(uid, data, events, elapsed_time_ms)
        }
        ActionState::Death => {
            update_action_state_timer_when_death(uid, data, events, elapsed_time_ms)
        }
        ActionState::Reload => {
            update_action_state_timer_when_reload(uid, data, events, elapsed_time_ms);
        }
        ActionState::Skill => {
            update_action_state_timer_when_skill(uid, data, events, elapsed_time_ms);
        }
        _ => {}
    }
}

/// [`ActionState::Idle`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_idle(
    _uid: UserId,
    data: &mut Player,
    _events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let duration = data.character_attributes().normal_idle_duration;
    data.action_state_timer.0 =
        (data.action_state_timer.0.saturating_add(elapsed_time_ms)) % duration;
}

/// [`ActionState::Aiming`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aiming(
    _uid: UserId,
    data: &mut Player,
    _events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let duration = data.character_attributes().normal_idle_duration;
    data.action_state_timer.0 =
        (data.action_state_timer.0.saturating_add(elapsed_time_ms)) % duration;
}

/// [`ActionState::AimAt`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aim_at(
    _uid: UserId,
    data: &mut Player,
    _events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let duration = data.character_attributes().normal_attack_start_duration;
    data.action_state_timer.0 = data.action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        data.prev_action_state = ActionState::AimAt;
        data.player_states.set_action_state(ActionState::Aiming);
        data.action_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::AimOff`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_aim_off(
    _uid: UserId,
    data: &mut Player,
    _events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let duration = data.character_attributes().normal_attack_end_duration;
    data.action_state_timer.0 = data.action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        data.prev_action_state = ActionState::AimOff;
        data.player_states.set_action_state(ActionState::Idle);
        data.action_state_timer.0 = diff_t as u16;
    }
}

/// [`ActionState::Attack`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_attack(
    uid: UserId,
    data: &mut Player,
    events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let duration = data.character_attributes().normal_attack_ing_duration;
    data.action_state_timer.0 = data.action_state_timer.0.saturating_add(elapsed_time_ms);

    let timing = &data.character_attributes().normal_attack_timing;
    let index = data.fire_per_attack as usize;
    let time_point = timing.get(index).cloned().unwrap_or(duration);

    if data.action_state_timer.0 < duration
        && time_point <= data.action_state_timer.0
        && data.current_bullet > 0
    {
        data.fire_per_attack += 1;
        data.current_bullet -= 1;

        let shooter_id = uid;
        let delay_time_ms = data.action_state_timer.0 - time_point;
        let event = GameWorldInGameRunStateEvent::SpawnBullet {
            shooter_id,
            delay_time_ms,
            bullet_kind: data.character_kind().into(),
            translation: data.translation,
            rotation: data.rotation,
        };
        let event = GameWorldEvent::inGameRunState(event);
        events.push(event);
    }

    let diff_t = data.action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        data.player_states.set_action_state(data.prev_action_state);
        data.prev_action_state = ActionState::Attack;
        data.action_state_timer.0 = diff_t as u16;
        data.fire_per_attack = 0;
    }
}

/// [`ActionState::Death`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_death(
    uid: UserId,
    data: &mut Player,
    events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    data.action_state_timer.0 = data.action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.action_state_timer.0 as i32 - RESPAWN_DELAY as i32;
    if diff_t >= 0 {
        data.prev_action_state = ActionState::Idle;
        data.player_states.set_action_state(ActionState::Idle);
        data.action_state_timer.0 = diff_t as u16;

        let event = GameWorldInGameRunStateEvent::RespawnPlayer(uid);
        let event = GameWorldEvent::inGameRunState(event);
        events.push(event);
    }
}

/// [`ActionState::Reload`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_reload(
    _uid: UserId,
    data: &mut Player,
    _events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    let duration = data.character_attributes().normal_reload_duration;
    data.action_state_timer.0 = data.action_state_timer.0.saturating_add(elapsed_time_ms);

    let diff_t = data.action_state_timer.0 as i32 - duration as i32;
    if diff_t >= 0 {
        data.player_states.set_action_state(data.prev_action_state);
        data.prev_action_state = ActionState::Reload;
        data.action_state_timer.0 = diff_t as u16;

        data.current_bullet = data.character_attributes().max_bullets;
    }
}

/// [`ActionState::Skill`]일 때 플레이어의 [`ActionStateTimer`]를 갱신합니다.
fn update_action_state_timer_when_skill(
    uid: UserId,
    data: &mut Player,
    events: &Queue<GameWorldEvent>,
    elapsed_time_ms: u16,
) {
    match data.character_kind() {
        CharacterKind::ArisOriginal => {}
        CharacterKind::MomoiOriginal => {}
        CharacterKind::MidoriOriginal => {}
        CharacterKind::YuukaOriginal => {}
    }
}
