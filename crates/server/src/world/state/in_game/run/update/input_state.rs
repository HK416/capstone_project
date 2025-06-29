//! 플레이어 입력 상태와 관련된 코드를 관리합니다.
//!

use mod_network::components::{MAX_INPUT_STATE_TIME, MovementState};

use crate::entities::Player;

/// 플레이어의 [`InputStateTimer`]를 갱신합니다.
pub fn update_input_sate_timer(data: &mut Player, elapsed_time_ms: u16) {
    let movement_state = data.player_states.movement_state();
    match movement_state {
        MovementState::Idle | MovementState::MoveToEnd => {
            data.input_state_timer.0 = (data.input_state_timer.0.saturating_add(elapsed_time_ms))
                .min(MAX_INPUT_STATE_TIME);
        }
        MovementState::Moving => {
            data.input_state_timer.0 = data.input_state_timer.0.saturating_sub(elapsed_time_ms);
        }
        MovementState::Jumping | MovementState::Landing => {}
    }
}
