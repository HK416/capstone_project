use std::sync::{Arc, OnceLock};

use mod_network::components::WorldId;
use mod_parallelism::collections::{Queue, SkipMap};

use super::CustomGameRoom;

/// 생성된 커스텀 게임을 관리하는 풀 객체입니다.
#[derive(Debug)]
pub struct CustomGamePool {
    /// 현재 활성화된 커스텀 게임 대기실 집합
    pool: SkipMap<WorldId, Arc<CustomGameRoom>>,
    /// 활성화되지 않은 커스텀 게임 대기실 집합
    retires: Queue<Arc<CustomGameRoom>>,
    /// 비활성화 요청 이벤트 대기열
    pool_handle: Arc<Queue<WorldId>>,
}

impl CustomGamePool {
    /// 싱글턴 인스턴스를 반환합니다.
    pub fn get_instance() -> &'static Self {
        static INSTANCE: OnceLock<CustomGamePool> = OnceLock::new();
        INSTANCE.get_or_init(|| CustomGamePool { 
            pool: SkipMap::new(), 
            retires: Queue::new(), 
            pool_handle: Arc::new(Queue::new()) 
        })
    }

    /// 주어진 식별자에 해당하는 활성화된 커스텀 게임 대기실을 가져옵니다.
    pub fn get(&self, key: &WorldId) -> Option<Arc<CustomGameRoom>> {
        self.pool.get(key).map(|item| item.clone())
    }
}
