use std::cmp;
use std::ptr;
use std::mem::MaybeUninit;
use std::sync::atomic;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering as MemOrdering;
use std::sync::Mutex;

use crate::epoch::EBRGuard;
use crate::epoch::EBR;

/// Skip List의 최대 레벨입니다.
const MAX_LEVELS: usize = 10;

/// Skip List 최대 레벨의 인덱스입니다.
const MAX_LEVEL_INDEX: usize = MAX_LEVELS - 1;





/// `SkipMap`의 키 값입니다.
/// `head`와 `tail`을 비교하기 위해 사용합니다.
#[derive(Debug)]
enum Key<K> {
    Head, 
    Tail, 
    Val(K)
}

impl<K: Eq> Eq for Key<K> { }

impl<K: Eq> PartialEq<Self> for Key<K> {
    #[inline]
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
            Key::Val(lhs) => match other {
                Key::Val(rhs) => lhs.eq(rhs), 
                _ => false, 
            },
        }
    }
}

impl<K: Ord> Ord for Key<K> {
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
            Key::Val(lhs) => match other {
                Key::Head => cmp::Ordering::Greater, 
                Key::Tail => cmp::Ordering::Less, 
                Key::Val(rhs) => lhs.cmp(rhs), 
            }
        }
    }
}

impl<K: Ord> PartialOrd<Self> for Key<K> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}





/// `SkipMap`의 노드입니다.
/// 게으른 동기화(Lazy-synchronization)를 위한 데이터가 포함되어 있습니다.
pub struct Node<K, V> {
    key: Key<K>, 
    value: MaybeUninit<V>, 
    top_level: usize, 
    removed: AtomicBool, 
    fully_linked: AtomicBool, 
    next: [*mut Node<K, V>; MAX_LEVELS], 
    mtx: [Mutex<()>; MAX_LEVELS], 
}

impl<K, V> Node<K, V> {
    const ARRAY_REPEAT_VAL: Mutex<()> = Mutex::new(());

    /// 무작위 값의 노드 레벨을 생성합니다.
    #[must_use]
    fn generate_random_level() -> usize {
        let mut top_level = 0;
        while top_level < MAX_LEVEL_INDEX {
            if rand::random() {
                break;
            }
            top_level += 1;
        }
        return top_level;
    }

    /// `SkipMap`의 `head` 노드를 생성합니다.
    #[must_use]
    fn head() -> Box<Self> {
        Box::new(Self { 
            key: Key::Head, 
            value: MaybeUninit::uninit(), 
            top_level: MAX_LEVEL_INDEX, 
            removed: AtomicBool::new(false), 
            fully_linked: AtomicBool::new(true), 
            next: [ptr::null_mut(); MAX_LEVELS], 
            mtx: [Self::ARRAY_REPEAT_VAL; MAX_LEVELS] 
        })
    }

    /// `SkipMap`의 `tail` 노드를 생성합니다.
    #[must_use]
    fn tail() -> Box<Self> {
        Box::new(Self { 
            key: Key::Tail, 
            value: MaybeUninit::uninit(), 
            top_level: MAX_LEVEL_INDEX, 
            removed: AtomicBool::new(false), 
            fully_linked: AtomicBool::new(true), 
            next: [ptr::null_mut(); MAX_LEVELS], 
            mtx: [Self::ARRAY_REPEAT_VAL; MAX_LEVELS] 
        })
    }

    /// 주어진 `key`, `val`, `top_level`로 노드를 생성합니다.
    /// 이 함수는 이전에 할당된 메모리 블록을 재사용할 수 있습니다.
    #[must_use]
    fn new(ebr_pin: EBRGuard<'_, Self>, key: Key<K>, val: V, top_level: usize) -> Box<Self> {
        let mut node = ebr_pin.alloc();
        node.key = key;
        node.value = MaybeUninit::new(val);
        node.top_level = top_level;
        node.removed.store(false, MemOrdering::Relaxed);
        node.fully_linked.store(false, MemOrdering::Relaxed);
        for lv in 0..MAX_LEVELS {
            node.next[lv] = ptr::null_mut();
        };
        return node;
    }
}

impl<K, V> Default for Node<K, V> {
    #[inline]
    fn default() -> Self {
        Self { 
            key: Key::Head, 
            value: MaybeUninit::uninit(), 
            top_level: MAX_LEVEL_INDEX, 
            removed: AtomicBool::new(false), 
            fully_linked: AtomicBool::new(false), 
            next: [ptr::null_mut(); MAX_LEVELS], 
            mtx: [Self::ARRAY_REPEAT_VAL; MAX_LEVELS]
        }
    }
}





/// ### Skip Map
/// 게으른 동기화(Lazy-synchronization)로 구현된 Skip List를 사용하는 Map 자료구조입니다.
/// 
/// O(log n) ~ O(n)의 검색 속도를 가집니다.
/// 
#[derive(Debug)]
pub struct SkipMap<K, V> {
    ebr: EBR<Node<K, V>>, 
    head: AtomicPtr<Node<K, V>>, 
    tail: AtomicPtr<Node<K, V>>, 
}

