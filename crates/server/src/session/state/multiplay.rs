//! 게임으로 들어가기 전에 Push하여 멀티플레이시에 필요한 동작을 처리합니다.

use std::sync::Arc;

use mod_network::components::UserId;
use mod_parallelism::collections::Queue;

use crate::{session::{Session, SessionState, SessionStateFlow}, world::{GameWorldEvent, GameWorldSystemEvent}};

pub struct SessionMultiplayState {
    uid: UserId,
    sender: Arc<Queue<GameWorldEvent>>,
}

impl SessionMultiplayState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(uid: UserId, sender: Arc<Queue<GameWorldEvent>>) -> Self {
        Self {
            uid,
            sender,
        }
    }
}

impl SessionState for SessionMultiplayState {
    fn on_exit(&mut self, session: &Arc<Session>) {
        // 게임 월드 떠남 알림을 보냅니다.
        let event = GameWorldSystemEvent::PlayerLeave;
        let event = GameWorldEvent::System {
            session: session.clone(),
            uid: self.uid,
            event,
        };
        self.sender.push(event);
    }

    fn on_resume(&mut self, session: &Arc<Session>) {
        // 세션 상태를 변경합니다.
        let state = SessionStateFlow::Pop;
        session.add_flow(state);
    }
}