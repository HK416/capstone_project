use std::fmt;
use std::ptr;
use std::collections::VecDeque;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering as MemOrdering;

use thread_local::ThreadLocal;



/// ### Epoch List
/// 각 스레드의 Epoch를 모아놓은 연결 리스트 자료구조입니다.
/// 
/// Lock-Free 연결 리스트로 구현되어 있으며, Epoch의 할당만 합니다.
/// 
/// 새로운 스레드가 추가될 경우 Epoch List에 등록되며, 작업이 끝날 때 해제됩니다.
/// 
struct EpochList<T> {
    head: AtomicPtr<Epoch<T>>, 
}

impl<T> EpochList<T> {
    /// 비어있는 Epoch List를 반환합니다.
    #[inline]
    #[must_use]
    const fn new() -> Self {
        Self { head: AtomicPtr::new(ptr::null_mut()) }
    }

    /// Epoch List에 스레드 로컬 데이터를 Lock-Free하게 추가합니다.
    unsafe fn add(&self, ptr: *mut Local<T>) {
        let new = Epoch::new(ptr);
        loop {
            let current = self.head.load(MemOrdering::Relaxed);
            (*new).next.store(current, MemOrdering::Relaxed);
            if current != self.head.load(MemOrdering::Relaxed) {
                continue;
            }
            if self.head.compare_exchange(current, new, MemOrdering::SeqCst, MemOrdering::Relaxed).is_ok() {
                break;
            }
        }
    }
}

impl<T> Drop for EpochList<T> {
    fn drop(&mut self) {
        // drop은 모든 작업이 끝난 후 싱글스레드에서 호출됩니다.
        // Epoch를 정리합니다.
        let mut ptr = self.head.load(MemOrdering::Relaxed);
        while !ptr.is_null() {
            let temp = ptr;
            ptr = unsafe { (*ptr).next.load(MemOrdering::Relaxed) };
            unsafe { drop(Box::from_raw(temp)) };
        }
    }
}





/// ### Epoch
/// 각 스레드의 Free List와 Epoch Counter를 저장한 노드입니다.
struct Epoch<T> {
    /// 스레드 로컬 저장소 데이터의 주소값 입니다.
    local: AtomicPtr<Local<T>>, 

    /// 다음 Epoch의 주소 값입니다.
    next: AtomicPtr<Epoch<T>>, 
}

impl<T> Epoch<T> {
    /// 새로운 Epoch를 메모리에서 할당 받아 생성합니다.
    /// 
    /// # Warning 
    /// 할당 받은 메모리는 자동으로 회수되지 않습니다.
    /// 따라서 사용이 끝난 후 직접 메모리를 반환해야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new(p: *mut Local<T>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            local: AtomicPtr::new(p), 
            next: AtomicPtr::new(ptr::null_mut())
        }))
    }
}

impl<T> Drop for Epoch<T> {
    fn drop(&mut self) {
        // drop은 모든 작업이 끝난 후 싱글스레드에서 호출됩니다.
        // Local을 정리합니다.
        let ptr = self.local.load(MemOrdering::Relaxed);
        unsafe { drop(Box::from_raw(ptr)) }
    }
}





/// ### Local
/// 스레드 로컬 저장소에서 사용하는 데이터입니다.
struct Local<T> {
    /// 해제할 메모리 블록의 정보가 담긴 Queue 자료구조입니다.
    /// 
    /// 오직 하나의 스레드만 접근할 수 있으며, 각 스레드에서
    /// 할당된 메모리 블록을 재사용할 때 사용됩니다.
    /// 
    /// 작업이 끝날 때 대기열에 담긴 메모리 블록이 해제됩니다.
    /// 
    freelist: VecDeque<(u64, Box<T>)>, 

    /// 각 스레드의 현재 시대(Epoch) 정보입니다.
    counter: AtomicU64, 
}

impl<T> Local<T> {
    /// 새로운 스레드 로컬 저장소 데이터를 메모리에서 할당 받아 생성합니다.
    /// 
    /// # Warning 
    /// 할당 받은 메모리는 자동으로 회수되지 않습니다.
    /// 따라서 사용이 끝난 후 직접 메모리를 반환해야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new() -> *mut Self {
        Box::into_raw(Box::new(Self { 
            freelist: VecDeque::with_capacity(64), 
            counter: AtomicU64::new(0) 
        }))
    }
}





