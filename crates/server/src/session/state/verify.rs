use std::sync::Arc;

use crate::session::Session;

use super::{SessionState, SessionStateFlow, login::SessionLoginState};

/// 클라이언트가 서버에 연결된 직후 데이터 무결성을 검사하는 상태입니다.
pub struct SessionVerifyState;

impl SessionVerifyState {
    /// 새로운 `VerifyState`를 생성합니다.
    pub const fn new() -> Self {
        Self
    }
}

impl SessionState for SessionVerifyState {
    fn on_enter(&mut self, session: &Arc<Session>) {
        // 다음 상태로 전환합니다.
        // TODO: 현재 클라이언트 데이터 무결성 검사를 진행하고 있지 않습니다.
        println!(
            "현재 클라이언트 데이터 무결성 검사를 진행하고 있지 않습니다. {}(을)를 다음 세션 상태로 변경합니다.",
            &session
        );
        let state = Box::new(SessionLoginState::new());
        let flow = SessionStateFlow::Change(state);
        session.flows.push(flow);
    }
}
