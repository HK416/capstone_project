use std::{
    collections::VecDeque,
    ptr::{null_mut, NonNull},
    sync::atomic::{AtomicPtr, AtomicU64, Ordering as MemOrdering},
    u64,
};

use thread_local::ThreadLocal;

use crate::backoff::Backoff;

/// ## Node
/// - T: 관리 대상 자료형
/// - M: 회수된 메모리 저장 용량
///
/// `ThreadList`의 노드입니다.
///
#[derive(Debug)]
struct Node<T, const M: usize> {
    data: AtomicPtr<ThreadData<T, M>>,
    next: AtomicPtr<Node<T, M>>,
}

impl<T, const M: usize> Node<T, M> {
    pub fn new(data: NonNull<ThreadData<T, M>>) -> Self {
        Self {
            data: AtomicPtr::new(data.as_ptr()),
            next: AtomicPtr::new(null_mut()),
        }
    }
}

impl<T, const M: usize> Drop for Node<T, M> {
    fn drop(&mut self) {
        unsafe { drop(Box::from_raw(self.data.load(MemOrdering::Relaxed))) };
    }
}

/// ## Thread List
/// - T: 관리 대상 자료형
/// - M: 회수된 메모리 저장 용량
///
/// 각 스레드의 `ThreadData`의 집합입니다.  
/// Lock-Free 연결 리스트로 구현되어 있으며, `ThreadList`에서는 새로운 노드를 생성만 합니다.  
/// 생성된 노드는 모든 작업이 끝난 후 `ThreadList`가 제거될 때 제거됩니다.
///
#[derive(Debug)]
struct ThreadList<T, const M: usize> {
    head: AtomicPtr<Node<T, M>>,
}

impl<T, const M: usize> ThreadList<T, M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// 새로운 `ThreadData`를 추가합니다.
    pub fn append(&self, ptr: NonNull<ThreadData<T, M>>) {
        let mut backoff = Backoff::new();
        let new = Box::into_raw(Box::new(Node::new(ptr)));
        loop {
            let current = self.head.load(MemOrdering::Relaxed);
            unsafe { (*new).next.store(current, MemOrdering::Relaxed) };

            // `current`가 `head`인지 확인합니다. (`CAS` 명령어 사용을 최소화 하기 위함)
            if current != self.head.load(MemOrdering::Relaxed) {
                continue;
            }

            if self
                .head
                .compare_exchange(current, new, MemOrdering::SeqCst, MemOrdering::Relaxed)
                .is_ok()
            {
                break;
            }

            backoff.wait();
        }
    }

    /// `ThreadData`의 가장 작고, 큰 Epoch 값을 반환합니다.
    pub fn get_min_max_epoch(&self) -> (u64, u64) {
        let mut minimum = u64::MAX;
        let mut maximum = u64::MIN;

        let mut curr = self.head.load(MemOrdering::Relaxed);
        while !curr.is_null() {
            let local_data = unsafe { (*curr).data.load(MemOrdering::Relaxed) };
            let epoch = unsafe { (*local_data).epoch_count.load(MemOrdering::Relaxed) };
            minimum = minimum.min(epoch);
            maximum = maximum.max(epoch);
            curr = unsafe { (*curr).next.load(MemOrdering::Relaxed) };
        }

        (minimum, maximum)
    }
}

impl<T, const M: usize> Default for ThreadList<T, M> {
    fn default() -> Self {
        Self {
            head: AtomicPtr::new(null_mut()),
        }
    }
}

impl<T, const M: usize> Drop for ThreadList<T, M> {
    fn drop(&mut self) {
        let mut curr = self.head.load(MemOrdering::Relaxed);
        while !curr.is_null() {
            let temp = curr;
            curr = unsafe { (*curr).next.load(MemOrdering::Relaxed) };
            unsafe { drop(Box::from_raw(temp)) };
        }
    }
}

/// ## Retire
/// - T: 관리 대상 자료형
///
/// 회수된 메모리의 주소 값과 시대를 가집니다.
///
struct Retire<T> {
    ptr: Box<T>,
    epoch: u64,
}

/// ## Thread Data
/// - T: 관리 대상 자료형
/// - M: 회수된 메모리 저장 용량
///
/// 각 스레드가 관리하는 스레드 로컬 데이터입니다.  
/// 다른 스레드는 읽기만 가능합니다.
///
struct ThreadData<T, const M: usize> {
    free_list: VecDeque<Retire<T>>,
    epoch_count: AtomicU64,
}

impl<T, const M: usize> ThreadData<T, M> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, const M: usize> Default for ThreadData<T, M> {
    fn default() -> Self {
        Self {
            free_list: VecDeque::with_capacity(M),
            epoch_count: AtomicU64::new(0),
        }
    }
}

