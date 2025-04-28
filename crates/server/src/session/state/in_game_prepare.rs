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

pub struct SessionInGamePrepareState {
    // 세션 상태의 실행 여부
    is_running: bool,
    /// 사용자 계정 데이터
    account: Option<UserAccount>,
    /// 연결된 게임 월드
    world: Option<Weak<GameWorld>>,
}

impl SessionInGamePrepareState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(account: UserAccount, world: Weak<GameWorld>) -> Self {
        Self {
            is_running: true,
            account: Some(account),
            world: Some(world),
        }
    }
}

//--------------------------------------------------------------------------------------------
// 처리와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl SessionInGamePrepareState {
    /// `StartGamePlay`이벤트를 처리합니다.
    fn handle_start_game_play_event(&mut self, session: &Arc<Session>) {
        // 다음 세션 상태로 전환합니다.
        self.is_running = false;
        let account = self.account.take().unwrap();
        let world = self.world.take().unwrap();
        let next_state = SessionInGameState::new(account, world);
        let control_flow = SessionStateFlow::Change(Box::new(next_state));
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }
}

//--------------------------------------------------------------------------------------------

impl SessionState for SessionInGamePrepareState {
    fn handle_event(&mut self, event: SessionEvents, session: &Arc<Session>) {
        // 세션 상태가 실행 중이 아닌 경우 함수 실행을 생략합니다.
        if !self.is_running {
            return;
        }

        match event {
            SessionEvents::StartGamePlay => {
                self.handle_start_game_play_event(session);
            }
            _ => {
                log::warn!(
                    "ignored >> unused session event (EVENT:{:?}, STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn handle_packets(&mut self, session: &Arc<Session>) {
        // 현재 세션 상태에서는 클라이언트로부터 들어오는 패킷을 처리하지 않습니다.
        while let Some(_) = session.received_packets.pop() { /* empty */ }
    }
}

impl fmt::Debug for SessionInGamePrepareState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGamePrepareState))
    }
}
