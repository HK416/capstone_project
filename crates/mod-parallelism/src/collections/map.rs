use std::cmp;
use std::fmt;
use std::ptr;
use std::mem::MaybeUninit;
use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering as MemOrdering;

use crate::hazard::Collector;

/// 합성 포인터에서 주소 값을 추출하기 위한 비트 마스크입니다.
#[cfg(target_pointer_width = "32")]
const PTR_MASK: usize = 0xFFFFFFFE;

/// 합성 포인터에서 주소 값을 추출하기 위한 비트 마스크입니다.
#[cfg(target_pointer_width = "64")]
const PTR_MASK: usize = 0xFFFFFFFFFFFFFFFE;

/// 합성 포인터에서 Marking을 추출하기 위한 비트 마스크입니다.
const MARKING_MASK: usize = 0x01;

/// Skip List의 최대 레벨입니다.
const MAX_LEVELS: usize = 10;

/// Skip List 최대 레벨의 인덱스입니다.
const MAX_LEVEL_INDEX: usize = MAX_LEVELS - 1;




/// 주소 값과 Marking이 합쳐진 합성 포인터입니다.
struct Stamp<T> {
    inner: AtomicUsize, 
    _phantom: PhantomData<T>
}

impl<T> Stamp<T> {
    /// Marking되지 않고 주소 값이 `null`인 새로운 합성 포인터를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            inner: AtomicUsize::new(0), 
            _phantom: PhantomData, 
        }
    }

    /// Marking되지 않은 새로운 합성 포인터를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(ptr: *mut T) -> Self {
        Self { 
            inner: AtomicUsize::new(ptr as usize), 
            _phantom: PhantomData 
        }
    }

    /// 주소 값과 Marking을 동시에 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_ptr_with_marking(&self) -> (*mut T, bool) {
        let ptr = self.inner.load(MemOrdering::Relaxed);
        let marking = (ptr & MARKING_MASK) == MARKING_MASK;
        ((ptr & PTR_MASK) as *mut T, marking)
    }

    /// 주소 값을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_ptr(&self) -> *mut T {
        let ptr = self.inner.load(MemOrdering::Relaxed);
        (ptr & PTR_MASK) as *mut T
    }

    /// 주소 값을 설정합니다.
    #[inline]
    pub fn set_ptr(&self, p: *mut T) {
        self.inner.store(p as usize, MemOrdering::Relaxed)
    }

    /// `CAS` 연산을 사용하여 Stamp 값의 변경을 시도합니다.
    /// 
    /// 이미 다른 스레드가 먼저 Stamp를 변경하여 `CAS` 연산이 실패한 경우 `false`를 반환합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn try_change(&self, current_p: *mut T, new_p: *mut T, current_mark: bool, new_mark: bool) -> bool {
        let mut current = current_p as usize;
        if current_mark { current = current | MARKING_MASK };

        let mut new = new_p as usize;
        if new_mark { new = new | MARKING_MASK }

        self.inner.compare_exchange(
            current, 
            new, 
            MemOrdering::SeqCst, 
            MemOrdering::Relaxed
        ).is_ok()
    }
}

impl<T> Default for Stamp<T> {
    #[inline]
    fn default() -> Self {
        Self { 
            inner: AtomicUsize::new(0), 
            _phantom: PhantomData 
        }
    }
}

impl<T> fmt::Debug for Stamp<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (address, marking) = self.get_ptr_with_marking();
        f.debug_struct(stringify!(Stamp<T>))
            .field("Address", &address)
            .field("Marking", &marking)
            .finish()
    }
}



/// 키 값을 저장하는 자료형입니다.
#[derive(Debug)]
enum Key<K> {
    Head, 
    Tail, 
    Value(MaybeUninit<K>)
}

impl<K: Ord> Ord for Key<K> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match self {
            Key::Head => match other {
                Key::Head => cmp::Ordering::Equal, 
                _ => cmp::Ordering::Less, 
            }, 
            Key::Tail => match other {
                Key::Tail => cmp::Ordering::Equal, 
                _ => cmp::Ordering::Greater, 
            }, 
            Key::Value(lhs) => match other {
                Key::Head => cmp::Ordering::Greater, 
                Key::Tail => cmp::Ordering::Less, 
                Key::Value(rhs) => unsafe {
                    lhs.assume_init_ref().cmp(rhs.assume_init_ref())
                }, 
            }
        }
    }
}

impl<K: Ord> PartialOrd<Self> for Key<K> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Eq> Eq for Key<K> { }

