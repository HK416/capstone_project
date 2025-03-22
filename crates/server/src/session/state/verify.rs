use std::{fmt, sync::Arc};

use crate::session::Session;

use super::{ControlFlow, SessionState, login::LoginState};

/// 클라이언트가 서버에 연결된 직후 데이터 무결성을 검사하는 상태입니다.
pub struct VerifyState;

impl VerifyState {
    /// 새로운 `VerifyState`를 생성합니다.
    pub fn new() -> Self {
        Self {}
    }
}

impl SessionState for VerifyState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, _session: &Arc<Session>) {
        // 다음 상태로 전환합니다.
        // TODO: 현재 클라이언트 데이터 무결성 검사를 진행하고 있지 않습니다.
        let next_state = Box::new(LoginState::new());
        *flow = Some(ControlFlow::Change(next_state));
    }
}

impl fmt::Debug for VerifyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(VerifyState))
    }
}