/// ### Epoch Based Reclamation
/// 시대(Epoch)를 이용한 간단한 쓰레기 수집기입니다.
/// 
/// 쓰레기 수집을 통해 멀티 스레드에서 dangling 포인터 참조 또는 ABA 문제를 해결할 수 있습니다.
/// 
pub struct EBR<T> {
    counter: AtomicU64, 
    epoch_list: EpochList<T>, 
    tls: ThreadLocal<AtomicPtr<Local<T>>>, 
}

impl<T> EBR<T> {
    /// 새로운 Epoch Based Reclamation를 사용하는 쓰레기 수집기를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { 
            counter: AtomicU64::new(1), 
            epoch_list: EpochList::new(), 
            tls: ThreadLocal::new(), 
        }
    }

    /// 스레드 로컬 데이터를 가져옵니다.
    #[must_use]
    fn get_local(&self) -> &mut Local<T> {
        let ptr = self.tls.get_or(|| unsafe {
            // 새로운 스레드 로컬 데이터를 할당 받습니다.
            let ptr = Local::new();

            // Epoch List에 스레드 로컬 데이터를 Lock-Free하게 추가합니다.
            self.epoch_list.add(ptr);

            AtomicPtr::new(ptr)
        }).load(MemOrdering::Relaxed);
        unsafe { &mut *ptr }
    }

    /// 각 스레드의 시대(Epoch)중 가장 큰 값을 가져옵니다.
    #[must_use]
    fn get_max_epoch(&self) -> u64 {
        let mut max_epoch = u64::MIN;
        unsafe {
            let mut p = self.epoch_list.head.load(MemOrdering::Relaxed);
            while !p.is_null() {
                let local_ptr = (*p).local.load(MemOrdering::Relaxed);
                let cnt = (*local_ptr).counter.load(MemOrdering::Relaxed);
                max_epoch = max_epoch.max(cnt);
                p = (*p).next.load(MemOrdering::Relaxed);
            }
        };
        return max_epoch;
    }

    /// 각 스레드의 시대(Epoch)중 0이 아닌 가장 작은 값을 가져옵니다.
    #[must_use]
    fn get_min_epoch(&self) -> u64 {
        let mut min_epoch = u64::MAX;
        unsafe {
            let mut p = self.epoch_list.head.load(MemOrdering::Relaxed);
            while !p.is_null() {
                let local_ptr = (*p).local.load(MemOrdering::Relaxed);
                let cnt = (*local_ptr).counter.load(MemOrdering::Relaxed);
                if cnt != 0 {
                    min_epoch = min_epoch.min(cnt);
                }
                p = (*p).next.load(MemOrdering::Relaxed);
            }
        };
        return min_epoch;
    }

    pub fn pin(&self) -> EBRGuard<T> {
        let local = self.get_local();
        let my_epoch = self.counter.fetch_add(1, MemOrdering::AcqRel);
        local.counter.store(my_epoch, MemOrdering::Relaxed);
        EBRGuard::new(self)
    }
}

impl<T> fmt::Debug for EBR<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Epoch Based Reclamation GC")
    }
}





/// ### Epoch Based Reclamation Guard
/// 범위를 벗어날 때 자동으로 현재 스레드의 Epoch를 초기화하는 가드입니다.
pub struct EBRGuard<'a, T> {
    inner: &'a EBR<T>, 
}

impl<'a, T> EBRGuard<'a, T> {
    /// 새로운 EBR 가드를 생성합니다.
    #[inline]
    #[must_use]
    fn new(inner: &'a EBR<T>) -> Self {
        Self { inner }
    }

    /// 주어진 메모리 블록을 회수합니다.
    #[inline]
    pub fn dealloc(&self, blob: Box<T>) {
        let local = self.inner.get_local();
        let max_epoch = self.inner.get_max_epoch();
        local.freelist.push_back((max_epoch, blob));
    }
}

impl<'a, T: Default> EBRGuard<'a, T> {
    /// 메모리 블록을 할당 받습니다.
    #[must_use]
    pub fn alloc(&self) -> Box<T> {
        let local = self.inner.get_local();
        if local.freelist.is_empty() {
            return Box::new(T::default());
        }

        let curr_epoch = self.inner.get_min_epoch();
        let (remove_point, _) = unsafe { (*local).freelist.front().unwrap_unchecked() };
        if curr_epoch.le(remove_point) {
            return Box::new(T::default());
        }

        let (_, node) = unsafe { local.freelist.pop_front().unwrap_unchecked() };
        return node;
    }
}

impl<'a, T> Drop for EBRGuard<'a, T> {
    fn drop(&mut self) {
        let local = self.inner.get_local();
        local.counter.store(0, MemOrdering::Relaxed);
    }
}
