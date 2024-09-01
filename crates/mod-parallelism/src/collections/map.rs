use std::cmp;
use std::fmt;
use std::ptr;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::marker::PhantomData;
use std::sync::atomic;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering as MemOrdering;

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
    pub const fn new() -> Self {
        Self {
            inner: AtomicUsize::new(0), 
            _phantom: PhantomData, 
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
    const ARRAY_REPEAT_VAL: Stamp<Self> = Stamp::new();

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
            head: AtomicPtr::new(head), 
            tail: AtomicPtr::new(tail) 
        }
    }
}

impl<K: Ord, V> SkipMap<K, V> {
    /// Skip List에 주어진 키 값의 이전, 이후 노드를 검색합니다.
    /// 주어진 키 값이 포함되어 있는 경우 `true`를 반환합니다.
    unsafe fn search_position(&self, key: &Key<K>) -> (
        bool, 
        [*mut Node<K, V>; MAX_LEVELS], 
        [*mut Node<K, V>; MAX_LEVELS], 
    ) {
        let mut prevs = [ptr::null_mut(); MAX_LEVELS];
        let mut currs = [ptr::null_mut(); MAX_LEVELS];
        'retry: loop {
            let mut prev = self.head.load(MemOrdering::Relaxed);
            let mut curr = ptr::null_mut();
            
            for lv in (0..MAX_LEVELS).rev() {
                curr = (*prev).next[lv].get_ptr();

                loop {
                    let (mut succ, mut removed) = (*curr).next[lv].get_ptr_with_marking();
                    while removed {
                        // 이미 다른 스레드가 노드 제거를 수행한 경우 처음 부터 다시 시도.
                        if !(*prev).next[lv].try_change(curr, succ, false, false) {
                            continue 'retry;
                        }

                        curr = (*prev).next[lv].get_ptr();
                        (succ, removed) = (*curr).next[lv].get_ptr_with_marking();
                    }

                    if (*curr).key.lt(key) {
                        prev = curr;
                        curr = succ;
                    } else {
                        break;
                    }
                }

                prevs[lv] = prev;
                currs[lv] = curr;
            }

            return ((*curr).key.eq(key), prevs, currs);
        }
    }

    /// `SkipMap`에 데이터를 넣습니다.
    /// 주어진 키 값이 이미 존재할 경우 `false`를 반환합니다.
    pub fn insert(&self, key: K, val: V) -> bool {
        let new = Box::into_raw(Box::new(Node::new(key, val)));
        unsafe {
            loop {
                let (is_contains, mut prevs, mut currs) = self.search_position(&(*new).key);
                if is_contains {
                    drop(Box::from_raw(new));
                    return false;
                }

                // 다음 노드를 설정합니다.
                for lv in 0..=(*new).top_level {
                    (*new).next[lv].set_ptr(currs[lv]);
                }
                atomic::fence(MemOrdering::SeqCst);

                let mut prev = prevs[0];
                let mut curr = currs[0];
                if !(*prev).next[0].try_change(curr, new, false, false) {
                    continue;
                }

                for lv in 1..=(*new).top_level {
                    loop {
                        prev = prevs[lv];
                        curr = currs[lv];
                        if (*prev).next[lv].try_change(curr, new, false, false) {
                            break;
                        }
                        (_, prevs, currs) = self.search_position(&(*new).key);
                    }
                }

                return true;
            }
        }
    }

    pub fn remove(&self, key: K) -> Option<V> {
        let key = Key::Value(MaybeUninit::new(key));
        unsafe {
            loop {
                let (is_contains, _, currs) = self.search_position(&key);
                if !is_contains {
                    return None;
                }

                let value = (*currs[0]).value.assume_init_read();
                let mut value = ManuallyDrop::new(value);

                let target = currs[0];
                for lv in (1..=(*target).top_level).rev() {
                    let (mut succ, mut removed) = (*target).next[lv].get_ptr_with_marking();
                    #[allow(unused_must_use)]
                    while !removed {
                        (*target).next[lv].try_change(succ, succ, false, true);
                        (succ, removed) = (*target).next[lv].get_ptr_with_marking();
                    }
                }

                let mut removed;
                let mut succ = (*target).next[0].get_ptr();
                loop {
                    let marked = (*target).next[0].try_change(succ, succ, false, true);
                    (succ, removed) = (*currs[0]).next[0].get_ptr_with_marking();
                    if marked {
                        self.search_position(&key);
                        return Some(ManuallyDrop::take(&mut value));
                    } else if removed {
                        return None;
                    }
                }
            }
        }
    }

    pub fn contains(&self, key: K) -> bool {
        unsafe { self.search_position(&Key::Value(MaybeUninit::new(key))).0 }
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
    use std::thread;
    use std::sync::Arc;

    use super::SkipMap;

    const MAX_NUM: usize = 10_000;
    const MAX_THREADS: usize = 16;
    const NUM_TESTS: usize = 10_000_000;

    enum History {
        Insert {
            val: u32, 
            res: bool, 
        }, 
        Remove {
            val: u32, 
            res: bool, 
        }
    }



    fn thread_main(num_threads: usize, map: Arc<SkipMap<u32, u32>>) -> Vec<History> {
        let num_tests = NUM_TESTS / num_threads;
        (0..num_tests).into_iter()
            .map(|_| {
                let mut op: u32 = rand::random();
                op = op % 2;
                let mut val = rand::random();
                val = val % MAX_NUM as u32;
                match op {
                    0 => History::Insert { val, res: map.insert(val, val) }, 
                    1 => History::Remove { val, res: map.remove(val).is_some() },
                    _ => panic!("out of range!")
                }
            })
            .collect()
    }

    fn check_history(historys: Vec<Vec<History>>, map: Arc<SkipMap<u32, u32>>) {
        let mut survive = [0; MAX_NUM + 1];

        for historys in historys {
            for history in historys {
                match history {
                    History::Insert { val, res } if res => {
                        survive[val as usize] += 1;
                    }, 
                    History::Remove { val, res } if res => {
                        survive[val as usize] -= 1;
                    }, 
                    _ => { }
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

            let historys: Vec<_> = handles.into_iter()
                .map(|handle| handle.join().unwrap())
                .collect();
            check_history(historys, map);

            num_threads *= 2;
        }
    }
}
