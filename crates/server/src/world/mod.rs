mod event;
mod pool;
mod state;

use std::{
    collections::VecDeque,
    fmt,
    sync::{
        Arc,
        atomic::{self, AtomicBool, AtomicU32, Ordering as MemOrdering},
    },
};

use ahash::RandomState;
use dashmap::{DashMap, iter::Iter};
use mod_network::components::{
    JoinFailedReason, MAX_IN_GAME_PLAYERS, ObjectId, Permission, Team, UserAccount, UserId, WorldId,
};
use mod_parallelism::collections::Queue;
use parking_lot::FairMutex;
use rand::seq::SliceRandom;

use crate::{
    entities::{BulletObject, PlayerObject},
    session::Session,
};

pub use self::{event::*, pool::*, state::*};

/// 게임을 진행하고, 생성된 오브젝트를 관리합니다.
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
    sessions: DashMap<Arc<Session>, UserId>,
    /// 플레이어 오브젝트 집합입니다.
    players: DashMap<UserId, PlayerObject, RandomState>,
    /// 총알 오브젝트 집합입니다.
    bullets: DashMap<ObjectId, BulletObject, RandomState>,

    /// 게임 월드 이벤트 대기열입니다.
    events: Queue<GameWorldEvent>,
}

impl GameWorld {
    /// 새로운 게임 월드를 생성합니다.
    pub fn new(world_id: WorldId) -> Self {
        Self {
            world_id,
            is_running: AtomicBool::new(false),
            is_closed: AtomicBool::new(true),
            admin: AtomicU32::new(UserId::NULL.into_inner()),
            num_players: FairMutex::new(0),
            sessions: DashMap::default(),
            players: DashMap::default(),
            bullets: DashMap::default(),
            events: Queue::new(),
        }
    }

    /// 게임 월드의 식별자를 반환합니다.
    pub fn id(&self) -> WorldId {
        self.world_id
    }

    /// 게임 월드의 실행 여부를 가져옵니다.
    pub fn is_running(&self) -> bool {
        self.is_running.load(MemOrdering::Relaxed)
    }

