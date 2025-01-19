use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
    ptr::null_mut,
    sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering as MemOrdering},
};

use ahash::HashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::epoch::{Collector, ScopeGuard};

/// ## Node
/// - N: 최대 Skip-List 레벨
///
/// Skip-List로 구현된 `Map`에서 사용되는 노드입니다.
///
#[derive(Debug)]
pub struct Node<K, V, const N: usize> {
    key: Option<K>,
    value: Option<V>,
    top_level: usize,
    removed: AtomicBool,
    fully_linked: AtomicBool,
    next: [*mut Node<K, V, N>; N],
    mtx: RwLock<()>,
}

impl<K, V, const N: usize> Default for Node<K, V, N> {
    fn default() -> Self {
        Self {
            key: None,
            value: None,
            top_level: N - 1,
            removed: AtomicBool::new(false),
            fully_linked: AtomicBool::new(false),
            next: [null_mut(); N],
            mtx: RwLock::new(()),
        }
    }
}

/// 무작위 값의 레벨을 생성합니다.
fn random_level<const N: usize>() -> usize {
    let mut level = 0;
    while level < N - 1 {
        if rand::random() {
            break;
        }
        level += 1;
    }
    level
}

/// ## SkipMap
/// - K: 키 자료형
/// - V: 값 자료형
/// - N: 최대 Skip-List 레벨
/// - M: 회수된 메모리 저장 용량
///
/// 게으른 동기화(Lazy-synchronization)로 구현된 Skip-List를 사용하는 Map 자료구조입니다.  
/// 참고: n개의 노드에서 Skip-List의 효율적인 최대 레벨: log2(n)
///
#[derive(Debug)]
pub struct SkipMap<K, V, const N: usize = 12, const M: usize = 32> {
    collector: Box<Collector<Node<K, V, N>, M>>,
    head: AtomicPtr<Node<K, V, N>>,
    tail: AtomicPtr<Node<K, V, N>>,
    len: AtomicUsize,
}

