use std::{
    collections::VecDeque,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU32, Ordering as MemOrdering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use mod_network::components::{RecruitPhasePlayer, UserAccount, WorldId};
use mod_parallelism::collections::{Queue, SkipMap};

use crate::{
    game::{GameWorld, GameWorldState, GameWorldStatus, PlayerRecruitState, StateControlFlow},
    session::Session,
};

const MAX_GAME_WORLDS: usize = 1_000_000;

/// 생성된 게임 월드를 관리하는 풀 객체입니다.
static POOL: OnceLock<SkipMap<WorldId, Arc<GameWorld>>> = OnceLock::new();
/// 비활성화된 게임 월드를 재사용하기 위해 저장합니다.
static RETIRES: OnceLock<Queue<Arc<GameWorld>>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> &'static SkipMap<WorldId, Arc<GameWorld>> {
    POOL.get_or_init(|| SkipMap::default())
}

/// 비활성화된 게임 월드를 저장하는 대기열을 가져옵니다.
fn get_retires() -> &'static Queue<Arc<GameWorld>> {
    RETIRES.get_or_init(|| Queue::default())
}

/// 게임 월드 식별자를 생성합니다.
///
/// FIXME: `MAX_GAME_WORLDS`를 초과할 경우 어떻게 처리할 것인가...
///
fn generate_id() -> WorldId {
    /// 게임 월드 식별자를 생성하기 위한 카운터입니다.
    static COUNTER: AtomicU32 = AtomicU32::new(1);

    let mut id = WorldId::NULL;
    loop {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        let counter_bitfield = COUNTER.fetch_add(1, MemOrdering::AcqRel) & 0xFFF;
        let time_bitfield = duration.subsec_millis() & 0xFF;
        let val = (time_bitfield << 12 | counter_bitfield) % (MAX_GAME_WORLDS as u32);
        id = WorldId::new(val);

        if !get_pool().contains_key(&id) {
            break;
        }
    }
    id
}

/// 생성된 게임 월드를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 전역 변수로 선언되어있으며, `GameWorldPool`은 변수에 접근할 수 있는 인터페이스를 제공합니다.
pub struct GameWorldPool;

impl GameWorldPool {
    /// 게임 월드 풀 객체를 초기화합니다.
    pub fn init() {
        let pool = get_pool();
        let retires = get_retires();
        let mut count = 50;
        while count > 0 {
            let world_id = generate_id();
            let world = Arc::new(GameWorld::new(world_id));
            pool.insert(world_id, world.clone());
            retires.push(world);
            count -= 1;
        }
    }

    /// 주어진 식별자에 해당하는 활성화된 게임 월드를 가져옵니다.
    pub fn get(id: &WorldId) -> Option<Arc<GameWorld>> {
        get_pool()
            .get(id)
            .filter(|world| world.status() == GameWorldStatus::Open)
            .map(|world| world.clone())
    }

    /// 새로운 게임 월드를 생성합니다.
    pub fn create_custom(
        account: UserAccount,
        session: &Arc<Session>,
    ) -> (Arc<GameWorld>, Vec<RecruitPhasePlayer>) {
        // 게임 월드를 할당받습니다.
        let world = match get_retires().pop() {
            Some(world) => world,
            None => {
                // 게임 월드 식별자를 할당 받습니다.
                let world_id = generate_id();
                // 게임 월드를 생성합니다.
                let world = Arc::new(GameWorld::new(world_id));
                // 풀 객체에 게임 월드를 추가합니다.
                get_pool().insert(world_id, world.clone());
                world
            }
        };

        log::info!("GameWorld({}) is allocated.", world.id());
        println!("GameWorld({}) is allocated.", world.id());

        // 게임 월드를 초기화합니다.
        let players = world.reset_custom(account, session);

        // 게임 월드를 실행합니다.
        tokio::spawn(running_loop(world.clone()));

        (world, players)
    }
}

/// 게임 월드를 실행하는 루프함수입니다.
async fn running_loop(world: Arc<GameWorld>) {
    // 상태 스텍
    let mut stack: VecDeque<Box<dyn GameWorldState>> = VecDeque::with_capacity(4);

    // 상태 제어자
    let mut flow = Some(StateControlFlow::Reset(Box::new(PlayerRecruitState::new())));

    // 활성화된 게임 월드를 실행합니다.
    while world.status() != GameWorldStatus::Closed {
        // 게임 월드 상태를 갱신합니다.
        handle_flow(&mut flow, &mut stack, &world);

        // 게임 월드 상태를 실행합니다.
        if let Some(curr_state) = stack.back_mut() {
            curr_state.on_advanced(&mut flow, &world);
        }
    }

    // 비활성화된 게임 월드를 회수합니다.
    log::info!("GameWorld({}) is released.", world.id());
    println!("GameWorld({}) is released.", world.id());
    get_retires().push(world);
}

/// 게임 월드 상태를 제어합니다.
fn handle_flow(
    flow: &mut Option<StateControlFlow>,
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &GameWorld,
) {
    if let Some(flow) = flow.take() {
        match flow {
            StateControlFlow::None => {}
            StateControlFlow::Pop => {
                pop_state(stack, world);
            }
            StateControlFlow::Push(new) => {
                push_state(new, stack, world);
            }
            StateControlFlow::Change(new) => {
                change_state(new, stack, world);
            }
            StateControlFlow::Reset(new) => {
                reset_state(new, stack, world);
            }
        }
    }
}

/// 현재 상태를 제거합니다.
fn pop_state(stack: &mut VecDeque<Box<dyn GameWorldState>>, world: &GameWorld) {
    if let Some(mut state) = stack.pop_back() {
        state.on_exit(world);
    }
}

/// 새로운 상태를 추가합니다.
fn push_state(
    mut new: Box<dyn GameWorldState>,
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &GameWorld,
) {
    new.on_enter(world);
    stack.push_back(new);
}

/// 모든 상태를 제거합니다.
fn clear(stack: &mut VecDeque<Box<dyn GameWorldState>>, world: &GameWorld) {
    while let Some(mut state) = stack.pop_back() {
        state.on_exit(world);
    }
}

/// 현재 상태를 제거하고 새로운 상태를 추가합니다.
fn change_state(
    new: Box<dyn GameWorldState>,
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &GameWorld,
) {
    pop_state(stack, world);
    push_state(new, stack, world);
}

/// 모든 상태를 제거하고 새로운 상태를 추가합니다.
fn reset_state(
    new: Box<dyn GameWorldState>,
    stack: &mut VecDeque<Box<dyn GameWorldState>>,
    world: &GameWorld,
) {
    clear(stack, world);
    push_state(new, stack, world);
}
