use std::{
    hash::{Hash, Hasher}, 
    sync::atomic::{AtomicU64, Ordering as MemOrdering}, 
    sync::{Arc, Weak}, 
};

use mod_parallelism::collections::Queue;



/// 게임 오브젝트 식별자를 생성하는 생성기입니다.
#[derive(Debug)]
pub struct IdGenerator {
    /// 현재 게임 오브젝트 식별자입니다.
    number: AtomicU64, 
    /// 재활용 가능한 게임 오브젝트 식별자입니다.
    recycle: Queue<u64>, 
}

impl IdGenerator {
    /// 새로운 게임 오브젝트 식별자 생성기를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Arc<Self> {
        Self { 
            number: AtomicU64::new(1), 
            recycle: Queue::new() 
        }.into()
    }

    /// 새로운 식별자를 할당받습니다.
    #[must_use]
    pub fn alloc(self: &Arc<Self>) -> ArenaID {
        let value = match self.recycle.pop() {
            Some(value) => value, 
            None => self.number.fetch_add(1, MemOrdering::AcqRel)
        };

        ArenaID(UniqueId {value, allocator: Arc::downgrade(self) }.into())
    }

    /// 할당한 식별자를 회수합니다.
    #[inline]
    fn retire(&self, value: u64) {
        self.recycle.push(value);
    }
}



/// 고유 식별자입니다.
#[derive(Debug, Clone)]
struct UniqueId {
    value: u64, 
    allocator: Weak<IdGenerator>, 
}

impl Drop for UniqueId {
    fn drop(&mut self) {
        if let Some(allocator) = self.allocator.upgrade() {
            allocator.retire(self.value);
        }
    }
}



/// 게임 오브젝트의 식별자입니다.
#[derive(Debug, Clone)]
pub struct ArenaID(Arc<UniqueId>);

impl Eq for ArenaID { }

impl PartialEq<Self> for ArenaID {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.value.eq(&other.0.value)
    }
}

impl Ord for ArenaID {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.value.cmp(&other.0.value)
    }
}

impl PartialOrd<Self> for ArenaID {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.value.partial_cmp(&other.0.value)
    }
}

impl Hash for ArenaID {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.value.hash(state);
    }
}