    /// 게임 월드의 외부 출입 차단 여부를 가져옵니다.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(MemOrdering::Relaxed)
    }

    /// 게임 월드의 외부 출입 차단 여부를 설정합니다.
    pub fn set_closed(&self, flag: bool) {
        self.is_closed.store(flag, MemOrdering::Release);
    }

    /// 게임 월드를 비활성화합니다.
    pub fn disable(&self) {
        // 락을 획득합니다.
        let mut num_player = self.num_players.lock();
        // 게임 월드를 비활성화 합니다.
        self.is_running.store(false, MemOrdering::Release);
        self.is_closed.store(true, MemOrdering::Release);
        // 게임 월드 데이터를 초기화합니다.
        self.admin
            .store(UserId::NULL.into_inner(), MemOrdering::Release);
        self.sessions.clear();

        self.players.clear();
        self.bullets.clear();
        *num_player = 0;
    }

    /// 게임 월드 관리자의 식별자를 가져옵니다.
    pub fn admin(&self) -> UserId {
        UserId::new(self.admin.load(MemOrdering::Acquire))
    }

    /// 게임 월드 이벤트를 추가합니다.
    pub fn push_event(&self, event: GameWorldEvent) {
        if self.is_running() {
            self.events.push(event);
        }
    }

    /// 게임 월드를 커스텀 게임 월드로 재설정합니다.
    ///
    /// # Panics
    /// 게임 월드는 비활성화된 상태여야합니다. 그렇지 않은 경우 `panic!`을 호출합니다.
    ///
    pub fn run_custom(self: &Arc<Self>, account: &UserAccount, session: &Arc<Session>) {
        // 락을 획득합니다.
        let mut num_players = self.num_players.lock();
        assert!(!self.is_running(), "the game world is active!");

        // 게임 관리자를 설정합니다.
        self.admin
            .store(account.uid.into_inner(), MemOrdering::Release);

        // 세션 집합과 플레이어 집합에 게임 관리자를 추가합니다.
        self.sessions.insert(session.clone(), account.uid);
        self.players.insert(
            account.uid,
            PlayerObject::new(account.clone(), Permission::Admin, Team::Blue),
        );
        *num_players += 1;

        // 게임 월드를 활성화합니다.
        self.is_running.store(true, MemOrdering::Release);
        self.is_closed.store(false, MemOrdering::Release);

        // 상태 변경 이벤트를 추가합니다.
        let init_state = Box::new(GameWorldRoomState::new());
        let control_flow = GameWorldStateFlow::Reset(init_state);
        let event = GameWorldEvent::SetControlFlow(control_flow);
        self.push_event(event);

        atomic::fence(MemOrdering::SeqCst);

        // 게임 월드를 실행합니다.
        tokio::spawn(running_loop(self.clone()));
    }

    /// 커스텀 게임 참여를 시도합니다.
    /// - 플레이어 추가에 성공한 경우 현재 참여한 플레이어 정보를 반환합니다.
    /// - 플레이어 추가에 실패한 경우 실패 사유를 반환합니다.
    pub fn try_join(
        &self,
        account: UserAccount,
        session: &Arc<Session>,
    ) -> Result<(), JoinFailedReason> {
        // 락을 획득합니다.
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

        // 각 팀의 인원 수를 계산합니다.
        let mut num_red_team = 0;
        let mut num_blue_team = 0;
        for player in self.players.iter() {
            if player.team() == Team::Blue {
                num_blue_team += 1;
            } else {
                num_red_team += 1;
            }
        }

        if num_red_team < num_blue_team {
            // 세션을 추가합니다.
            self.sessions.insert(session.clone(), account.uid);
            // 플레이어를 추가합니다.
            self.players.insert(
                account.uid,
                PlayerObject::new(account, Permission::User, Team::Red),
            );
        } else {
            // 세션을 추가합니다.
            self.sessions.insert(session.clone(), account.uid);
            // 플레이어를 추가합니다.
            self.players.insert(
                account.uid,
                PlayerObject::new(account, Permission::User, Team::Blue),
            );
        }
        *num_players += 1;

        Ok(())
    }

    /// 게임 월드에서 해당 플레이어를 제거합니다.
    pub fn exit(&self, session: &Session) {
        // 락을 획득합니다.
        let mut num_players = self.num_players.lock();

        // 해당 플레이어를 제거합니다.
        if let Some((_, uid)) = self.sessions.remove(session) {
            if let Some((_, player)) = self.players.remove(&uid) {
                // 플레이어 수를 1줄이고, 이벤트를 추가합니다.
                *num_players -= 1;
                self.events.push(GameWorldEvent::PlayerLeave(uid));

                // 모든 플레이어가 게임 월드에서 나간 경우 게임 월드를 비활성화합니다.
                if self.players.len() == 0 {
                    // 게임 월드를 비활성화 합니다.
                    self.is_running.store(false, MemOrdering::Release);
                    self.is_closed.store(true, MemOrdering::Release);
                    // 게임 월드 데이터를 초기화합니다.
                    self.admin
                        .store(UserId::NULL.into_inner(), MemOrdering::Release);
                    self.sessions.clear();
                    self.players.clear();
                    self.bullets.clear();
                    *num_players = 0;
                    return;
                }

                // 제거된 플레이어의 권한이 관리자인 경우
                // 남아있는 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
                if player.permission() == Permission::Admin {
                    let mut remaining_players: Vec<_> =
                        self.players.iter().map(|item| item.key().clone()).collect();
                    remaining_players.shuffle(&mut rand::rng());

                    // Safe: 플레이어는 비어있지 않음
                    let uid = unsafe { remaining_players.pop().unwrap_unchecked() };
                    // 게임 관리자를 설정합니다.
                    self.admin.store(uid.into_inner(), MemOrdering::Release);
                    let mut player = unsafe { self.players.get_mut(&uid).unwrap_unchecked() };
                    player
                        .with_permission(Permission::Admin)
                        .with_bool_flag(false);
                }
            }
        }
    }

    /// 세션에 해당하는 게임 월드 플레이어에 접근합니다.  
    /// 주어진 세션에 해당하는 게임 월드 플레이어가 존재하지 않는 경우 `false`를 반환합니다.
    pub fn access_mut<F>(&self, session: &Session, func: F) -> bool
    where
        F: FnOnce(&mut PlayerObject),
    {
        if let Some(uid) = self.sessions.get(session) {
            if let Some(mut player) = self.players.get_mut(&uid) {
                func(&mut player);
                return true;
            }
        }

        false
    }

    /// 플레이어의 반복자를 반환합니다.
    pub fn iter_players(&self) -> Iter<'_, UserId, PlayerObject, RandomState> {
        self.players.iter()
    }
}

