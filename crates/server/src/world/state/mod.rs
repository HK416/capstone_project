mod formation;
mod in_game;
mod in_game_prepare;
mod in_game_sync;
mod room;

use std::{fmt, sync::Arc, time::Duration};

pub use self::room::*;

use super::{GameWorld, GameWorldEvent};

/// 게임 월드의 상태를 제어합니다.
#[allow(dead_code)]
#[derive(Debug)]
pub enum GameWorldStateFlow {
    Clear,
    Change(Box<dyn GameWorldState>),
    Push(Box<dyn GameWorldState>),
    Pop,
    Reset(Box<dyn GameWorldState>),
}

/// 게임 월드 상태가 구현해야 하는 `trait`입니다.
#[allow(unused_variables)]
pub trait GameWorldState: fmt::Debug + Send {
    /// 게임 월드 상태에 진입할 때 호출되는 함수입니다.
    fn on_enter(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 상태에서 빠져나올 때 호출되는 함수입니다.
    fn on_exit(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 상태가 일시정지할 때 호출되는 함수입니다.
    fn on_pause(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 상태가 재개될 때 호출되는 함수입니다.
    fn on_resume(&mut self, world: &Arc<GameWorld>) {}

    /// 게임 월드 이벤트를 처리합니다.
    fn handle_event(&mut self, event: GameWorldEvent, world: &Arc<GameWorld>) {
        log::warn!("ignored >> unused world event (STATE:{:?})", &self);
    }

    /// 게임 월드 상태를 갱신할 때 호출되는 함수입니다.
    fn on_advanced(&mut self, world: &Arc<GameWorld>) {}

    /// 다른 작업을 수행할 수 있도록 양보합니다.
    fn yield_now(&self) -> Duration {
        Duration::from_millis(5) // FIXME: 추후 32로 늘려야 함!
    }
}
