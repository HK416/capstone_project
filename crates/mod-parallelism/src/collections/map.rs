use std::borrow::Borrow;
use std::ops;
use std::ptr;
use std::sync::atomic;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering as MemOrdering;

use parking_lot::ReentrantMutex;
use parking_lot::ReentrantMutexGuard;

use crate::epoch::EBRGuard;
use crate::epoch::EBR;

/// Skip List의 최대 레벨입니다.
const MAX_LEVELS: usize = 10;

/// Skip List 최대 레벨의 인덱스입니다.
const MAX_LEVEL_INDEX: usize = MAX_LEVELS - 1;



/// `SkipMap`의 노드입니다.
/// 게으른 동기화(Lazy-synchronization)를 위한 데이터가 포함되어 있습니다.
pub struct Node<K, V> {
    key: Option<K>, // 값이 `None`인 경우 `head` 또는 `tail`노드
    value: Option<V>, 
    top_level: usize, 
    removed: AtomicBool, 
    fully_linked: AtomicBool, 
    next: [*mut Node<K, V>; MAX_LEVELS], 
    mtx: ReentrantMutex<()>, 
}

impl<K, V> Node<K, V> {
    /// 주어진 `key`, `val`, `top_level`로 노드를 생성합니다.
    /// 이 함수는 이전에 할당된 메모리 블록을 재사용할 수 있습니다.
    #[must_use]
    fn new(ebr_guard: &EBRGuard<'_, Self>, key: K, val: V, top_level: usize) -> Box<Self> {
        let mut node = ebr_guard.alloc();
        node.key = Some(key);
        node.value = Some(val);
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
            key: None, 
            value: None, 
            top_level: MAX_LEVEL_INDEX, 
            removed: AtomicBool::new(false), 
            fully_linked: AtomicBool::new(false), 
            next: [ptr::null_mut(); MAX_LEVELS], 
            mtx: ReentrantMutex::new(()), 
        }
    }
}

/// 무작위 값의 노드 레벨을 생성합니다.
#[must_use]
fn random_level() -> usize {
    let mut level = 0;
    while level < MAX_LEVEL_INDEX {
        if rand::random() {
            break;
        }
        level += 1;
    }
    return level;
}



/// `SkipMap`에 담겨있는 요소를 보호하기 위한 reference 가드입니다.
pub struct RefGuard<'a, K, V> {
    value: &'a V, 
    _lock_guard: ReentrantMutexGuard<'a, ()>, 
    _ebr_guard: EBRGuard<'a, Node<K, V>>
}

impl<'a, K, V> ops::Deref for RefGuard<'a, K, V> {
    type Target = V;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// `SkipMap`에 담겨있는 요소를 보호하기 위한 mutable 가드입니다.
pub struct MutGuard<'a, K, V> {
    value: &'a mut V, 
    _lock_guard: ReentrantMutexGuard<'a, ()>, 
    _ebr_guard: EBRGuard<'a, Node<K, V>>
}

impl<'a, K, V> ops::Deref for MutGuard<'a, K, V> {
    type Target = V;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, K, V> ops::DerefMut for MutGuard<'a, K, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
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
    len: AtomicUsize, 
    head: AtomicPtr<Node<K, V>>, 
    tail: AtomicPtr<Node<K, V>>, 
}

impl<K, V> SkipMap<K, V> {
    /// 새로운 `SkipMap`을 생성합니다.
    #[must_use]
    pub fn new() -> Self {
        let head = Box::into_raw(Box::new(Node::default()));
        let tail = Box::into_raw(Box::new(Node::default()));
        for lv in 0..MAX_LEVELS {
            unsafe { (*head).next[lv] = tail };
        }

        Self { 
            ebr: EBR::new(), 
            len: AtomicUsize::new(0), 
            head: AtomicPtr::new(head), 
            tail: AtomicPtr::new(tail)
        }
    }

    /// 주어진 `key`의 위치를 찾습니다.
    /// `prevs`, `currs`에 `key`의 이전, 이후 노드의 주소 값이 저장됩니다.
    /// 
    /// 주어진 `key`와 일치하는 노드를 찾지 못한 경우 `None`을 반환합니다.
    /// 
    unsafe fn find_position<Q: ?Sized>(
        &self, 
        key: &Q, 
        prevs: &mut [*mut Node<K, V>], 
        currs: &mut [*mut Node<K, V>]
    ) -> Option<usize> 
    where K: Borrow<Q> + Ord, Q: Ord {
        let mut found_level = None;
        prevs[MAX_LEVEL_INDEX] = self.head.load(MemOrdering::Relaxed);
        for lv in (0..MAX_LEVELS).rev() {
            if lv != MAX_LEVEL_INDEX {
                prevs[lv] = prevs[lv + 1];
            }

            currs[lv] = (*prevs[lv]).next[lv];

            while (*currs[lv]).key.as_ref().is_some_and(|k| k.borrow() < key) {
                prevs[lv] = currs[lv];
                currs[lv] = (*currs[lv]).next[lv];
            }

            if found_level.is_none() 
            && (*currs[lv]).key.as_ref().is_some_and(|k| k.borrow() == key) {
                found_level = Some(lv);
            }
        }

        found_level
    }