impl fmt::Debug for GameWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameWorld({})", self.world_id)
    }
}

/// 게임 월드를 실행하는 루프함수입니다.
async fn running_loop(world: Arc<GameWorld>) {
    // 게임 월드 상태를 저장하는 스텍 컨테이너입니다.
    let mut stack = VecDeque::new();
    let mut events = VecDeque::new();
    while world.is_running() {
        // 게임 월드 이벤트를 처리합니다.
        while let Some(event) = world.events.pop() {
            match event {
                GameWorldEvent::SetControlFlow(control_flow) => {
                    update_state(&mut stack, control_flow, &world);
                }
                _ => events.push_back(event),
            };
        }

        // 현재 상태를 가져옵니다.
        let curr_state = match stack.back_mut() {
            Some(state) => state,
            None => {
                // 현재 상태가 없는 경우 게임 월드를 비활성화합니다.
                world.disable();
                break;
            }
        };

        // 처리하지 않은 게임 월드 이벤트를 게임 월드 상태에서 처리합니다.
        while let Some(event) = events.pop_front() {
            curr_state.handle_event(event, &world);
        }

        // 현재 상태를 갱신합니다.
        curr_state.on_advanced(&world);

        // 다른 작업에 양도합니다.
        tokio::time::sleep(curr_state.yield_now()).await;
    }

    // 비활성화된 게임 월드를 회수합니다.
    log::info!("GameWorld({}) is released.", world.id());
    println!("GameWorld({}) is released.", world.id());
    get_retires().push(world);
}

/// 게임 월드의 이벤트를 처리합니다.
fn update_state(
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    control_flow: GameWorldStateFlow,
    world: &Arc<GameWorld>,
) {
    match control_flow {
        GameWorldStateFlow::Clear => {
            clear_state(stack, world);
        }
        GameWorldStateFlow::Change(new) => {
            change_state(stack, world, new);
        }
        GameWorldStateFlow::Push(new) => {
            push_state(stack, world, new);
        }
        GameWorldStateFlow::Pop => {
            pop_state(stack, world);
        }
        GameWorldStateFlow::Reset(new) => {
            reset_state(stack, world, new);
        }
    }
}

/// 모든 게임 월드 상태를 정리합니다.
fn clear_state(stack: &mut VecDeque<Box<dyn GameWorldState>>, world: &Arc<GameWorld>) {
    while let Some(mut state) = stack.pop_back() {
        log::info!("GamwWorld({:?}) exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }
}

/// 현재 게임 월드 상태를 정리하고, 새로운 게임 월드 상태를 추가합니다.
fn change_state(
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    mut new: Box<dyn GameWorldState>,
) {
    if let Some(mut state) = stack.pop_back() {
        log::info!("GamwWorld({:?}) exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }

    log::info!("GamwWorld({:?}) enter GameWorldState({:?})", &world, &new);
    new.on_enter(world);
    stack.push_back(new);
}

/// 새로운 게임 월드 상태를 추가합니다.
fn push_state(
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    mut new: Box<dyn GameWorldState>,
) {
    if let Some(curr_state) = stack.back_mut() {
        log::info!(
            "GamwWorld({:?}) pause GameWorldState({:?})",
            &world,
            &curr_state
        );
        curr_state.on_pause(world);
    }

    log::info!("GamwWorld({:?}) enter GameWorldState({:?})", &world, &new);
    new.on_enter(world);
    stack.push_back(new);
}

/// 현재 게임 월드 상태를 제거합니다.
fn pop_state(stack: &mut VecDeque<Box<dyn GameWorldState>>, world: &Arc<GameWorld>) {
    if let Some(mut state) = stack.pop_back() {
        log::info!("GamwWorld({:?}) exit GameWorldState({:?})", &world, &state);
        state.on_exit(world);
    }

    if let Some(curr_state) = stack.back_mut() {
        log::info!(
            "GamwWorld({:?}) resume GameWorldState({:?})",
            &world,
            &curr_state
        );
        curr_state.on_resume(world);
    }
}

/// 모든 게임 월드 상태를 정리하고, 새로운 게임 월드 상태를 추가합니다.
fn reset_state(
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &Arc<GameWorld>,
    new: Box<dyn GameWorldState>,
) {
    clear_state(stack, world);
    push_state(stack, world, new);
}
