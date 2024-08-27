use std::fmt;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering as MemOrdering;



/// 무잠금 Queue에서 사용하는 노드입니다.
struct Node<T> {
    value: NonNull<T>, 
    next: AtomicPtr<Node<T>>, 
}

impl<T> Node<T> {
    /// 비어있는 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub fn zeroed() -> Self {
        Self { 
            value: NonNull::dangling(),  
            next: AtomicPtr::new(ptr::null_mut()) 
        }
    }

    /// 새로운 노드를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            value: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(value))) }, 
            next: AtomicPtr::new(ptr::null_mut()), 
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

    pub fn pop(&self) -> Option<T> {
        unsafe {
            loop {
                let first = self.head.load(MemOrdering::Relaxed);
                let last = self.tail.load(MemOrdering::Relaxed);
                let next = (*first).get_next();

                // first가 head가 아닌 경우 (다른 스레드가 head를 변경한 경우) 다시 시도
                if first != self.head.load(MemOrdering::Relaxed) {
                    continue;
                }

                // next가 null인 경우 (Queue가 비어있는 경우) None을 반환
                if next.is_null() {
                    return None;
                }

                // first와 last가 같은 경우 (tail이 마지막 노드를 가리키고 있지 않는 경우)
                // tail을 갱신하고 다시 시도
                #[allow(unused_must_use)]
                if first == last {
                    self.tail.compare_exchange(last, next, MemOrdering::SeqCst, MemOrdering::Relaxed);
                    continue;
                }

                // head를 next로 옮기기를 시도 (다른 스레드가 head를 변경하지 않아 head가 first인 경우 성공)
                // 실패할 경우 다시 시도.
                let value = (*next).value;
                if !self.head.compare_exchange(first, next, MemOrdering::SeqCst, MemOrdering::Relaxed).is_ok() {
                    continue;
                }

                //---- delete first -----
                return Some(*Box::from_raw(value.as_ptr()));
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
    //! 유효성 검사 방식
    //! 1. Lock-Free Queue에 push(enqueue), pop(dequeue) 할 때 마다 기록을 남깁니다.
    //! 2. Queue에 집어넣은 값과 Queue에서 꺼낸 값 + Queue에 남아있는 값이 같은지 확인합니다.
    //! 
    //! 이때 두 값이 맞지 않는 경우 Lock-Free Queue 구현에 문제가 있다 판단할 수 있습니다.
    //! 

    use std::thread;
    use std::sync::Arc;

    use super::Queue;

    const MAX_NUM: usize = 10_000;
    const MAX_THREADS: usize = 16;
    const MAX_TESTS: usize = 10_000_000;

    enum History {
        Push(u32), 
        Pop(Option<u32>)
    }

    fn thread_main(num_threads: usize, queue: Arc<Queue<u32>>) -> Vec<History> {
        let num_tests = MAX_TESTS / num_threads;
        let mut history = Vec::with_capacity(num_tests);

        for _ in 0..num_tests {
            if rand::random() {
                let mut val = rand::random();
                val = val % (MAX_NUM as u32 + 1);
                queue.push(val);
                history.push(History::Push(val));
            } else {
                history.push(History::Pop(queue.pop()));
            }
        }

        return history;
    }

    fn check_invalidation(historys: Vec<Vec<History>>, queue: Arc<Queue<u32>>) {
        let mut numbers: [i32; MAX_NUM + 1] = [0; MAX_NUM + 1];
        for history in historys {
            for record in history {
                match record {
                    History::Push(val) => {
                        numbers[val as usize] += 1
                    }, 
                    History::Pop(result) => {
                        if let Some(val) = result {
                            numbers[val as usize] -= 1;
                        }
                    }
                }
            }
        }

        while let Some(val) = queue.pop() {
            numbers[val as usize] -= 1;
        }

        for (number, count) in numbers.into_iter().enumerate() {
            if count == 0 {
                continue;
            } else if count < 0 {
                panic!("pop(dequeue) function is invalid! ({})", number);
            } else {
                panic!("push(enqueue) function is invalid! ({})", number);
            }
        }
    }

    #[test]
    fn check_consistency() {
        let mut num_threads = 1;
        while num_threads <= MAX_THREADS {
            println!("Checking validation... (Threads={})", num_threads);

            let queue = Arc::new(Queue::new());
            let handles: Vec<_> = (0..num_threads).into_iter()
                .map(|_| {
                    let queue_cloned = queue.clone();
                    thread::spawn(move || thread_main(num_threads, queue_cloned))
                })
                .collect();

            let mut historys = Vec::with_capacity(num_threads);
            for handle in handles {
                historys.push(handle.join().unwrap());
            }
            
            check_invalidation(historys, queue);
            num_threads *= 2;
        }
    }
}