/// ## Collector
/// - T: 관리 대상 자료형
/// - M: 회수된 메모리 저장 용량
///
/// Epoch Based Reclamation 기법을 사용한 쓰레기 수집기입니다.  
/// 멀티 스레드에서 Dangling 포인터 참조와 ABA 문제를 해결하기 위해 사용합니다.
///
#[derive(Debug)]
pub struct Collector<T, const M: usize = 32> {
    epoch_counter: AtomicU64,
    thread_list: ThreadList<T, M>,
    thread_local: ThreadLocal<AtomicPtr<ThreadData<T, M>>>,
}

impl<T, const M: usize> Collector<T, M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `ScopeGuard`를 생성합니다.
    pub fn scope(&self) -> ScopeGuard<'_, T, M> {
        ScopeGuard::new(self)
    }

    /// 현재 스레드의 로컬 데이터를 가져옵니다.
    fn get_local_data(&self) -> NonNull<ThreadData<T, M>> {
        let ptr = self
            .thread_local
            .get_or(|| {
                let ptr = Box::into_raw(Box::new(ThreadData::new()));
                self.thread_list
                    .append(unsafe { NonNull::new_unchecked(ptr) });
                AtomicPtr::new(ptr)
            })
            .load(MemOrdering::Relaxed);
        unsafe { NonNull::new_unchecked(ptr) }
    }
}

impl<T, const M: usize> Default for Collector<T, M> {
    fn default() -> Self {
        Self {
            epoch_counter: AtomicU64::new(1),
            thread_list: ThreadList::new(),
            thread_local: ThreadLocal::new(),
        }
    }
}

/// ## Scope Guard
/// - T: 관리 대상 자료형
/// - M: 회수된 메모리 저장 용량
///
/// 범위를 벗어나면 자동으로 현재 스레드의 Epoch를 초기화하는 가드입니다.
///
#[derive(Debug)]
pub struct ScopeGuard<'a, T, const M: usize> {
    collector: &'a Collector<T, M>,
}

impl<'a, T, const M: usize> ScopeGuard<'a, T, M> {
    fn new(collector: &'a Collector<T, M>) -> Self {
        let epoch = collector.epoch_counter.fetch_add(1, MemOrdering::AcqRel);
        let local_data = unsafe { collector.get_local_data().as_ref() };
        local_data.epoch_count.store(epoch, MemOrdering::Relaxed);
        Self { collector }
    }

    /// `Collector`로 부터 메모리를 할당받습니다.
    /// 회수된 메모리가 재사용 될 수 있습니다.
    pub fn alloc<F>(&self, func: F) -> Box<T>
    where
        F: FnOnce() -> T,
    {
        let local_data = unsafe { self.collector.get_local_data().as_mut() };
        if local_data.free_list.is_empty() {
            return Box::new(func());
        }

        let (min_epoch, _) = self.collector.thread_list.get_min_max_epoch();
        let front = unsafe { local_data.free_list.front().unwrap_unchecked() };
        if min_epoch <= front.epoch {
            return Box::new(func());
        }

        let mut front = unsafe { local_data.free_list.pop_front().unwrap_unchecked() };
        *front.ptr = func();
        front.ptr
    }

    /// `Collector`로 메모리를 회수합니다.
    /// 회수된 메모리는 재사용 될 수 있습니다.
    pub fn dealloc(&self, ptr: Box<T>) {
        let (min_epoch, max_epoch) = self.collector.thread_list.get_min_max_epoch();
        let local_data = unsafe { self.collector.get_local_data().as_mut() };

        // 오래된 메모리를 제거합니다.
        loop {
            if local_data.free_list.len() < M {
                break;
            }; // 저장 공간이 남아있는 경우 루프를 빠져나옴.
            if local_data.free_list.is_empty() {
                break;
            }; // 회수된 메모리가 없는 경우 루프를 빠져나옴.

            // 회수된 메모리에 다른 스레드가 접근할 가능성이 있는 경우 루프를 빠져나옴.
            if local_data
                .free_list
                .front()
                .is_some_and(|front| min_epoch <= front.epoch)
            {
                break;
            }

            local_data.free_list.pop_front();
        }

        // 회수된 메모리를 추가합니다.
        local_data.free_list.push_back(Retire {
            ptr,
            epoch: max_epoch,
        });
    }
}

impl<'a, T, const M: usize> Clone for ScopeGuard<'a, T, M> {
    fn clone(&self) -> Self {
        ScopeGuard {
            collector: &self.collector,
        }
    }
}

impl<'a, T, const M: usize> Drop for ScopeGuard<'a, T, M> {
    fn drop(&mut self) {
        let local_data = unsafe { self.collector.get_local_data().as_ref() };
        local_data.epoch_count.store(0, MemOrdering::Relaxed);
    }
}
