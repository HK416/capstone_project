mod event;
mod pool;
mod state;

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering as MemOrdering},
    },
};

use ahash::RandomState;
use dashmap::DashMap;
use mod_network::{
    components::{
        GameTier, MAX_IN_GAME_PLAYERS, Permission, ProfileIcon, UserId, UserName, WorldId,
    },
    protocol::JoinFailedReason,
};
use mod_parallelism::collections::Queue;
use parking_lot::FairMutex;

use crate::{entities::Player, session::Session};

pub use self::{event::*, pool::*, state::*};

/// 게임 월드 관리자 초기화 값입니다.
const NULL_ID: u32 = UserId::NULL.into_inner();

/// 게임을 진행하고, 생성된 오브젝트를 관리합니다.
#[derive(Debug)]
pub struct GameWorld {
    /// 게임 월드 식별자입니다.
    world_id: WorldId,
    /// 게임 월드의 실행 여부입니다.
    is_running: AtomicBool,
    /// 외부 플레이어 출입의 제한 여부입니다.
    is_closed: AtomicBool,

    /// 게임 월드 관리자의 사용자 식별자입니다.
    admin: AtomicU32,

    /// 커스텀 게임 대기실에 참여한 플레이어 수 (동기화를 위해 Mutex를 사용함)
    num_players: FairMutex<usize>,

    /// 게임 월드에 참여한 세션 집합입니다.
    sessions: DashMap<Arc<Session>, UserId, RandomState>,
    /// 플레이어 오브젝트 집합입니다.
    players: DashMap<UserId, Player, RandomState>,

    /// 게임 월드 이벤트 대기열입니다.
    received_events: Queue<GameWorldEvent>,
    /// 게임 월드 상태 흐름 대기열입니다.
    flows: Queue<GameWorldStateFlow>,
}