impl<K: Eq> PartialEq<Self> for Key<K> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Key::Head => match other {
                Key::Head => true, 
                _ => false,
            }, 
            Key::Tail => match other {
                Key::Tail => true, 
                _ => false, 
            }, 
            Key::Value(lhs) => match other {
                Key::Value(rhs) => unsafe { 
                    lhs.assume_init_ref().eq(rhs.assume_init_ref())
                }, 
                _ => false, 
            },
        }
    }
}



/// Skip List로 구현된 `SkipMap`에서 사용하는 노드입니다.
#[derive(Debug)]
pub struct Node<K, V> {
    key: Key<K>, 
    value: MaybeUninit<V>, 
    top_level: usize, 
    next: [Stamp<Self>; MAX_LEVELS], 
}

impl<K, V> Node<K, V> {
    const ARRAY_REPEAT_VAL: Stamp<Self> = Stamp::zeroed();

    /// Skip List로 구현된 `SkipMap`의 `head` 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn head() -> Self {
        Self { 
            key: Key::Head, 
            value: MaybeUninit::uninit(), 
            top_level: MAX_LEVEL_INDEX, 
            next: [Self::ARRAY_REPEAT_VAL; MAX_LEVELS] 
        }
    }

    /// Skip List로 구현된 `SkipMap`의 `tail` 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn tail() -> Self {
        Self { 
            key: Key::Tail, 
            value: MaybeUninit::uninit(), 
            top_level: MAX_LEVEL_INDEX, 
            next: [Self::ARRAY_REPEAT_VAL; MAX_LEVELS] 
        }
    }

    /// Skip List로 구현된 `SkipMap`의 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(key: K, val: V) -> Self {
        Self { 
            key: Key::Value(MaybeUninit::new(key)), 
            value: MaybeUninit::new(val), 
            top_level: generate_top_level(), 
            next: [Self::ARRAY_REPEAT_VAL; MAX_LEVELS] 
        }
    } 
}

/// 노드의 최대 레벨을 생성합니다.
#[inline]
#[must_use]
fn generate_top_level() -> usize {
    let mut top_level = 0;
    while top_level < MAX_LEVEL_INDEX {
        if rand::random() {
            top_level += 1;
        } else {
            break;
        }
    }
    return top_level;
}



/// ### SkipMap
/// Lock-Free Skip List로 구현된 Map 자료구조입니다.
/// 
/// O(log n) ~ O(n)의 검색 성능을 가집니다.
/// 
pub struct SkipMap<K, V> {
    collector: Collector<Node<K, V>>, 
    head: AtomicPtr<Node<K, V>>, 
    tail: AtomicPtr<Node<K, V>>, 
}

impl<K, V> SkipMap<K, V> {
    /// 새로운 `SkipMap`을 생성합니다.
    #[must_use]
    pub fn new() -> Self {
        let head = Box::into_raw(Box::new(Node::head()));
        let tail = Box::into_raw(Box::new(Node::tail()));

        // Safety: head는 null이 아님.
        for next in unsafe { (*head).next.iter() } {
            next.set_ptr(tail);
        }

        Self { 
            collector: Collector::new(), 
            head: AtomicPtr::new(head), 
            tail: AtomicPtr::new(tail) 
        }
    }
}

impl<K, V> Drop for SkipMap<K, V> {
    fn drop(&mut self) {
        let head = self.head.load(MemOrdering::Relaxed);
        let tail = self.tail.load(MemOrdering::Relaxed);
        
        // Safety: head는 null이 아님.
        let mut ptr = unsafe { (*head).next[0].get_ptr() };
        while ptr != tail {
            let temp = ptr;
            // Safety: ptr은 null이 아님.
            ptr = unsafe { (*ptr).next[0].get_ptr() };
            // Safety: temp는 null이 아님.
            drop(unsafe { Box::from_raw(temp) })
        }

        // Safety: head는 null이 아님.
        drop(unsafe { Box::from_raw(head) });
        // Safety: tail은 null이 아님.
        drop(unsafe { Box::from_raw(tail) });
    }
}



#[cfg(test)]
mod tests {
    use std::thread::{self, spawn};
    use std::sync::Arc;

    use super::SkipMap;

    const MAX_NUM: usize = 10_000;
    const MAX_THREADS: usize = 16;
    const MAX_TESTS: usize = 10_000_000;

    fn thread_main(num_threads: usize, map: Arc<SkipMap<u32, u32>>) {
        // TODO
    }

    #[test]
    fn check_consistency() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            println!("Checking validation... (Threads={})", num_threads);

            let map = Arc::new(SkipMap::new());
            let handles: Vec<_> = (0..num_threads).into_iter()
                .map(|_| {
                    let map_cloned = map.clone();
                    thread::spawn(move || thread_main(num_threads, map_cloned))
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            num_threads *= 2;
        }
    }
}