impl<K: Ord, V> SkipMap<K, V> {
    /// 주어진 `key`의 위치를 찾습니다.
    /// `prevs`, `currs`에 `key`의 이전, 이후 노드의 주소 값이 저장됩니다.
    /// 
    /// 주어진 `key`와 일치하는 노드를 찾지 못한 경우 `None`을 반환합니다.
    /// 
    unsafe fn find_position(
        &self, 
        key: &Key<K>, 
        prevs: &mut [*mut Node<K, V>], 
        currs: &mut [*mut Node<K, V>]
    ) -> Option<usize> {
        let mut found_level = None;
        prevs[MAX_LEVEL_INDEX] = self.head.load(MemOrdering::Relaxed);
        for lv in (0..MAX_LEVELS).rev() {
            if lv != MAX_LEVEL_INDEX {
                prevs[lv] = prevs[lv + 1];
            }

            currs[lv] = (*prevs[lv]).next[lv];
            while (*currs[lv]).key < (*key) {
                prevs[lv] = currs[lv];
                currs[lv] = (*currs[lv]).next[lv];
            }

            if found_level.is_none() && (*currs[lv]).key == (*key) {
                found_level = Some(lv);
            }
        }
        return found_level;
    }


    
    /// 주어진 `key`와 `val`을 `SkipMap`에 넣습니다.
    /// 이미 `SkipMap`에 주어진 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    /// 
    unsafe fn insert_impl(&self, key: K, val: V) -> Option<V> {
        let key = Key::Val(key);
        let ebr_pin = self.ebr.pin();
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        loop {
            if let Some(found_level) = self.find_position(&key, &mut prevs, &mut currs) {
                if (*currs[found_level]).removed.load(MemOrdering::Relaxed) {
                    continue;
                }

                // fully linke 일 때 까지 대기
                while !(*currs[found_level]).fully_linked.load(MemOrdering::Relaxed) { }

                let _guard = (*currs[found_level]).mtx[0].lock().unwrap();
                let old = (*currs[found_level]).value.assume_init_read();
                (*currs[found_level]).value.write(val);
                return Some(old);
            }

            let top_level = Node::<K, V>::generate_random_level();
            let mut invalidate = false;
            let mut locked_mtx = Vec::with_capacity(MAX_LEVELS);
            for lv in 0..=top_level {
                locked_mtx.push((*prevs[lv]).mtx[lv].lock().unwrap_unchecked());
                if (*prevs[lv]).removed.load(MemOrdering::Relaxed)
                || (*currs[lv]).removed.load(MemOrdering::Relaxed)
                || (*prevs[lv]).next[lv] != currs[lv] {
                    invalidate = true;
                    break;
                }
            }

            if invalidate {
                for guard in locked_mtx {
                    drop(guard);
                }
                continue;
            }

            let node = Box::into_raw(Node::new(ebr_pin, key, val, top_level));
            for lv in 0..=top_level {
                (*node).next[lv] = currs[lv];
            }
            atomic::fence(MemOrdering::SeqCst);

            for lv in 0..=top_level {
                (*prevs[lv]).next[lv] = node;
            }
            atomic::fence(MemOrdering::SeqCst);

            (*node).fully_linked.store(true, MemOrdering::Relaxed);
            for guard in locked_mtx {
                drop(guard);
            }

            return None;
        };
    }



    /// 주어진 `key`에 해당하는 노드를`SkipMap`에서 제거합니다.
    /// `SkipMap`에 주어진 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    /// 
    unsafe fn remove_impl(&self, key: K) -> Option<V> {
        let key = Key::Val(key);
        let ebr_pin = self.ebr.pin();
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];

        if let Some(found_level) = self.find_position(&key, &mut prevs, &mut currs) {
            let victim = currs[found_level];
            if (*victim).fully_linked.load(MemOrdering::Relaxed) { return None };
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };
            if (*victim).top_level != found_level { return None };

            let guard = (*victim).mtx[0].lock();
            if (*victim).removed.load(MemOrdering::Relaxed) {
                return None;
            }

            (*victim).removed.store(true, MemOrdering::Relaxed);
            let top_level = (*victim).top_level;
            loop {
                let mut invalidate = false;
                let mut locked_mtx = Vec::with_capacity(MAX_LEVELS);
                for lv in 0..=top_level {
                    locked_mtx.push((*prevs[lv]).mtx[lv].lock().unwrap());
                    if (*prevs[lv]).removed.load(MemOrdering::Relaxed)
                    || (*prevs[lv]).next[lv] != victim {
                        invalidate = true;
                        break;
                    }
                }

                if invalidate {
                    for guard in locked_mtx {
                        drop(guard);
                    }
                    self.find_position(&key, &mut prevs, &mut currs);
                    continue;
                }

                for lv in (0..=top_level).rev() {
                    (*prevs[lv]).next[lv] = (*victim).next[lv];
                }

                for guard in locked_mtx {
                    drop(guard);
                }
                drop(guard);

                let value = (*victim).value.assume_init_read();
                ebr_pin.dealloc(Box::from_raw(victim));
                return Some(value);
            }
        }

        return None;
    }



    /// 주어진 `key`가 `SkipMap`에 포함되어있는지 여부를 반환합니다.
    unsafe fn contains_key_impl(&self, key: K) -> bool {
        let key = Key::Val(key);
        let _ebr_pin = self.ebr.pin();
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];

        if let Some(found_level) = self.find_position(&key, &mut prevs, &mut currs) {
            let target = currs[found_level];
            return (*target).fully_linked.load(MemOrdering::Relaxed) 
            && !(*target).removed.load(MemOrdering::Relaxed);
        }

        return false;
    }
}