    /// 주어진 `key`와 `val`을 `SkipMap`에 넣습니다.
    /// 이미 `SkipMap`에 주어진 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    /// 
    #[inline]
    pub fn insert(&self, key: K, val: V) -> Option<V> 
    where K: Ord {
        let ebr_guard = self.ebr.pin();
        unsafe { self.insert_impl(&ebr_guard, key, val) }
    }
    
    unsafe fn insert_impl(
        &self, 
        ebr_guard: &EBRGuard<'_, Node<K, V>>, 
        key: K, 
        val: V
    ) -> Option<V> 
    where K: Ord {
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        loop {
            if let Some(found_level) = self.find_position(&key, &mut prevs, &mut currs) {
                if (*currs[found_level]).removed.load(MemOrdering::Relaxed) {
                    continue;
                }

                // fully linked일 때 까지 대기
                while !(*currs[found_level]).fully_linked.load(MemOrdering::Relaxed) {
                    std::hint::spin_loop();
                }

                let _lock_guard = (*currs[found_level]).mtx.lock();
                if (*currs[found_level]).removed.load(MemOrdering::Relaxed) {
                    continue;
                }

                return (*currs[found_level]).value.replace(val);
            }

            let top_level = random_level();

            let mut invalidate = false;
            let mut lock_guards = Vec::with_capacity(MAX_LEVELS);
            for lv in 0..=top_level {
                lock_guards.push((*prevs[lv]).mtx.lock());
                if (*prevs[lv]).removed.load(MemOrdering::Relaxed)
                || (*currs[lv]).removed.load(MemOrdering::Relaxed)
                || (*prevs[lv]).next[lv] != currs[lv] {
                    invalidate = true;
                    break;
                }
            }

            if invalidate {
                continue;
            }

            let node = Box::into_raw(Node::new(ebr_guard, key, val, top_level));
            for lv in 0..=top_level {
                (*node).next[lv] = currs[lv];
            }
            atomic::fence(MemOrdering::SeqCst);

            for lv in 0..=top_level {
                (*prevs[lv]).next[lv] = node;
            }
            atomic::fence(MemOrdering::SeqCst);
            
            (*node).fully_linked.store(true, MemOrdering::Relaxed);
            self.len.fetch_add(1, MemOrdering::AcqRel);
            return None;
        }
    }

    /// 주어진 `key`에 해당하는 노드를`SkipMap`에서 제거합니다.
    /// `SkipMap`에 주어진 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    /// 
    #[inline]
    pub fn remove<Q>(&self, key: &Q) -> Option<V> 
    where K: Borrow<Q> + Ord, Q: Ord {
        let ebr_guard = self.ebr.pin();
        unsafe { self.remove_impl(&ebr_guard, key) }
    }

    unsafe fn remove_impl<Q>(
        &self,
        ebr_guard: &EBRGuard<'_, Node<K, V>>, 
        key: &Q
    ) -> Option<V> 
    where K: Borrow<Q> + Ord, Q: Ord {
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        if let Some(found_level) = self.find_position(key, &mut prevs, &mut currs) {
            let victim = currs[found_level];
            if !(*victim).fully_linked.load(MemOrdering::Relaxed) { return None };
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };
            if (*victim).top_level != found_level { return None };

            let lock_guard = (*victim).mtx.lock();
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };
            (*victim).removed.store(true, MemOrdering::Relaxed);

