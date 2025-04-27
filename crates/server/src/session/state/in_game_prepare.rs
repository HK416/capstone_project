use std::{
    fmt,
    sync::{Arc, Weak},
};

use mod_network::components::UserAccount;

use crate::{
    session::{Session, SessionEvents},
    world::GameWorld,
};

use super::{SessionState, SessionStateFlow, in_game::SessionInGameState};

/// 게임 진행 전에 잠시 대기하는 단계 (캐릭터 조작을 무효화하기 위해 추가됨)
pub struct SessionInGamePrepareState {
    /// 세션 상태 실행 여부
    is_running: bool,

    /// 사용자 계정 데이터
    account: UserAccount,
    /// 연결된 게임 월드
    world: Weak<GameWorld>,
}

impl SessionInGamePrepareState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(account: UserAccount, world: &Weak<GameWorld>) -> Self {
        Self {
            is_running: true,
            account,
            world: world.clone(),
        }
    }
}

impl SessionState for SessionInGamePrepareState {
    fn handle_event(&mut self, event: SessionEvents, session: &Arc<Session>) {
        match event {
            SessionEvents::EnterInGame => {
                // 다음 세션 상태로 전환합니다.
                self.is_running = false;
                let next_state = SessionInGameState::new(self.account, &self.world);
                let control_flow = SessionStateFlow::Change(Box::new(next_state));
                let event = SessionEvents::SetControlFlow(control_flow);
                session.push_event(event);
            }
            _ => {
                log::warn!(
                    "ignored >> unused session event (EVENT:{:?}, STATE:{:?})",
                    &event,
                    &self,
                );
            }
        }
    }

    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(_) = session.received_packets.pop() { /* empty */ }
    }
}

impl fmt::Debug for SessionInGamePrepareState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGamePrepareState))
    }
}
