//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

mod action_state;
mod movement_state;
mod player_state;

use std::cmp;

use crate::components::UserId;

pub use self::{action_state::*, movement_state::*, player_state::*};

#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionNotify {
    #[default]
    None = 0,
    EnterAttack = 1,
    Retreat = 2,
    Reload = 3,
    EnterSkill = 4,
    FirstAttack = 5,
    FirstSkill = 6,
}

impl ActionNotify {
    /// 주어진 정수로 행동 상태 알림을 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 [`ActionNotify::None`]을 반환합니다.
    ///
    pub const fn new(val: u8) -> ActionNotify {
        match val {
            1 => Self::EnterAttack,
            2 => Self::Retreat,
            3 => Self::Reload,
            4 => Self::EnterSkill,
            5 => Self::FirstAttack,
            6 => Self::FirstSkill,
            _ => Self::None,
        }
    }

    /// 행동 상태 알림을 가져옵니다.
    pub fn take(&mut self) -> ActionNotify {
        let temp = *self;
        *self = ActionNotify::None;
        temp
    }
}

/// 행동 상태 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEvent {
    /// 행동 상태가 변경될 때 발생하는 이벤트
    Changed(ActionState),
    /// 총알 재장전 이벤트
    Reload,
    /// 플레이어 리스폰 이벤트
    Respawn,
    /// 총알 발사 이벤트
    Attack,
    /// 스킬 응답 이벤트
    Skill,
}

/// 행동 상태 이벤트의 상세 내용입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionEventDetail {
    pub uid: UserId,
    pub timing: u16,
    pub event: ActionEvent,
}

impl Ord for ActionEventDetail {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.timing.cmp(&other.timing)
    }
}

impl PartialOrd<Self> for ActionEventDetail {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.timing.partial_cmp(&other.timing)
    }
}
