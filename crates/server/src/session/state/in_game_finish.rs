use std::{fmt, sync::Weak};

use mod_network::components::UserAccount;

use crate::world::GameWorld;

use super::SessionState;

/// 게임 진행 결과를 확인하는 단계
pub struct SessionInGameFinishState {
    /// 세션 상태 실행 여부
    is_running: bool,

    /// 사용자 계정 데이터
    account: UserAccount,
    /// 연결된 게임 월드
    world: Weak<GameWorld>,
}

impl SessionInGameFinishState {
    /// 새로운 세션 상태를 생성합니다.
    pub fn new(account: UserAccount, world: &Weak<GameWorld>) -> Self {
        Self {
            is_running: true,
            account,
            world: world.clone(),
        }
    }
}

impl SessionState for SessionInGameFinishState {}

impl fmt::Debug for SessionInGameFinishState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGameFinishState))
    }
}
