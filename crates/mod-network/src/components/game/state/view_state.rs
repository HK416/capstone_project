//! 시야 상태와 관련된 코드를 관리합니다.
//! 

use crate::components::{ActionState, CharacterAttributes, HeldInput, ViewState, ViewStateTimer};

/// [`ViewState`]과 입력 상태에 따라 [`ViewState`]를 갱신합니다.
pub fn update_view_state(
    action_state: ActionState,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    character_attributes: &CharacterAttributes,
    held_input: HeldInput,
) {
    match action_state {
        ActionState::Idle
        | ActionState::Aiming
        | ActionState::AimAt
        | ActionState::AimOff
        | ActionState::Attack => match view_state {
            ViewState::Idle => update_state_when_idle(
                view_state,
                view_state_timer,
                character_attributes,
                held_input,
            ),
            ViewState::ZoomIn => update_state_when_zoom_in(
                view_state,
                view_state_timer,
                character_attributes,
                held_input,
            ),
            ViewState::ZoomOut => update_state_when_zoom_out(
                view_state,
                view_state_timer,
                character_attributes,
                held_input,
            ),
            ViewState::Aiming => update_state_when_aiming(
                view_state,
                view_state_timer,
                character_attributes,
                held_input,
            ),
        },
        _ => {}
    }
}

/// [`ViewState::Idle`]일 떄 입력에 따라 [`ViewState`]를 갱신합니다.
fn update_state_when_idle(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    _character_attributes: &CharacterAttributes,
    held_input: HeldInput,
) {
    if held_input.contains(HeldInput::Aiming) {
        *view_state = ViewState::ZoomIn;
        view_state_timer.0 = 0;
    }
}

/// [`ViewState::ZoomIn`]일 떄 입력에 따라 [`ViewState`]를 갱신합니다.
fn update_state_when_zoom_in(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    character_attributes: &CharacterAttributes,
    held_input: HeldInput,
) {
    if !held_input.contains(HeldInput::Aiming) {
        *view_state = ViewState::ZoomOut;

        let zoom_in_duration = character_attributes.normal_attack_start_duration;
        let zoom_out_duration = character_attributes.normal_attack_end_duration;
        let s = view_state_timer.0 as f32 / zoom_in_duration as f32;
        let t = (1.0 - s) * zoom_out_duration as f32;
        view_state_timer.0 = t.floor() as u16;
    }
}

/// [`ViewState::ZoomOut`]일 떄 입력에 따라 [`ViewState`]를 갱신합니다.
fn update_state_when_zoom_out(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    character_attributes: &CharacterAttributes,
    held_input: HeldInput,
) {
    if held_input.contains(HeldInput::Aiming) {
        *view_state = ViewState::ZoomIn;

        let zoom_in_duration = character_attributes.normal_attack_start_duration;
        let zoom_out_duration = character_attributes.normal_attack_end_duration;
        let s = view_state_timer.0 as f32 / zoom_out_duration as f32;
        let t = (1.0 - s) * zoom_in_duration as f32;
        view_state_timer.0 = t.floor() as u16;
    }
}

/// [`ViewState::Aiming`]일 떄 입력에 따라 [`ViewState`]를 갱신합니다.
fn update_state_when_aiming(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    _character_attributes: &CharacterAttributes,
    held_input: HeldInput,
) {
    if !held_input.contains(HeldInput::Aiming) {
        *view_state = ViewState::ZoomOut;
        view_state_timer.0 = 0;
    }
}

/// [`ViewState`]에 따라 [`ViewStateTimer`]를 갱신합니다.
pub fn update_view_state_timer(
    action_state: ActionState,
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    match action_state {
        ActionState::Idle | ActionState::Aiming | ActionState::Attack => match view_state {
            ViewState::ZoomIn => update_timer_when_zoom_in(
                view_state,
                view_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            ViewState::ZoomOut => update_timer_when_zoom_out(
                view_state,
                view_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            ViewState::Idle | ViewState::Aiming => {}
        },
        ActionState::AimAt => update_timer_when_zoom_in(
            view_state,
            view_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::AimOff => update_timer_when_zoom_out(
            view_state,
            view_state_timer,
            character_attributes,
            elapsed_time_ms,
        ),
        ActionState::Retreat | ActionState::Reload | ActionState::Skill => match view_state {
            ViewState::Idle => {}
            ViewState::ZoomIn => {
                *view_state = ViewState::ZoomOut;

                let zoom_in_duration = character_attributes.normal_attack_start_duration;
                let zoom_out_duration = character_attributes.normal_attack_end_duration;
                let s = view_state_timer.0 as f32 / zoom_in_duration as f32;
                let t = (1.0 - s) * zoom_out_duration as f32;
                view_state_timer.0 = t.floor() as u16;
            }
            ViewState::ZoomOut => update_timer_when_zoom_out(
                view_state,
                view_state_timer,
                character_attributes,
                elapsed_time_ms,
            ),
            ViewState::Aiming => {
                *view_state = ViewState::ZoomOut;
            }
        },
        _ => {}
    }
}

/// [`ViewState::ZoomIn`]일 때 [`ViewStateTimer`]를 갱신합니다.
fn update_timer_when_zoom_in(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    let duration = character_attributes.normal_attack_start_duration;
    view_state_timer.0 = view_state_timer.0.saturating_add(elapsed_time_ms);

    if view_state_timer.0 >= duration {
        *view_state = ViewState::Aiming;
        view_state_timer.0 = 0;
    }
}

/// [`ViewState::ZoomOut`]일 때 [`ViewStateTimer`]를 갱신합니다.
fn update_timer_when_zoom_out(
    view_state: &mut ViewState,
    view_state_timer: &mut ViewStateTimer,
    character_attributes: &CharacterAttributes,
    elapsed_time_ms: u16,
) {
    let duration = character_attributes.normal_attack_end_duration;
    view_state_timer.0 = view_state_timer.0.saturating_add(elapsed_time_ms);

    if view_state_timer.0 >= duration {
        *view_state = ViewState::Idle;
        view_state_timer.0 = 0;
    }
}
