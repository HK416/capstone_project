use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU32, Ordering as MemOrdering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use mod_network::components::{
    GameTier, MAX_IN_GAME_PLAYERS, Permission, ProfileIcon, UserId, UserName, WorldId,
};
use mod_parallelism::collections::{Queue, SkipMap};

use crate::{
    entities::Player,
    session::Session,
    world::{GameWorldEvent, GameWorldSystemEvent, world_state_loop},
};

use super::GameWorld;

/// 최대 게임 월드의 개수입니다.
const MAX_GAME_WORLDS: usize = 999;
static_assertions::const_assert!(0 < MAX_GAME_WORLDS);

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

    let mut count: u32;
    let mut loop_count = 0;
    let n = loop {
        // 최대 시도 횟수를 초과한 경우 `None`을 반환합니다.
        const MAX_TRY_COUNT: usize = 1_000;
        if loop_count >= MAX_TRY_COUNT {
            return None;
        }

        // 카운터를 가져옵니다.
        // 카운터가 최대 게임 월드의 개수보다 클 경우 `None`을 반환합니다.
        count = COUNTER.load(MemOrdering::Relaxed);
        if count >= MAX_GAME_WORLDS as u32 {
            return None;
        }

        // CAS 명령어를 사용하여 카운터를 증가시킵니다.
        // 카운터 증가에 성공한 경우 `current`값을 취합니다.
        if COUNTER
            .compare_exchange(count, count + 1, MemOrdering::Release, MemOrdering::Relaxed)
            .is_ok()
        {
            break count;
        }

        loop_count += 1;
    };

    let mut loop_count = 0;
    loop {
        // 최대 시도 횟수를 초과한 경우 `None`을 반환합니다.
        const MAX_TRY_COUNT: usize = 1_000;
        if loop_count >= MAX_TRY_COUNT {
            return None;
        }

        // 현재 시간을 가져옵니다.
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        // 게임 월드 식별자를 생성합니다.
        let number_bit = n & 0xFFF;
        let time_bit = duration.subsec_millis() & 0xFF;
        let val = (time_bit << 12 | number_bit) % ((MAX_GAME_WORLDS + 1) as u32);
        let id = WorldId::new(val);

        // 게임 월드 식별자가 풀 객체에 존재하는지 확인합니다.
        if id != WorldId::NULL && !get_pool().contains_key(&id) {
            return Some(id);
        }

        loop_count += 1;
    }
}

/// 생성된 게임 월드를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 전역 변수로 선언되어있으며, `GameWorldPool`은 변수에 접근할 수 있는 인터페이스를 제공합니다.
pub struct GameWorldPool;

impl GameWorldPool {
    /// 게임 월드 풀 객체를 초기화합니다.
    pub fn init() {
        get_pool();
        get_retires();
    }

    /// 주어진 식별자에 해당하는 활성화된 게임 월드를 가져옵니다.
    pub fn get(id: &WorldId) -> Option<Arc<GameWorld>> {
        get_pool()
            .get(id)
            .filter(|world| world.is_running() && world.admin() != UserId::NULL)
            .map(|world| world.clone())
    }

    /// 새로운 게임 월드를 생성합니다.
    ///
    /// # Warnings
    /// 이 함수는 tokio [`Runtime`](tokio::runtime::Runtime)에서 실행될 경우 데드락을 발생시킬 수 있습니다.
    ///
    /// tokio에서 함수를 호출할 경우 [`tokio::task::spawn_blocking`]을 사용해 호출해야 합니다.
    ///
    /// # Panics
    /// - 주어진 `uid`는 `UserId::NULL`이 될 수 없습니다.
    ///
    pub fn create_custom(
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_icon: ProfileIcon,
        session: Arc<Session>,
    ) -> Option<Arc<GameWorld>> {
        assert_ne!(uid, UserId::NULL, "the given uid cannot be null!");

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

                log::info!("{} is allocated.", &world);
                println!("{} is allocated.", &world);
                world
            }
        };

        // 락을 획득합니다.
        // 주의: tokio 런타임에서 호출될 경우 스케쥴링 과정에서 데드락이 발생할 수 있음.
        let mut num_players = world.num_players.lock();

        // 게임 월드 관리자를 설정합니다.
        world.set_admin(uid);

        // 게임 월드 세션 집합에 현재 세션을 추가합니다.
        world.sessions.insert(session.clone(), uid);

        // 게임 월드 플레이어 집합에 플레이어 데이터를 추가합니다.
        world.players.insert(
            uid,
            Player::new(name)
                .with_tier(tier)
                .with_profile_icon(profile_icon)
                .with_permission(Permission::Admin),
        );

        *num_players += 1;
        log::info!("{} joined the {}", &session, &world);
        println!("{} joined the {}", &session, &world);

        // 게임 월드 이벤트를 추가합니다.
        let event = GameWorldSystemEvent::PlayerJoin;
        let event = GameWorldEvent::System {
            session,
            uid,
            event,
        };
        world.received_events.push(event);
        drop(num_players);

        // 게임 월드를 활성화합니다.
        world.is_running.store(true, MemOrdering::Release);
        world.is_closed.store(false, MemOrdering::Release);
        log::info!("{} enabled.", &world);
        println!("{} enabled.", &world);

        // 게임 월드를 실행합니다.
        tokio::spawn(world_state_loop(world.clone()));

        Some(world)
    }

    /// 접속 가능한 월드 아이디 목록을 가져옵니다.
    pub fn get_world_lists() -> Vec<WorldId> {
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
