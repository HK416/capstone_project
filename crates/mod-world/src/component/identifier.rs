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
    pub fn alloc(self: &Arc<Self>) -> WorldID {
        let value = match self.recycle.pop() {
            Some(value) => value, 
            None => self.number.fetch_add(1, MemOrdering::AcqRel)
        };

        WorldID(UniqueId {value, allocator: Arc::downgrade(self) }.into())
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

impl Default for UniqueId {
    #[inline]
    fn default() -> Self {
        Self { 
            value: 0, 
            allocator: Weak::new(), 
        }
    }
}

impl Drop for UniqueId {
    fn drop(&mut self) {
        if let Some(allocator) = self.allocator.upgrade() {
            if self.value != 0 {
                allocator.retire(self.value);
            }
        }
    }
}




/// 게임 오브젝트의 식별자입니다.
#[derive(Clone)]
pub struct WorldID(Arc<UniqueId>);

impl Default for WorldID {
    #[inline]
    fn default() -> Self {
        Self(Arc::new(UniqueId::default()))
    }
}

impl Eq for WorldID { }

impl PartialEq<Self> for WorldID {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.value.eq(&other.0.value)
    }
}

impl Ord for WorldID {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.value.cmp(&other.0.value)
    }
}

impl PartialOrd<Self> for WorldID {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.value.partial_cmp(&other.0.value)
    }
}

impl Hash for WorldID {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.value.hash(state);
    }
}

impl std::fmt::Debug for WorldID {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(WorldID))
            .field(&self.0.value)
            .finish()
    }
}