impl<K, V, const N: usize, const M: usize> SkipMap<K, V, N, M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Map`이 비어있는 경우 `true`를 반환합니다.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `Map` 요소의 개수를 가져옵니다.
    pub fn len(&self) -> usize {
        self.len.load(MemOrdering::Acquire)
    }

    /// 주어진 키 값의 위치를 찾습니다.
    fn find_position<Q: ?Sized>(
        &self,
        key: &Q,
        prevs: &mut [*mut Node<K, V, N>],
        currs: &mut [*mut Node<K, V, N>],
    ) -> Option<usize>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let mut found_level = None;
        prevs[N - 1] = self.head.load(MemOrdering::Relaxed);
        for lv in (0..N).rev() {
            if lv != (N - 1) {
                prevs[lv] = prevs[lv + 1];
            }

            currs[lv] = unsafe { (*prevs[lv]).next[lv] };

            while compare_lt_key(currs[lv], key) {
                prevs[lv] = currs[lv];
                currs[lv] = unsafe { (*currs[lv]).next[lv] };
            }

            if found_level.is_none() && compare_eq_key(currs[lv], key) {
                found_level = Some(lv);
            }
        }

        found_level
    }

    /// `Map`에 요소를 추가합니다. 이미 해당 `key`값이 존재하는 경우 기존의 값을 반환합니다.
    pub fn insert(&self, key: K, val: V) -> Option<V>
    where
        K: Ord,
    {
        let scope = self.collector.scope();
        let mut prevs = [null_mut(); N];
        let mut currs = [null_mut(); N];
        loop {
            let found_level = self.find_position(&key, &mut prevs, &mut currs);
            if let Some(found_level) = found_level {
                if is_removed(currs[found_level]) {
                    continue;
                }

                while !is_fully_linked(currs[found_level]) {
                    std::hint::spin_loop();
                }

                let _lock_guard = unsafe { (*currs[found_level]).mtx.write() };
                if is_removed(currs[found_level]) {
                    continue;
                }

                let old_value = unsafe { (*currs[found_level]).value.replace(val) };
                return old_value;
            }

            let top_level = random_level::<N>();
            let mut invalidate = false;
            let mut lock_guards = HashMap::default();
            for lv in 0..=top_level {
                if !lock_guards.contains_key(&prevs[lv]) {
                    lock_guards.insert(prevs[lv], unsafe { (*prevs[lv]).mtx.write() });
                }

                if is_removed(prevs[lv])
                    || is_removed(currs[lv])
                    || !is_next_node(lv, prevs[lv], currs[lv])
                {
                    invalidate = true;
                    break;
                }
            }

            if invalidate {
                continue;
            }

            let mut node = scope.alloc(move || Node {
                key: Some(key),
                value: Some(val),
                top_level,
                removed: AtomicBool::new(false),
                fully_linked: AtomicBool::new(false),
                next: [null_mut(); N],
                mtx: RwLock::new(()),
            });
            for lv in 0..=top_level {
                node.next[lv] = currs[lv];
            }
            std::sync::atomic::fence(MemOrdering::SeqCst);

            let node = Box::into_raw(node);
            for lv in 0..=top_level {
                unsafe { (*prevs[lv]).next[lv] = node };
            }
            std::sync::atomic::fence(MemOrdering::SeqCst);

            unsafe { (*node).fully_linked.store(true, MemOrdering::Relaxed) };
            drop(lock_guards);

            self.len.fetch_add(1, MemOrdering::AcqRel);
            return None;
        }
    }

    /// `Map`에 요소를 제거합니다. `key`값 에 해당하는 요소가 없는 경우 `None`을 반환합니다.
    pub fn remove<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let scope = self.collector.scope();
        let mut prevs = [null_mut(); N];
        let mut currs = [null_mut(); N];
        let found_level = self.find_position(key, &mut prevs, &mut currs);
        if let Some(found_level) = found_level {
            let victim = currs[found_level];
            if !is_fully_linked(victim) {
                return None;
            };
            if is_removed(victim) {
                return None;
            };
            if !compare_eq_lv(victim, found_level) {
                return None;
            };

            let lock_guard = unsafe { (*victim).mtx.write() };
            if is_removed(victim) {
                return None;
            };
            unsafe { (*victim).removed.store(true, MemOrdering::Relaxed) };

            let top_level = unsafe { (*victim).top_level };
            loop {
                let mut invalidate = false;
                let mut lock_guards = HashMap::default();
                for lv in 0..=top_level {
                    if !lock_guards.contains_key(&prevs[lv]) {
                        lock_guards.insert(prevs[lv], unsafe { (*prevs[lv]).mtx.write() });
                    }

                    if is_removed(prevs[lv]) || !is_next_node(lv, prevs[lv], victim) {
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
                    unsafe { (*prevs[lv]).next[lv] = (*victim).next[lv] };
                }

                drop(lock_guards);
                drop(lock_guard);

                let value = unsafe { (*victim).value.take().unwrap_unchecked() };
                scope.dealloc(unsafe { Box::from_raw(victim) });
                self.len.fetch_sub(1, MemOrdering::AcqRel);
                return Some(value);
            }
        } else {
            return None;
        }
    }

    /// `Map`에 `key`에 해당하는 요소를 가져옵니다.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<Ref<K, V, N, M>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let scope = self.collector.scope();
        let mut prevs = [null_mut(); N];
        let mut currs = [null_mut(); N];
        let found_level = self.find_position(key, &mut prevs, &mut currs);
        if let Some(found_level) = found_level {
            let victim = currs[found_level];
            if !is_fully_linked(victim) {
                return None;
            };
            if is_removed(victim) {
                return None;
            };
            if !compare_eq_lv(victim, found_level) {
                return None;
            };

            let lock_guard = unsafe { (*victim).mtx.read() };
            if is_removed(victim) {
                return None;
            };

            let value = unsafe { (*victim).value.as_ref().unwrap_unchecked() };
            return Some(Ref::new(value, lock_guard, scope));
        }

        return None;
    }

    /// `Map`에 `key`에 해당하는 요소를 가져옵니다.
    pub fn get_mut<Q: ?Sized>(&self, key: &Q) -> Option<Mut<K, V, N, M>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let scope = self.collector.scope();
        let mut prevs = [null_mut(); N];
        let mut currs = [null_mut(); N];
        let found_level = self.find_position(key, &mut prevs, &mut currs);
        if let Some(found_level) = found_level {
            let victim = currs[found_level];
            if !is_fully_linked(victim) {
                return None;
            };
            if is_removed(victim) {
                return None;
            };
            if !compare_eq_lv(victim, found_level) {
                return None;
            };

            let lock_guard = unsafe { (*victim).mtx.write() };
            if is_removed(victim) {
                return None;
            };

            let value = unsafe { (*victim).value.as_mut().unwrap_unchecked() };
            return Some(Mut::new(value, lock_guard, scope));
        }

        return None;
    }

    /// `Map`에 `key`에 해당하는 요소가 존재하는지 여부를 반환합니다.
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let scope = self.collector.scope();
        let mut prevs = [null_mut(); N];
        let mut currs = [null_mut(); N];
        let found_level = self.find_position(key, &mut prevs, &mut currs);
        if let Some(found_level) = found_level {
            let victim = currs[found_level];
            let exists = is_fully_linked(victim) && !is_removed(victim);
            drop(scope);
            exists
        } else {
            drop(scope);
            false
        }
    }

    /// `Map`의 키 값을 순회하는 반복자를 반환합니다.
    pub fn keys(&self) -> Keys<'_, K, V, N, M> {
        let scope = self.collector.scope();
        Keys {
            scope,
            tail: self.tail.load(MemOrdering::Relaxed),
            curr: self.head.load(MemOrdering::Relaxed),
        }
    }

    /// `Map`의 값을 순회하는 반복자를 반환합니다.
    pub fn values(&self) -> Values<'_, K, V, N, M> {
        let scope = self.collector.scope();
        Values {
            scope,
            tail: self.tail.load(MemOrdering::Relaxed),
            curr: self.head.load(MemOrdering::Relaxed),
        }
    }

    /// `Map`의 값을 순회하는 반복자를 반환합니다.
    pub fn values_mut(&self) -> ValuesMut<'_, K, V, N, M> {
        let scope = self.collector.scope();
        ValuesMut {
            scope,
            tail: self.tail.load(MemOrdering::Relaxed),
            curr: self.head.load(MemOrdering::Relaxed),
        }
    }

    /// `Map`의 키와 값을 순회하는 반복자를 반환합니다.
    pub fn iter(&self) -> Iter<'_, K, V, N, M> {
        let scope = self.collector.scope();
        Iter {
            scope,
            tail: self.tail.load(MemOrdering::Relaxed),
            curr: self.head.load(MemOrdering::Relaxed),
        }
    }

    /// `Map`의 키와 값을 순회하는 반복자를 반환합니다.
    pub fn iter_mut(&self) -> IterMut<'_, K, V, N, M> {
        let scope = self.collector.scope();
        IterMut {
            scope,
            tail: self.tail.load(MemOrdering::Relaxed),
            curr: self.head.load(MemOrdering::Relaxed),
        }
    }
}

impl<K, V, const N: usize, const M: usize> Default for SkipMap<K, V, N, M> {
    fn default() -> Self {
        let tail = Box::into_raw(Box::new(Node {
            key: None,
            value: None,
            top_level: N - 1,
            removed: AtomicBool::new(false),
            fully_linked: AtomicBool::new(true),
            next: [null_mut(); N],
            mtx: RwLock::new(()),
        }));
        let head = Box::into_raw(Box::new(Node {
            key: None,
            value: None,
            top_level: N - 1,
            removed: AtomicBool::new(false),
            fully_linked: AtomicBool::new(true),
            next: [tail; N],
            mtx: RwLock::new(()),
        }));

        Self {
            collector: Box::new(Collector::new()),
            head: AtomicPtr::new(head),
            tail: AtomicPtr::new(tail),
            len: AtomicUsize::new(0),
        }
    }
}

impl<K, V, const N: usize, const M: usize> Drop for SkipMap<K, V, N, M> {
    fn drop(&mut self) {
        let mut curr = self.head.load(MemOrdering::Relaxed);
        while !curr.is_null() {
            let temp = curr;
            curr = unsafe { (*curr).next[0] };
            unsafe { drop(Box::from_raw(temp)) };
        }
    }
}

/// `Map`에 포함된 키 값입니다.
#[derive(Debug)]
pub struct Key<'a, K, V, const N: usize, const M: usize> {
    key: &'a K,
    _lock: RwLockReadGuard<'a, ()>,
    _scope: ScopeGuard<'a, Node<K, V, N>, M>,
}

impl<'a, K, V, const N: usize, const M: usize> Key<'a, K, V, N, M> {
    fn new(
        key: &'a K,
        lock: RwLockReadGuard<'a, ()>,
        scope: ScopeGuard<'a, Node<K, V, N>, M>,
    ) -> Self {
        Self {
            key,
            _lock: lock,
            _scope: scope,
        }
    }

    pub fn get(&self) -> &K {
        self.key
    }

    /// 내부 값을 가져옵니다.
    ///
    /// # Warnings
    /// 이 함수는 `Key`에 포함된 `RwLockReadGuard`와 `ScopeGuard`를 해제합니다.  
    /// **따라서 반환된 값의 안전성을 보장할 수 없습니다.**
    ///
    pub unsafe fn into_inner(self) -> &'a K {
        self.key
    }
}

impl<'a, K, V, const N: usize, const M: usize> Deref for Key<'a, K, V, N, M> {
    type Target = K;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// `Map`에 포함된 값입니다.
#[derive(Debug)]
pub struct Ref<'a, K, V, const N: usize, const M: usize> {
    value: &'a V,
    _lock: RwLockReadGuard<'a, ()>,
    _scope: ScopeGuard<'a, Node<K, V, N>, M>,
}

impl<'a, K, V, const N: usize, const M: usize> Ref<'a, K, V, N, M> {
    fn new(
        value: &'a V,
        lock: RwLockReadGuard<'a, ()>,
        scope: ScopeGuard<'a, Node<K, V, N>, M>,
    ) -> Self {
        Self {
            value,
            _lock: lock,
            _scope: scope,
        }
    }

    pub fn get(&self) -> &V {
        self.value
    }

    /// 내부 값을 가져옵니다.
    ///
    /// # Warnings
    /// 이 함수는 `RefValue`에 포함된 `RwLockReadGuard`와 `ScopeGuard`를 해제합니다.  
    /// **따라서 반환된 값의 안전성을 보장할 수 없습니다.**
    ///
    pub unsafe fn into_inner(self) -> &'a V {
        self.value
    }
}

impl<'a, K, V, const N: usize, const M: usize> Deref for Ref<'a, K, V, N, M> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// `Map`에 포함된 값입니다.
#[derive(Debug)]
pub struct Mut<'a, K, V, const N: usize, const M: usize> {
    value: &'a mut V,
    _lock: RwLockWriteGuard<'a, ()>,
    _scope: ScopeGuard<'a, Node<K, V, N>, M>,
}

impl<'a, K, V, const N: usize, const M: usize> Mut<'a, K, V, N, M> {
    fn new(
        value: &'a mut V,
        lock: RwLockWriteGuard<'a, ()>,
        scope: ScopeGuard<'a, Node<K, V, N>, M>,
    ) -> Self {
        Self {
            value,
            _lock: lock,
            _scope: scope,
        }
    }

    pub fn get(&self) -> &V {
        self.value
    }

    pub fn get_mut(&mut self) -> &mut V {
        self.value
    }

    /// 내부 값을 가져옵니다.
    ///
    /// # Warnings
    /// 이 함수는 `RefValue`에 포함된 `RwLockWriteGuard`와 `ScopeGuard`를 해제합니다.  
    /// 따라서 반환된 값의 스레드 안전성을 보장할 수 없습니다.
    ///
    pub unsafe fn into_inner(self) -> &'a mut V {
        self.value
    }
}

impl<'a, K, V, const N: usize, const M: usize> Deref for Mut<'a, K, V, N, M> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, K, V, const N: usize, const M: usize> DerefMut for Mut<'a, K, V, N, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// `Map`에 포함된 키와 값 입니다.
#[derive(Debug)]
pub struct Pair<'a, K, V, const N: usize, const M: usize> {
    key: &'a K,
    value: &'a V,
    _lock: RwLockReadGuard<'a, ()>,
    _scope: ScopeGuard<'a, Node<K, V, N>, M>,
}

impl<'a, K, V, const N: usize, const M: usize> Pair<'a, K, V, N, M> {
    fn new(
        key: &'a K,
        value: &'a V,
        lock: RwLockReadGuard<'a, ()>,
        scope: ScopeGuard<'a, Node<K, V, N>, M>,
    ) -> Self {
        Self {
            key,
            value,
            _lock: lock,
            _scope: scope,
        }
    }

    pub fn key(&self) -> &K {
        self.key
    }

    pub fn value(&self) -> &V {
        self.value
    }

    pub fn get(&self) -> (&K, &V) {
        (self.key, self.value)
    }
}

/// `Map`에 포함된 키와 값 입니다.
#[derive(Debug)]
pub struct PairMut<'a, K, V, const N: usize, const M: usize> {
    key: &'a K,
    value: &'a mut V,
    _lock: RwLockWriteGuard<'a, ()>,
    _scope: ScopeGuard<'a, Node<K, V, N>, M>,
}

impl<'a, K, V, const N: usize, const M: usize> PairMut<'a, K, V, N, M> {
    fn new(
        key: &'a K,
        value: &'a mut V,
        lock: RwLockWriteGuard<'a, ()>,
        scope: ScopeGuard<'a, Node<K, V, N>, M>,
    ) -> Self {
        Self {
            key,
            value,
            _lock: lock,
            _scope: scope,
        }
    }

    pub fn key(&self) -> &K {
        self.key
    }

    pub fn value(&self) -> &V {
        self.value
    }

    pub fn value_mut(&mut self) -> &mut V {
        self.value
    }

    pub fn get(&self) -> (&K, &V) {
        (self.key, self.value)
    }

    pub fn get_mut(&mut self) -> (&K, &mut V) {
        (self.key, self.value)
    }
}

/// `Map`의 키 값을 순회하는 반복자입니다.
#[derive(Debug)]
pub struct Keys<'a, K, V, const N: usize, const M: usize> {
    scope: ScopeGuard<'a, Node<K, V, N>, M>,
    tail: *mut Node<K, V, N>,
    curr: *mut Node<K, V, N>,
}

impl<'a, K, V, const N: usize, const M: usize> Iterator for Keys<'a, K, V, N, M> {
    type Item = Key<'a, K, V, N, M>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 현재 노드가 `tail`인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 다음 노드로 이동합니다.
            self.curr = unsafe { (*self.curr).next[0] };

            // 이동한 노드가 `tail`노드인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 노드가 유효한지 확인합니다.
            if is_fully_linked(self.curr) && !is_removed(self.curr) {
                // 현제 노드의 읽기 락을 획득합니다.
                let lock = unsafe { (*self.curr).mtx.read() };

                // 현재 노드가 제거됐는지 확인합니다.
                if is_removed(self.curr) {
                    continue;
                }

                let key = unsafe { (*self.curr).key.as_ref().unwrap_unchecked() };
                return Some(Key::new(key, lock, self.scope.clone()));
            }
        }
    }
}

/// `Map`의 값을 순회하는 반복자입니다.
#[derive(Debug)]
pub struct Values<'a, K, V, const N: usize, const M: usize> {
    scope: ScopeGuard<'a, Node<K, V, N>, M>,
    tail: *mut Node<K, V, N>,
    curr: *mut Node<K, V, N>,
}

impl<'a, K, V, const N: usize, const M: usize> Iterator for Values<'a, K, V, N, M> {
    type Item = Ref<'a, K, V, N, M>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 현재 노드가 `tail`인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 다음 노드로 이동합니다.
            self.curr = unsafe { (*self.curr).next[0] };

            // 이동한 노드가 `tail`노드인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 노드가 유효한지 확인합니다.
            if is_fully_linked(self.curr) && !is_removed(self.curr) {
                // 현제 노드의 읽기 락을 획득합니다.
                let lock = unsafe { (*self.curr).mtx.read() };

                // 현재 노드가 제거됐는지 확인합니다.
                if is_removed(self.curr) {
                    continue;
                }

                let value = unsafe { (*self.curr).value.as_ref().unwrap_unchecked() };
                return Some(Ref::new(value, lock, self.scope.clone()));
            }
        }
    }
}

/// `Map`의 값을 순회하는 반복자입니다.
#[derive(Debug)]
pub struct ValuesMut<'a, K, V, const N: usize, const M: usize> {
    scope: ScopeGuard<'a, Node<K, V, N>, M>,
    tail: *mut Node<K, V, N>,
    curr: *mut Node<K, V, N>,
}

impl<'a, K, V, const N: usize, const M: usize> Iterator for ValuesMut<'a, K, V, N, M> {
    type Item = Mut<'a, K, V, N, M>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 현재 노드가 `tail`인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 다음 노드로 이동합니다.
            self.curr = unsafe { (*self.curr).next[0] };

            // 이동한 노드가 `tail`노드인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 노드가 유효한지 확인합니다.
            if is_fully_linked(self.curr) && !is_removed(self.curr) {
                // 현제 노드의 쓰기 락을 획득합니다.
                let lock = unsafe { (*self.curr).mtx.write() };

                // 현재 노드가 제거됐는지 확인합니다.
                if is_removed(self.curr) {
                    continue;
                }

                let value = unsafe { (*self.curr).value.as_mut().unwrap_unchecked() };
                return Some(Mut::new(value, lock, self.scope.clone()));
            }
        }
    }
}

/// `Map`의 키와 값을 순회하는 반복자입니다.
#[derive(Debug)]
pub struct Iter<'a, K, V, const N: usize, const M: usize> {
    scope: ScopeGuard<'a, Node<K, V, N>, M>,
    tail: *mut Node<K, V, N>,
    curr: *mut Node<K, V, N>,
}

impl<'a, K, V, const N: usize, const M: usize> Iterator for Iter<'a, K, V, N, M> {
    type Item = Pair<'a, K, V, N, M>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 현재 노드가 `tail`인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 다음 노드로 이동합니다.
            self.curr = unsafe { (*self.curr).next[0] };

            // 이동한 노드가 `tail`노드인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 노드가 유효한지 확인합니다.
            if is_fully_linked(self.curr) && !is_removed(self.curr) {
                // 현제 노드의 읽기 락을 획득합니다.
                let lock = unsafe { (*self.curr).mtx.read() };

                // 현재 노드가 제거됐는지 확인합니다.
                if is_removed(self.curr) {
                    continue;
                }

                let key = unsafe { (*self.curr).key.as_ref().unwrap_unchecked() };
                let value = unsafe { (*self.curr).value.as_ref().unwrap_unchecked() };
                return Some(Pair::new(key, value, lock, self.scope.clone()));
            }
        }
    }
}

/// `Map`의 키와 값을 순회하는 반복자입니다.
#[derive(Debug)]
pub struct IterMut<'a, K, V, const N: usize, const M: usize> {
    scope: ScopeGuard<'a, Node<K, V, N>, M>,
    tail: *mut Node<K, V, N>,
    curr: *mut Node<K, V, N>,
}

impl<'a, K, V, const N: usize, const M: usize> Iterator for IterMut<'a, K, V, N, M> {
    type Item = PairMut<'a, K, V, N, M>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // 현재 노드가 `tail`인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 다음 노드로 이동합니다.
            self.curr = unsafe { (*self.curr).next[0] };

            // 이동한 노드가 `tail`노드인지 확인합니다.
            if self.curr == self.tail {
                return None;
            }

            // 노드가 유효한지 확인합니다.
            if is_fully_linked(self.curr) && !is_removed(self.curr) {
                // 현제 노드의 쓰기 락을 획득합니다.
                let lock = unsafe { (*self.curr).mtx.write() };

                // 현재 노드가 제거됐는지 확인합니다.
                if is_removed(self.curr) {
                    continue;
                }

                let key = unsafe { (*self.curr).key.as_ref().unwrap_unchecked() };
                let value = unsafe { (*self.curr).value.as_mut().unwrap_unchecked() };
                return Some(PairMut::new(key, value, lock, self.scope.clone()));
            }
        }
    }
}

// 노드의 키 값이 주어진 키 값보다 작은 경우 `true`를 반환하는 함수
fn compare_lt_key<K, V, const N: usize, Q: ?Sized>(curr: *mut Node<K, V, N>, key: &Q) -> bool
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    unsafe { (*curr).key.as_ref().is_some_and(|k| k.borrow() < key) }
}

// 노드의 키 값이 주어진 키 값과 같은 경우 `true`를 반환하는 함수
fn compare_eq_key<K, V, const N: usize, Q: ?Sized>(curr: *mut Node<K, V, N>, key: &Q) -> bool
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    unsafe { (*curr).key.as_ref().is_some_and(|k| k.borrow() == key) }
}

// 노드의 `top_level`과 `lv`이 같은지 여부를 반환하는 함수
fn compare_eq_lv<K, V, const N: usize>(node: *mut Node<K, V, N>, lv: usize) -> bool {
    unsafe { (*node).top_level == lv }
}

// 노드가 삭제되었는지 여부를 반환하는 함수
fn is_removed<K, V, const N: usize>(node: *mut Node<K, V, N>) -> bool {
    unsafe { (*node).removed.load(MemOrdering::Relaxed) }
}

// 노드가 모두 연결됐는지 여부를 반환하는 함수
fn is_fully_linked<K, V, const N: usize>(node: *mut Node<K, V, N>) -> bool {
    unsafe { (*node).fully_linked.load(MemOrdering::Relaxed) }
}

// 레벨이 `lv`일 때 `next`가 `prev`의 다음 노드인지 여부를 반환하는 함수
fn is_next_node<K, V, const N: usize>(
    lv: usize,
    prev: *mut Node<K, V, N>,
    next: *mut Node<K, V, N>,
) -> bool {
    unsafe { (*prev).next[lv] == next }
}

#[cfg(test)]
mod test_0 {
    //! `Map`에 요소의 추가와 삭제가 올바른지 확인합니다.
    //!
    //! 검증 방법
    //! 1. 각 스레드에서 공유되는 `Map`에 랜덤하게 추가와 삭제를 진행합니다.
    //! 2. 각 스레드에서 추가와 삭제를 진행한 기록을 분석하여 중복되거나 존재하지 않는 요소가 있는지 살펴봅니다.
    //!
    use std::{sync::Arc, thread};

    use super::*;

    const MAX_THREADS: usize = 16;
    const NUM_RANGE: usize = 100_000;
    const NUM_TESTS: usize = 2_500_000;

    enum Record {
        Insert { val: u32, result: bool },
        Remove { val: u32, result: bool },
        Get,
        GetMut,
    }

    fn check_history(historys: Vec<Vec<Record>>, map: Arc<SkipMap<u32, u32, 16, 64>>) {
        let mut survive = vec![0; NUM_RANGE];
        for history in historys {
            for record in history {
                match record {
                    Record::Insert { val, result } if result => {
                        survive[val as usize] += 1;
                    }
                    Record::Remove { val, result } if result => {
                        survive[val as usize] -= 1;
                    }
                    _ => {}
                }
            }
        }

        for (num, cnt) in survive.into_iter().enumerate() {
            if cnt < 0 {
                panic!(
                    "ERROR. The value {} removed while it is not in the set.",
                    num
                );
            } else if cnt > 1 {
                panic!(
                    "ERROR. The value {} is added while the set already have it.",
                    num
                );
            } else if cnt == 0 && map.contains_key(&(num as u32)) {
                panic!("ERROR. The value {} should not exists.", num);
            } else if cnt == 1 && !map.contains_key(&(num as u32)) {
                panic!("ERROR. The value {} should exists.", num);
            }
        }
    }

    fn thread_main(num_threads: usize, map: Arc<SkipMap<u32, u32, 16, 64>>) -> Vec<Record> {
        let num_tests = NUM_TESTS / num_threads;
        let mut history = Vec::with_capacity(num_tests);
        for _ in 0..num_tests {
            let op = rand::random::<u8>() % 4;
            let val = rand::random::<u32>() % (NUM_RANGE as u32);
            let record = match op {
                0 => Record::Insert {
                    val,
                    result: map.insert(val, val).is_none(),
                },
                1 => Record::Remove {
                    val,
                    result: map.remove(&val).is_some(),
                },
                2 => {
                    if map.get(&val).is_some() {
                        // Windows에서 std::thread::sleep 동작이 달라 수정함.
                        for _ in 0..256 {
                            std::hint::spin_loop()
                        }
                    }
                    Record::Get
                }
                3 => {
                    if map.get_mut(&val).is_some() {
                        // Windows에서 std::thread::sleep 동작이 달라 수정함.
                        for _ in 0..256 {
                            std::hint::spin_loop()
                        }
                    }
                    Record::GetMut
                }
                _ => panic!("out of bounds"),
            };
            history.push(record);
        }
        history
    }

    #[test]
    fn check_validation() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            let map: Arc<SkipMap<u32, u32, 16, 64>> = Arc::new(SkipMap::new());
            let mut handles = Vec::with_capacity(num_threads);

            for _ in 0..num_threads {
                let map_cloned = map.clone();
                handles.push(thread::spawn(move || thread_main(num_threads, map_cloned)));
            }

            let mut historys = Vec::with_capacity(num_threads);
            for handle in handles {
                historys.push(handle.join().unwrap());
            }

            check_history(historys, map);

            num_threads *= 2;
        }
    }
}

#[cfg(test)]
mod test_1 {
    //! `Map`에서 가져온 요소가 원자적인지 확인합니다.
    //!
    //! 검증 방법
    //! 1. `Map`에 하나의 요소를 삽입합니다.
    //! 2. 각 스레드에서 공유되는 `Map`에 하나의 요소에 접근하여 연산을 수행합니다.
    //! 3. 각 스레드에서 수행한 연산 기록을 분석하여 `Map`의 요소 값과 분석한 값이 일치하는지 확인합니다.
    //!
    use std::{sync::Arc, thread};

    use super::*;

    const MAX_THREADS: usize = 16;
    const NUM_TESTS: usize = 2_500_000;

    enum Record {
        Add,
        Sub,
        Read,
    }

    fn check_history(historys: Vec<Vec<Record>>, map: Arc<SkipMap<u32, i32>>) {
        let mut count = 0;
        for history in historys {
            for record in history {
                match record {
                    Record::Add => count += 1,
                    Record::Sub => count -= 1,
                    _ => {}
                }
            }
        }

        let val = map.remove(&0).unwrap();
        assert_eq!(count, val);
    }

    fn thread_main(num_threads: usize, map: Arc<SkipMap<u32, i32>>) -> Vec<Record> {
        let num_tests = NUM_TESTS / num_threads;
        let mut history = Vec::with_capacity(num_tests);
        for _ in 0..num_tests {
            let op = rand::random::<u8>() % 3;
            let record = match op {
                0 => {
                    let mut val = map.get_mut(&0).unwrap();
                    *val += 1;
                    Record::Add
                }
                1 => {
                    let mut val = map.get_mut(&0).unwrap();
                    *val -= 1;
                    Record::Sub
                }
                2 => {
                    map.get(&0).unwrap();
                    // Windows에서 std::thread::sleep 동작이 달라 수정함.
                    for _ in 0..256 {
                        std::hint::spin_loop()
                    }
                    Record::Read
                }
                _ => panic!("out of bounds"),
            };
            history.push(record);
        }
        history
    }

    #[test]
    fn check_validation() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            let map = Arc::new(SkipMap::new());
            map.insert(0, 0);

            let mut handles = Vec::with_capacity(num_threads);
            for _ in 0..num_threads {
                let map_cloned = map.clone();
                handles.push(thread::spawn(move || thread_main(num_threads, map_cloned)));
            }

            let mut historys = Vec::with_capacity(num_threads);
            for handle in handles {
                historys.push(handle.join().unwrap());
            }

            check_history(historys, map);

            num_threads *= 2;
        }
    }
}
