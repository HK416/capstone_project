use std::fmt;
use std::mem;
use std::ptr;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering as MemOrdering;



/// 무잠금 Queue에서 사용하는 노드입니다.
struct Node<T> {
    value: T, 
    next: AtomicPtr<Node<T>>, 
}

impl<T> Node<T> {
    /// 비어있는 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn zeroed() -> Self {
        unsafe {
            Self { 
                value: mem::zeroed(), 
                next: AtomicPtr::new(ptr::null_mut()) 
            }
        }
    }

    /// 새로운 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value, 
            next: AtomicPtr::new(ptr::null_mut()), 
        }
    }

    /// 새로운 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub const fn new_with_next(value: T, next: *mut Node<T>) -> Self {
        Self { 
            value, 
            next: AtomicPtr::new(next) 
        }
    }

    #[inline]
    #[must_use]
    pub fn get_next(&self) -> *mut Node<T> {
        self.next.load(MemOrdering::Relaxed)
    }
}

impl<T: fmt::Debug> fmt::Debug for Node<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(Node<T>))
            .field("value", &self.value)
            .field("next", &self.next.load(MemOrdering::Relaxed))
            .finish()
    }
}



/// Lock-Free Queue 자료구조입니다.
pub struct Queue<T> {
    head: AtomicPtr<Node<T>>, 
    tail: AtomicPtr<Node<T>>, 
}

impl<T> Queue<T> {
    /// 새로운 Lock-Free Queue를 생성합니다.
    #[must_use]
    pub fn new() -> Self {
        let ptr = Box::into_raw(Box::new(Node::zeroed()));
        Self { 
            head: AtomicPtr::new(ptr), 
            tail: AtomicPtr::new(ptr) 
        }
    }

    pub fn push(&self, v: T) {
        let new = Box::into_raw(Box::new(Node::new(v)));
        unsafe {
            loop {
                let last = self.tail.load(MemOrdering::Relaxed);
                let next = (*last).get_next();

                // last가 tail이 아닌 경우 (다른 스레드가 tail을 변경한 경우) 다시 시도
                if last != self.tail.load(MemOrdering::Relaxed) {
                    continue;
                }

                // next가 null이 아닌 경우 (다른 스레드가 tail을 변경한 경우)
                // tail next로 옮기는 시도를 하고 다시 시도
                #[allow(unused_must_use)]
                if !next.is_null() {
                    self.tail.compare_exchange(last, next, MemOrdering::SeqCst, MemOrdering::Relaxed);
                    continue;
                }

                // last의 next의 값을 변경 시도 (다른 스레드가 tail을 변경하지 않아 last가 tail일 경우 성공)
                // 성공시 tail을 new로 옮기는 시도를 하고 종료
                #[allow(unused_must_use)]
                if (*last).next.compare_exchange(ptr::null_mut(), new, MemOrdering::SeqCst, MemOrdering::Relaxed).is_ok() {
                    self.tail.compare_exchange(next, new, MemOrdering::SeqCst, MemOrdering::Relaxed);
                    return;
                }
            }
        }
    }
} 

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            let mut ptr = (*self.head.load(MemOrdering::Relaxed)).get_next();
            while !ptr.is_null() {
                let temp = ptr;
                ptr = (*ptr).get_next();
                drop(Box::from_raw(temp)); // delete
            }
            drop(Box::from_raw(self.head.load(MemOrdering::Relaxed))); // delete
        }
    }
}




#[cfg(test)]
mod tests {
    use std::thread;
    use std::sync::Arc;

    use super::Queue;

    const MAX_THREADS: usize = 16;
    const MAX_TESTS: usize = 10_000_000;


    fn thread_main(num_threads: usize, queue: Arc<Queue<i32>>) {
        for _ in 0..MAX_TESTS / num_threads {
            let num = rand::random();
            queue.push(num);
        }
    }

    #[test]
    fn check_consistency() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            let queue = Arc::new(Queue::new());
            let handles: Vec<_> = (0..num_threads).into_iter()
                .map(|_| {
                    let queue_cloned = queue.clone();
                    thread::spawn(move || thread_main(num_threads, queue_cloned))
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
            
            drop(queue);
            num_threads *= 2;
        }
    }
}
