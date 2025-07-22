use std::{collections::VecDeque, sync::{Arc, RwLock}};

use mod_network::components::{UserId, MAX_IN_GAME_PLAYERS};

use crate::{account::Account, session::Session};

static MATCH_QUEUE: RwLock<VecDeque<(Account, Arc<Session>)>> = RwLock::new(VecDeque::new());

pub struct MatchMaker;

impl MatchMaker {
    /// 대기열에 사용자를 추가합니다.
    pub fn add_to_queue(account: Account, session: Arc<Session>) {
        let mut queue = MATCH_QUEUE.write().unwrap();
        queue.push_back((account, session));
    }

    /// 사용자가 대기열에 있는지 확인합니다.
    pub fn is_in_queue(user_id: UserId) -> bool {
        let queue = MATCH_QUEUE.read().unwrap();
        queue.iter().any(|(account, _)| account.uid == user_id)
    }

    /// 대기열에서 사용자를 제거합니다.
    pub fn remove_from_queue(user_id: UserId) {
        let mut queue = MATCH_QUEUE.write().unwrap();
        queue.retain(|(account, _)| account.uid != user_id);
    }

    /// 대기열에서 매칭된 계정들을 가져옵니다.
    /// 
    /// `MAX_IN_GAME_PLAYERS` 이상의 계정이 대기열에 있는 경우에 해당 계정들을 pop하여 반환합니다.
    /// 
    /// (현재는 테스트를 위해 2명이 모이면 매칭되도록 설정되어 있습니다.)  
    /// 
    pub fn pop_matched_accounts() -> Option<Vec<(Account, Arc<Session>)>> {
        let mut queue = MATCH_QUEUE.write().unwrap();
        
        // 연결이 끊긴 계정은 대기열에서 제거합니다.
        queue.retain(|(_account, session)| session.is_running());

        if queue.len() < 2 {
            None
        } else {
            Some(queue.drain(..2).collect())
        }
    }
}