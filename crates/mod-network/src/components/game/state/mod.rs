//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

mod action_state;
mod movement_state;

use crate::components::{ActionState, MovementState};

pub use self::{action_state::*, movement_state::*};

/// 상태 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateEvent {
    /// 행동 상태 변경 이벤트
    ChangeActionState {
        from: ActionState,
        to: ActionState,
        timing: u16,
    },
    /// 움직임 상태 변경 이벤트
    ChangeMovementState {
        from: MovementState,
        to: MovementState,
        timing: u16,
    },
    /// 총알 발사 이벤트
    BulletFired { timing: u16 },
    /// 스킬 응답 이벤트
    SkillResponse { timing: u16 },
}
