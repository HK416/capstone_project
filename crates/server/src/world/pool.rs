use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU32, Ordering as MemOrdering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use mod_network::components::{MAX_IN_GAME_PLAYERS, UserAccount, UserId, WorldId};
use mod_parallelism::collections::{Queue, SkipMap};

use crate::session::Session;

use super::GameWorld;

/// 최대 게임 월드의 개수입니다.
const MAX_GAME_WORLDS: usize = 1000;
/// 초기화시 생성되는 게임 월드의 개수입니다.
const INIT_GAME_WORLDS: usize = 5;
static_assertions::const_assert!(INIT_GAME_WORLDS < MAX_GAME_WORLDS);

/// 생성된 게임 월드를 관리하는 풀 객체입니다.
static POOL: OnceLock<SkipMap<WorldId, Arc<GameWorld>>> = OnceLock::new();
/// 비활성화된 게임 월드를 재사용하기 위해 저장합니다.
static RETIRES: OnceLock<Queue<Arc<GameWorld>>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> &'static SkipMap<WorldId, Arc<GameWorld>> {
    POOL.get_or_init(|| SkipMap::default())
}

/// 비활성화된 게임 월드를 저장하는 대기열을 가져옵니다.
pub(super) fn get_retires() -> &'static Queue<Arc<GameWorld>> {
    RETIRES.get_or_init(|| Queue::default())
}

/// 게임 월드 식별자를 생성합니다.
/// 생성된 게임 월드의 수가 `MAX_GAME_WORLDS`보다 클 경우 `None`을 반환합니다.
fn generate_id() -> Option<WorldId> {
    /// 게임 월드 식별자를 생성하기 위한 카운터입니다.
    static COUNTER: AtomicU32 = AtomicU32::new(1);

    // 생성된 게임 월드 수가 많은 경우 `None`을 반환합니다.
    let counter = COUNTER.fetch_add(1, MemOrdering::AcqRel);
    if counter >= MAX_GAME_WORLDS as u32 {
        return None;
    }

    loop {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        let counter_bitfield = counter & 0xFFF;
        let time_bitfield = duration.subsec_millis() & 0xFF;
        let val = (time_bitfield << 12 | counter_bitfield) % (MAX_GAME_WORLDS as u32);
        let id = WorldId::new(val);

        if id != WorldId::NULL && !get_pool().contains_key(&id) {
            return Some(id);
        }
    }
}

/// 생성된 게임 월드를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 전역 변수로 선언되어있으며, `GameWorldPool`은 변수에 접근할 수 있는 인터페이스를 제공합니다.
pub struct GameWorldPool;

impl GameWorldPool {
    /// 게임 월드 풀 객체를 초기화합니다.
    pub fn init() {
        let pool = get_pool();
        let retires = get_retires();
        let mut count = INIT_GAME_WORLDS;
        while count > 0 {
            // Safe: `INIT_GAME_WORLDS`는 `MAX_GAME_WORLDS`보다 작음
            let world_id = unsafe { generate_id().unwrap_unchecked() };
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
            .filter(|world| world.is_running() && world.admin() != UserId::NULL)
            .map(|world| world.clone())
    }

    /// 새로운 게임 월드를 생성합니다.
    pub fn create_custom(account: &UserAccount, session: &Arc<Session>) -> Option<Arc<GameWorld>> {
        // 게임 월드를 할당받습니다.
        let world = match get_retires().pop() {
            Some(world) => world,
            None => {
                // 게임 월드 식별자를 할당 받습니다.
                let world_id = generate_id()?;
                // 게임 월드를 생성합니다.
                let world = Arc::new(GameWorld::new(world_id));
                // 풀 객체에 게임 월드를 추가합니다.
                get_pool().insert(world_id, world.clone());
                world
            }
        };

        log::info!("GameWorld({}) is allocated.", world.id());
        println!("GameWorld({}) is allocated.", world.id());

        // 게임 월드를 실행합니다.
        world.run_custom(account, session);

        Some(world)
    }

    /// 접속 가능한 월드 아이디 목록을 가져옵니다.
    pub fn get_available_world_ids() -> Vec<WorldId> {
        let mut ids = Vec::new();
        for w in get_pool().iter() {
            let id = w.key();
            let world = w.value();

            if world.is_closed() || *world.num_players.lock() == MAX_IN_GAME_PLAYERS {
                continue;
            }

            ids.push(*id);
        }
        ids
    }
}
