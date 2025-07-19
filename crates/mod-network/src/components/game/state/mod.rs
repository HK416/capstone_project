//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

mod action_state;
mod movement_state;
mod player_state;

use std::cmp;

use crate::components::UserId;

pub use self::{action_state::*, movement_state::*, player_state::*};

/// 행동 상태 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEvent {
    /// 플레이어 리스폰 이벤트
    Respawn { timing: u16 },
    /// 총알 재장전 이벤트
    Reload,
    /// 총알 발사 이벤트
    Attack { timing: u16 },
    /// 스킬 응답 이벤트
    Skill { timing: u16 },
}

/// 행동 상태 이벤트의 상세 내용입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEventDetail {
    Respawn { uid: UserId, timing: u16 },
    Reload { uid: UserId },
    Attack { uid: UserId, timing: u16 },
    Skill { uid: UserId, timing: u16 },
}

impl ActionEventDetail {
    pub const fn timing(&self) -> u16 {
        match self {
            ActionEventDetail::Respawn { timing, .. } => *timing,
            ActionEventDetail::Reload { .. } => 0,
            ActionEventDetail::Attack { timing, .. } => *timing,
            ActionEventDetail::Skill { timing, .. } => *timing,
        }
    }
}

impl Ord for ActionEventDetail {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.timing().cmp(&other.timing())
    }
}

impl PartialOrd<Self> for ActionEventDetail {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.timing().partial_cmp(&other.timing())
    }
}
