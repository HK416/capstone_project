//! 클라이언트의 게임 장면에 따라 패킷을 다르게 처리해야합니다.   
//! 따라서 세션 상태에 따라 패킷을 다르게 처리합니다.
//!
//! ## 세션 상태 목록
//! - Verify: 클라이언트가 서버에 연결된 직후의 데이터 무결성을 검사하는 상태입니다.
//! - Login: 클라이언트가 게임 서버에 로그인을 시도하는 상태입니다.
//! - Lobby: 클라이언트가 게임 로비 장면에 있는 상태입니다.
//! - Room: 클라이언트가 커스텀 게임 대기실 장면에 있는 상태입니다.
//! - InGame: 클라이언트가 인게임 장면에 있는 상태입니다.
//!
mod in_game;
mod lobby;
mod login;
mod room;
mod verify;

use std::{collections::VecDeque, fmt, sync::Arc};

use verify::VerifyState;

use super::Session;

/// 세션 상태를 제어합니다.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ControlFlow {
    /// 현재 세션 상태를 제거하고, 새로운 세션 상태를 추가합니다.
    Change(Box<dyn SessionState>),
    /// 모든 세션 상태를 제거합니다.
    Clear,
    /// 현재 세션 상태를 제거합니다.
    Pop,
    /// 새로운 세션 상태를 추가합니다.
    Push(Box<dyn SessionState>),
    /// 모든 세션 상태를 제거하고, 새로운 세션 상태를 추가합니다.
    Reset(Box<dyn SessionState>),
}

/// Session의 상태를 관리하는 관리자입니다.
#[derive(Debug)]
pub struct SessionStateManager<'a> {
    session: &'a Arc<Session>,
    states: VecDeque<Box<dyn SessionState>>,
}

impl<'a> SessionStateManager<'a> {
    /// 새로운 세션 상태 관리자를 생성합니다.
    pub fn new(session: &'a Arc<Session>) -> Self {
        Self {
            session,
            states: VecDeque::new(),
        }
    }

    /// 세션 상태 관리자를 실행합니다.
    pub async fn run(mut self) {
        let session = self.session;
        let init_state = Box::new(VerifyState);
        let mut control_flow = Some(ControlFlow::Reset(init_state));

        while session.is_running() {
            // 세션 상태를 갱신합니다.
            self.update(&mut control_flow);

            match self.current_state() {
                Some(state) => {
                    state.handle_packets(&mut control_flow, session);
                }
                None => {
                    // 현재 세션 상태가 없는 경우 세션을 종료합니다.
                    self.session.close();
                    return;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }

    /// 현재 세션 상태를 가져옵니다.
    fn current_state(&mut self) -> Option<&mut Box<dyn SessionState>> {
        self.states.back_mut()
    }

    /// 세션 상태를 갱신합니다.
    fn update(&mut self, control_flow: &mut Option<ControlFlow>) {
        if let Some(flow) = control_flow.take() {
            match flow {
                ControlFlow::Change(state) => self.change(state),
                ControlFlow::Clear => self.clear(),
                ControlFlow::Pop => self.pop(),
                ControlFlow::Push(state) => self.push(state),
                ControlFlow::Reset(state) => self.reset(state),
            }
        }
    }

    /// 모든 세션 상태를 제거합니다.
    fn clear(&mut self) {
        while let Some(mut state) = self.states.pop_back() {
            state.on_exit(&self.session);
        }
    }

    /// 새로운 세션 상태로 교체합니다.
    fn change(&mut self, state: Box<dyn SessionState>) {
        self.pop();
        self.push(state);
    }

    /// 새로운 세션 상태를 추가합니다.
    fn push(&mut self, mut state: Box<dyn SessionState>) {
        state.on_enter(&self.session);
        self.states.push_back(state);
    }

    /// 현재 세션 상태를 제거합니다.
    fn pop(&mut self) {
        if let Some(mut state) = self.states.pop_back() {
            state.on_exit(&self.session);
        }
    }

    /// 새로운 세션 상태로 초기화합니다.
    fn reset(&mut self, state: Box<dyn SessionState>) {
        self.clear();
        self.push(state);
    }
}

impl<'a> Drop for SessionStateManager<'a> {
    fn drop(&mut self) {
        self.clear()
    }
}

/// 세션 상태가 구현해야하는 기능을 모아놓은 trait입니다.
#[allow(unused_variables)]
pub trait SessionState: fmt::Debug + Send {
    /// 상태에 진입할 때 호출되는 콜백 함수입니다.
    fn on_enter(&mut self, session: &Arc<Session>) {
        /* empty */
    }

    /// 상태에 빠져나올 때 호출되는 콜백 함수입니다.
    fn on_exit(&mut self, session: &Arc<Session>) {
        /* empty */
    }

    /// 해당 상태에서 수신된 패킷을 처리합니다.
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        /* empty */
    }
}