impl<K: Ord, V> SkipMap<K, V> {
    /// 새로운 `SkipMap`을 생성합니다.
    #[must_use]
    pub fn new() -> Self {
        let head = Box::into_raw(Node::head());
        let tail = Box::into_raw(Node::tail());
        for lv in 0..MAX_LEVELS {
            unsafe { (*head).next[lv] = tail };
        }

        Self { 
            ebr: EBR::new(), 
            head: AtomicPtr::new(head), 
            tail: AtomicPtr::new(tail)
        }
    }

    /// 주어진 `key`와 `val`을 `SkipMap`에 넣습니다.
    /// 이미 `SkipMap`에 주어진 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    /// 
    #[inline]
    pub fn insert(&self, key: K, val: V) -> Option<V> {
        unsafe { self.insert_impl(key, val) }
    }

    /// 주어진 `key`에 해당하는 노드를`SkipMap`에서 제거합니다.
    /// `SkipMap`에 주어진 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    /// 
    #[inline]
    pub fn remove(&self, key: K) -> Option<V> {
        unsafe { self.remove_impl(key) }
    }

    /// 주어진 `key`가 `SkipMap`에 포함되어있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn contains(&self, key: K) -> bool {
        unsafe { self.contains_key_impl(key) }
    }
}

impl<K, V> Drop for SkipMap<K, V> {
    fn drop(&mut self) {
        let head = self.head.load(MemOrdering::Relaxed);
        let tail = self.tail.load(MemOrdering::Relaxed);
        let mut ptr = unsafe { (*head).next[0] };
        while ptr != tail {
            let temp = ptr;
            ptr = unsafe { (*ptr).next[0] };
            drop(unsafe { Box::from_raw(temp) });
        }

        drop(unsafe { Box::from_raw(head) });
        drop(unsafe { Box::from_raw(tail) });
    }
}





#[cfg(test)]
mod tests {
    use std::thread;
    use std::sync::Arc;

    use super::SkipMap;

    const MAX_NUM: usize = 10_000;
    const MAX_THREADS: usize = 16;
    const NUM_TESTS: usize = 10_000_000;
    
    enum History {
        Insert { val: u32, result: Option<u32> }, 
        Remove { val: u32, result: Option<u32> }, 
    }
    
    
    
    fn check_history(historys: Vec<Vec<History>>, map: Arc<SkipMap<u32, u32>>) {
        let mut survive = [0; MAX_NUM + 1];
    
        for historys in historys {
            for history in historys {
                match history {
                    History::Insert { val, result } => {
                        survive[val as usize] += 1;
                        if let Some(old) = result {
                            survive[old as usize] -= 1;
                        }
                    },
                    History::Remove { val, result } => {
                        if let Some(own) = result {
                            assert_eq!(val, own);
                            survive[own as usize] -= 1;
                        }
                    }, 
                };
            }
        }
    
        for (num, cnt) in survive.into_iter().enumerate() {
            if cnt < 0 {
                panic!("ERROR. The value {} removed while it is not in the set.", num);
            } else if cnt > 1 {
                panic!("ERROR. The value {} is added while the set already have it.", num);
            } else if cnt == 0 && map.contains(num as u32) {
                panic!("ERROR. The value {} should not exists.", num);
            } else if cnt == 1 && !map.contains(num as u32) {
                panic!("ERROR. The value {} should exists.", num);
            }
        }
    }
    
    fn validation_main(num_threads: usize, map: Arc<SkipMap<u32, u32>>) -> Vec<History> {
        let num_tests = NUM_TESTS / num_threads;
        (0..num_tests).into_iter()
            .map(|_| {
                let mut val = rand::random();
                val = val % (MAX_NUM as u32 + 1);
                if rand::random() {
                    History::Insert { val, result: map.insert(val, val) }
                } else {
                    History::Remove { val, result: map.remove(val) }
                }
            })
            .collect()
    }
    
    #[test]
    fn check_consistency() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            let map = Arc::new(SkipMap::new());
            let handles: Vec<_> = (0..num_threads).into_iter()
                .map(|_| {
                    let map_cloned = map.clone();
                    thread::spawn(move || validation_main(num_threads, map_cloned))
                })
                .collect();
    
            let historys: Vec<_> = handles.into_iter()
                .map(|handle| handle.join().unwrap())
                .collect();
            check_history(historys, map);
    
            num_threads *= 2;
        }
    }    
}