impl GameWorld {
    /// 새로운 게임 월드를 생성합니다.
    pub fn new(world_id: WorldId) -> Self {
        Self {
            world_id,
            is_running: AtomicBool::new(false),
            is_closed: AtomicBool::new(true),
            admin: AtomicU32::new(NULL_ID),
            num_players: FairMutex::new(0),
            sessions: DashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new()),
            players: DashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS * 2, RandomState::new()),
            received_events: Queue::new(),
            flows: Queue::new(),
        }
    }

    /// 게임 월드의 실행 여부를 가져옵니다.
    pub fn is_running(&self) -> bool {
        self.is_running.load(MemOrdering::Acquire)
    }

    /// 게임 월드의 외부 출입 차단 여부를 가져옵니다.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(MemOrdering::Acquire)
    }

    /// 게임 월드의 외부 출입 차단 여부를 설정합니다.
    fn set_closed(&self, flag: bool) {
        self.is_closed.store(flag, MemOrdering::Release);
    }

    /// 게임 월드 관리자의 식별자를 가져옵니다.
    pub fn admin(&self) -> UserId {
        UserId::new(self.admin.load(MemOrdering::Acquire))
    }

    fn set_admin(&self, uid: UserId) {
        self.admin.store(uid.into_inner(), MemOrdering::Release);
    }

    /// 게임 월드 이벤트를 추가합니다.
    pub fn push_event(&self, event: GameWorldEvent) {
        if self.is_running() {
            self.received_events.push(event);
        }
    }

    /// 커스텀 게임 참여를 시도합니다.
    /// - 플레이어 추가에 성공한 경우 현재 참여한 플레이어 정보를 반환합니다.
    /// - 플레이어 추가에 실패한 경우 실패 사유를 반환합니다.
    ///
    /// # Warnings
    /// 이 함수는 tokio [`Runtime`](tokio::runtime::Runtime)에서 실행될 경우 데드락을 발생시킬 수 있습니다.
    ///
    /// tokio에서 함수를 호출할 경우 [`tokio::task::spawn_blocking`]을 사용해 호출해야 합니다.
    ///
    /// # Panics
    /// - 주어진 `uid`는 `UserId::NULL`이 될 수 없습니다.
    ///
    pub fn try_join(
        &self,
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_icon: ProfileIcon,
        session: Arc<Session>,
    ) -> Result<(), JoinFailedReason> {
        assert_ne!(uid, UserId::NULL, "the given uid cannot be null!");

        // 락을 획득합니다. 락은 함수 종료 시점에 해제됩니다.
        // 주의: tokio에서 호출될 경우 스케쥴링 과정에서 데드락이 발생할 수 있습니다.
        let mut num_players = self.num_players.lock();

        // 게임 월드가 활성화 상태인지 확인합니다.
        if !self.is_running() {
            return Err(JoinFailedReason::NotFound);
        }

        // 게임 월드가 외부 출입을 허용하고 있는지 확인합니다.
        if self.is_closed() {
            return Err(JoinFailedReason::InProgress);
        }

        // 게임 월드에 참여 인원이 가득찼는지 확인합니다.
        if *num_players == MAX_IN_GAME_PLAYERS {
            return Err(JoinFailedReason::FullCapacity);
        }

        // 게임 월드 세션 집합에 세션을 추가합니다.
        self.sessions.insert(session.clone(), uid);

        // 게임 월드 플레이어 집합에 플레이어 데이터를 추가합니다.
        self.players.insert(
            uid,
            Player::new(name)
                .with_tier(tier)
                .with_profile_icon(profile_icon)
                .with_permission(Permission::User),
        );

        *num_players += 1;
        log::info!("{} joined the {}", &session, &self);
        println!("{} joined the {}", &session, &self);

        // 게임 월드 이벤트를 추가합니다.
        let event = GameWorldSystemEvent::PlayerJoin;
        let event = GameWorldEvent::System {
            session,
            uid,
            event,
        };
        self.received_events.push(event);
        drop(num_players);

        Ok(())
    }

    /// 게임 월드에서 해당 플레이어를 제거합니다.
    ///
    /// # Warnings
    /// 이 함수는 tokio [`Runtime`](tokio::runtime::Runtime)에서 실행될 경우 데드락을 발생시킬 수 있습니다.
    ///
    /// tokio에서 함수를 호출할 경우 [`tokio::task::spawn_blocking`]을 사용해 호출해야 합니다.
    ///
    /// # Panics
    /// - 주어진 `uid`는 `UserId::NULL`이 될 수 없습니다.
    ///
    pub fn exit(&self, uid: UserId, session: Arc<Session>) {
        assert_ne!(uid, UserId::NULL, "the given uid cannot be null!");

        // 락을 획득합니다. 락은 함수 종료 시점에 해제됩니다.
        // 주의: tokio에서 호출될 경우 스케쥴링 과정에서 데드락이 발생할 수 있습니다.
        let mut num_players = self.num_players.lock();

        // 해당 플레이어를 게임 월드 세션 집합과 플레이어 집합에서 제거합니다.
        *num_players -= 1;
        let (_, uid) = match self.sessions.remove(&session) {
            Some(item) => item,
            None => {
                log::warn!("{} not found in {}", &session, &self);
                return;
            }
        };
        log::info!("{} leave the {}", &session, &self);
        println!("{} leave the {}", &session, &self);

        // 모든 플레이어가 게임 월드에서 나간 경우 게임 월드를 비활성화합니다.
        if *num_players == 0 {
            // 게임 월드를 비활성화 합니다.
            self.is_running.store(false, MemOrdering::Release);
            self.is_closed.store(true, MemOrdering::Release);
            // 게임 월드 데이터를 초기화합니다.
            self.set_admin(UserId::NULL);
            self.sessions.clear();
            self.players.clear();
            while let Some(_) = self.received_events.pop() {}
            while let Some(_) = self.flows.pop() {}
            log::info!("{} disabled.", &self);
            println!("{} disabled.", &self);
        } else {
            // 게임 월드 이벤트를 추가합니다.
            let event = GameWorldSystemEvent::PlayerLeave;
            let event = GameWorldEvent::System {
                session,
                uid,
                event,
            };
            self.received_events.push(event);
        }
    }
}

impl fmt::Display for GameWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameWorld({})", self.world_id)
    }
}
