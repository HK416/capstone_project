//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

mod action_state;
mod movement_state;
mod player_state;

pub use self::{action_state::*, movement_state::*, player_state::*};

/// 행동 상태 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEvent {
    /// 플레이어 리스폰 이벤트
    Respawn { timing: u16 },
    /// 총알 재장전 이벤트
    Reloading,
    /// 총알 발사 이벤트
    BulletFired { timing: u16 },
    /// 스킬 응답 이벤트
    Skill { timing: u16 },
}
