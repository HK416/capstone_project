use std::sync::Arc;

use crate::session::Session;

use super::{ControlFlow, SessionState, login::LoginState};

/// 클라이언트가 서버에 연결된 직후 데이터 무결성을 검사하는 상태입니다.
#[derive(Debug)]
pub struct VerifyState;

impl SessionState for VerifyState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, _session: &Arc<Session>) {
        // 다음 상태로 전환합니다.
        // TODO: 현재 클라이언트 데이터 무결성 검사를 진행하고 있지 않습니다.
        let next_state = Box::new(LoginState);
        *flow = Some(ControlFlow::Change(next_state));
    }
}
