mod event;
mod pool;
mod state;

use std::{fmt, sync::Arc};

use ahash::{HashMap, RandomState};
use mod_network::components::{MAX_IN_GAME_PLAYERS, UserId, WorldId};
use mod_parallelism::collections::Queue;

use crate::{entities::Player, session::Session};

pub use self::{event::*, pool::*, state::*};

/// 게임을 진행하고, 생성된 오브젝트를 관리합니다.
#[derive(Debug)]
pub struct GameWorld {
    /// 게임 월드 식별자입니다.
    id: WorldId,
    /// 게임 월드의 실행 여부입니다.
    running: bool,
    /// 외부 플레이어 출입의 제한 여부입니다.
    closed: bool,

    /// 게임 월드 관리자의 사용자 식별자입니다.
    admin: UserId,

    // 게임 월드에 참여한 세션 집합입니다.
    sessions: HashMap<Arc<Session>, UserId>,
    /// 플레이어 오브젝트 집합입니다.
    players: HashMap<UserId, Player>,

    events: Arc<Queue<GameWorldEvent>>,
    /// 게임 월드 상태 흐름 대기열입니다.
    flows: Queue<GameWorldStateFlow>,
}

impl GameWorld {
    /// 새로운 게임 월드를 생성합니다.
    pub fn new(world_id: WorldId) -> Self {
        Self {
            id: world_id,
            running: false,
            closed: true,
            admin: UserId::NULL,
            sessions: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new()),
            players: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS * 2, RandomState::new()),
            events: Arc::new(Queue::new()),
            flows: Queue::new(),
        }
    }

    pub fn disabled(&mut self) {
        self.running = false;
        self.closed = true;
        self.admin = UserId::NULL;
        self.sessions.clear();
        self.players.clear();
        while let Some(_) = self.events.pop() {}
        while let Some(_) = self.flows.pop() {}
    }
}

impl fmt::Display for GameWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameWorld({})", self.id)
    }
}
