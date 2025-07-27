//! 플레이어 상태와 관련된 코드를 관리합니다.
//!

mod action_state;
mod movement_state;
mod player_state;
mod view_state;

use std::cmp;

use crate::components::{BigEndian, UserId};

pub use self::{action_state::*, movement_state::*, player_state::*, view_state::*};

#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionNotify {
    #[default]
    None = 0,
    Retreat = 1,
    Reload = 2,
    StartAttack = 3,
    FirstAttack = 4,
    Attack = 5,
    StartSkill = 6,
    FirstSkill = 7,
    Skill = 8,
}

impl ActionNotify {
    /// 주어진 정수로 행동 상태 알림을 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 [`ActionNotify::None`]을 반환합니다.
    ///
    pub const fn new(val: u8) -> ActionNotify {
        match val {
            1 => Self::Retreat,
            2 => Self::Reload,
            3 => Self::StartAttack,
            4 => Self::FirstAttack,
            5 => Self::Attack,
            6 => Self::StartSkill,
            7 => Self::FirstSkill,
            8 => Self::Skill,
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

impl BigEndian for ActionNotify {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::new(u8::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
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
