use std::{fmt, sync::Arc};

use crate::session::{Session, SessionEvents};

use super::{SessionState, SessionStateFlow, login::SessionLoginState};

/// 클라이언트가 서버에 연결된 직후 데이터 무결성을 검사하는 상태입니다.
pub struct SessionVerifyState;

impl SessionVerifyState {
    /// 새로운 `VerifyState`를 생성합니다.
    pub fn new() -> Self {
        Self {}
    }
}

impl SessionState for SessionVerifyState {
    fn handle_packets(&mut self, session: &Arc<Session>) {
        // 다음 상태로 전환합니다.
        // TODO: 현재 클라이언트 데이터 무결성 검사를 진행하고 있지 않습니다.
        let next_state = Box::new(SessionLoginState::new());
        let control_flow = SessionStateFlow::Change(next_state);
        let event = SessionEvents::SetControlFlow(control_flow);
        session.push_event(event);
    }
}

impl fmt::Debug for SessionVerifyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(VerifyState))
    }
}
