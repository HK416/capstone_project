use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU32, Ordering as MemOrdering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use mod_network::components::{CustomGamePlayer, UserInfo, WorldId};
use mod_parallelism::collections::{Queue, SkipMap};

use crate::session::Session;

use super::CustomGameRoom;

/// 생성된 커스텀 게임을 관리하는 풀 객체입니다.
#[derive(Debug)]
pub struct CustomGamePool {
    /// 게임 월드 식별자를 생성하기 위한 카운터입니다.
    counter: AtomicU32,
    /// 생성된 커스텀 게임 대기실 집합
    pool: SkipMap<WorldId, Arc<CustomGameRoom>>,
    /// 활성화되지 않은 커스텀 게임 대기실 집합
    retires: Arc<Queue<Arc<CustomGameRoom>>>,
}

impl CustomGamePool {
    /// 싱글턴 인스턴스를 반환합니다.
    pub fn get_instance() -> &'static Self {
        static INSTANCE: OnceLock<CustomGamePool> = OnceLock::new();
        INSTANCE.get_or_init(|| CustomGamePool {
            counter: AtomicU32::new(rand::random()),
            pool: SkipMap::new(),
            retires: Arc::new(Queue::new()),
        })
    }

    /// 주어진 식별자에 해당하는 활성화된 커스텀 게임 대기실을 가져옵니다.
    pub fn get(&self, key: &WorldId) -> Option<Arc<CustomGameRoom>> {
        self.pool
            .get(key)
            .map(|item| item.clone())
            .filter(|room| room.is_activate())
    }

    /// 커스텀 게임 식별자를 생성합니다.
    fn generate_id(&self) -> WorldId {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

        let counter_bit = self.counter.fetch_add(1, MemOrdering::AcqRel) & 0xFFF;
        let time_bit = duration.subsec_millis() & 0xFF;

        WorldId::new(time_bit << 12 | counter_bit)
    }

    /// 새로운 커스텀 게임 대기실을 생성합니다.  
    pub fn create(
        &self,
        user_info: UserInfo,
        session: &Arc<Session>,
    ) -> (Arc<CustomGameRoom>, Vec<CustomGamePlayer>) {
        // 커스텀 게임 대기실을 할당받습니다.
        let room = match self.retires.pop() {
            Some(room) => room,
            None => {
                // 게임 월드 식별자를 할당 받습니다.
                let world_id = self.generate_id();
                // 커스텀 게임 대기실을 생성합니다.
                let room = Arc::new(CustomGameRoom::new(world_id));
                // 새로운 게임 월드를 풀 객체에 추가합니다.
                self.pool.insert(world_id, room.clone());
                room
            }
        };

        log::info!("CustomGameRoom({}) is allocated.", room.id());
        println!("CustomGameRoom({}) is allocated.", room.id());

        // 커스텀 게임 대기실을 초기화합니다.
        let players = room.reset(user_info, session);

        // 커스텀 게임 대기실을 실행합니다.
        tokio::spawn(running_loop(self.retires.clone(), room.clone()));

        (room, players)
    }
}

/// 커스텀 게임 대기실을 실행하는 루프 함수입니다.
async fn running_loop(retires: Arc<Queue<Arc<CustomGameRoom>>>, room: Arc<CustomGameRoom>) {
    // 활성화된 커스텀 게임 대기실을 실행합니다.
    while room.is_activate() {
        // 다른 태스크들이 실행될 기회를 주기 위해 양보
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }

    // 비활성화된 커스텀 게임 대기실을 회수합니다.
    log::info!("CustomGameRoom({}) is released.", room.id());
    println!("CustomGameRoom({}) is released.", room.id());
    retires.push(room);
}
