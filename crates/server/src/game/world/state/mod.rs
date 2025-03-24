//! 게임 월드 상태에 관련된 코드를 관리합니다.
//!

pub mod formation;
pub mod play;
pub mod recruit;

use std::fmt;

pub use self::{formation::*, play::*, recruit::*};

use super::GameWorld;

/// 게임 월드 상태를 제어합니다.
pub enum StateControlFlow {
    /// 아무것도 지정되지 않은 상태입니다.
    None,
    /// 현재 상태를 빠져나옵니다.
    Pop,
    /// 새로운 상태를 추가합니다.
    Push(Box<dyn GameWorldState>),
    /// 상태를 변경합니다.
    Change(Box<dyn GameWorldState>),
    /// 모든 상태를 제거하고 새로운 상태를 추가합니다.
    Reset(Box<dyn GameWorldState>),
}

/// 게임 월드 상태가 구현해야하는 `trait`입니다.
#[allow(unused_variables)]
pub trait GameWorldState {
    /// 상태에 진입할 때 호출되는 콜백함수입니다.
    fn on_enter(&mut self, world: &GameWorld) {}

    /// 상태에서 빠져나올 때 호출되는 콜백함수입니다.
    fn on_exit(&mut self, world: &GameWorld) {}

    /// 상태를 진행시킬 때 호출되는 콜백함수입니다.
    fn on_advanced(&mut self, flow: &mut Option<StateControlFlow>, world: &GameWorld) {}
}

impl fmt::Debug for PlayerRecruitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(PlayerRecruitState))
    }
}