            let top_level = (*victim).top_level;
            loop {
                let mut invalidate = false;
                let mut lock_guards = Vec::with_capacity(MAX_LEVELS);
                for lv in 0..=top_level {
                    lock_guards.push((*prevs[lv]).mtx.lock());
                    if (*prevs[lv]).removed.load(MemOrdering::Relaxed)
                    || (*prevs[lv]).next[lv] != victim {
                        invalidate = true;
                        break;
                    }
                }

                if invalidate {
                    drop(lock_guards);
                    self.find_position(&key, &mut prevs, &mut currs);
                    continue;
                }

                for lv in (0..=top_level).rev() {
                    (*prevs[lv]).next[lv] = (*victim).next[lv];
                }

                drop(lock_guards);
                drop(lock_guard);

                ebr_guard.dealloc(Box::from_raw(victim));
                self.len.fetch_sub(1, MemOrdering::AcqRel);
                return (*victim).value.take();
            }
        }
        None
    }

    /// 주어진 `key`가 `SkipMap`에 포함되어있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool 
    where K: Borrow<Q> + Ord, Q: Ord {
        let _ebr_guard = self.ebr.pin();
        unsafe { self.contains_key_impl(key) }
    }

    #[must_use]
    unsafe fn contains_key_impl<Q>(&self, key: &Q) -> bool 
    where K: Borrow<Q> + Ord, Q: Ord {
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        if let Some(found_level) = self.find_position(key, &mut prevs, &mut currs) {
            let victim = currs[found_level];
            return (*victim).fully_linked.load(MemOrdering::Relaxed)
            && !(*victim).removed.load(MemOrdering::Relaxed);
        }
        false
    }

    /// `SkipMap`에 포함된 요소의 개수를 반환합니다.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len.load(MemOrdering::Relaxed)
    }

    /// 주어진 `key`에 해당하는 노드의 값을 빌려옵니다.
    /// 
    /// # Performance Warning
    /// **이 함수는 병렬성이 없습니다.**
    /// 
    /// 반환되는 `RefGuard`는 내부적으로 뮤텍스 락을 소유하고 있으며, 
    /// 경쟁 상태를 방지하기 위해 `RefGuard`가 소멸될 때 까지 락을 유지합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get<'a, Q>(&'a self, key: &Q) -> Option<RefGuard<'a, K, V>> 
    where K: Borrow<Q> + Ord, Q: Ord {
        let ebr_guard = self.ebr.pin();
        unsafe { self.get_impl(ebr_guard, key) }
    }

    #[must_use]
    unsafe fn get_impl<'a, Q>(
        &'a self, 
        ebr_guard: EBRGuard<'a, Node<K, V>>, 
        key: &Q
    ) -> Option<RefGuard<'a, K, V>>
    where K: Borrow<Q> + Ord, Q: Ord {
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        if let Some(found_level) = self.find_position(key, &mut prevs, &mut currs) {
            let victim = currs[found_level];
            if !(*victim).fully_linked.load(MemOrdering::Relaxed) { return None };
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };
            if (*victim).top_level != found_level { return None };

            let lock_guard = (*victim).mtx.lock();
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };

            return Some(RefGuard {
                value: (*victim).value.as_ref().unwrap_unchecked(), 
                _lock_guard: lock_guard, 
                _ebr_guard: ebr_guard
            });
        }
        None
    }

    /// 주어진 `key`에 해당하는 노드의 값을 빌려옵니다.
    /// 
    /// # Performance Warning
    /// **이 함수는 병렬성이 없습니다.**
    /// 
    /// 반환되는 `MutGuard`는 내부적으로 뮤텍스 락을 소유하고 있으며, 
    /// 경쟁 상태를 방지하기 위해 `MutGuard`가 소멸될 때 까지 락을 유지합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get_mut<'a, Q>(&'a self, key: &Q) -> Option<MutGuard<'a, K, V>> 
    where K: Borrow<Q> + Ord, Q: Ord {
        let ebr_guard = self.ebr.pin();
        unsafe { self.get_mut_impl(ebr_guard, key) }
    }

    #[must_use]
    unsafe fn get_mut_impl<'a, Q>(
        &'a self, 
        ebr_guard: EBRGuard<'a, Node<K, V>>, 
        key: &Q
    ) -> Option<MutGuard<'a, K, V>>
    where K: Borrow<Q> + Ord, Q: Ord {
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        if let Some(found_level) = self.find_position(key, &mut prevs, &mut currs) {
            let victim = currs[found_level];
            if !(*victim).fully_linked.load(MemOrdering::Relaxed) { return None };
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };
            if (*victim).top_level != found_level { return None };

            let lock_guard = (*victim).mtx.lock();
            if (*victim).removed.load(MemOrdering::Relaxed) { return None };

            return Some(MutGuard {
                value: (*victim).value.as_mut().unwrap(), 
                _lock_guard: lock_guard, 
                _ebr_guard: ebr_guard
            });
        }
        None
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
mod test0 {
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
                        if result.is_none() {
                            survive[val as usize] += 1;
                        }
                    },
                    History::Remove { val, result } => {
                        if result.is_some() {
                            survive[val as usize] -= 1;
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
            } else if cnt == 0 && map.contains_key(&(num as u32)) {
                panic!("ERROR. The value {} should not exists.", num);
            } else if cnt == 1 && !map.contains_key(&(num as u32)) {
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
                    History::Remove { val, result: map.remove(&val) }
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

#[cfg(test)]
mod test1 {
    use std::thread;
    use std::sync::Arc;
    use super::SkipMap;

    const MAX_THREADS: usize = 16;
    const NUM_TESTS: usize = 10_000_000;
    
    enum History {
        Increase, 
        Decrease, 
    }
    
    fn check_history(historys: Vec<Vec<History>>, map: Arc<SkipMap<u32, i32>>) {
        let mut number = 0;
    
        for historys in historys {
            for history in historys {
                match history {
                    History::Increase => number += 1, 
                    History::Decrease => number -= 1
                };
            }
        }
    
        assert_eq!(number, map.remove(&0).unwrap());
    }
    
    fn validation_main(num_threads: usize, map: Arc<SkipMap<u32, i32>>) -> Vec<History> {
        let num_tests = NUM_TESTS / num_threads;
        let mut history = Vec::with_capacity(num_tests);
        for _ in 0..num_tests {
            if rand::random() {
                *(map.get_mut(&0).unwrap()) += 1;
                history.push(History::Increase);
            } else {
                *(map.get_mut(&0).unwrap()) -= 1;
                history.push(History::Decrease);
            }
        }
        history
    }
    
    #[test]
    fn check_consistency() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            let map = Arc::new(SkipMap::new());
            map.insert(0, 0);

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
