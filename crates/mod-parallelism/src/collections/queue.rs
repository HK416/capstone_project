use std::{
    mem::{ManuallyDrop, MaybeUninit},
    ptr::null_mut,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering as MemOrdering},
};

use crate::{backoff::Backoff, epoch::Collector};

/// ## Node
/// - T: 대상 자료형
///
/// Lock-Free Queue에 사용되는 노드입니다.
///
#[derive(Debug)]
struct Node<T> {
    value: MaybeUninit<T>,
    next: AtomicPtr<Node<T>>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            next: AtomicPtr::new(null_mut()),
        }
    }
}

/// ## Queue
/// - T: 대상 자료형
/// - M: 회수 메모리 저장 용량
///
/// Lock-Free Queue 자료구조입니다.
///
#[derive(Debug)]
pub struct Queue<T, const M: usize = 32> {
    collector: Box<Collector<Node<T>, M>>,
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
    len: AtomicUsize,
}

impl<T, const M: usize> Queue<T, M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Queue`가 비어있는 경우 `true`를 반환합니다.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `Queue`에 포함된 요소의 개수를 반환합니다.
    pub fn len(&self) -> usize {
        self.len.load(MemOrdering::Acquire)
    }

    /// `Queue`에 요소를 추가합니다.
    pub fn push(&self, val: T) {
        let mut backoff = Backoff::new();
        let guard = self.collector.scope();
        let new = Box::into_raw(guard.alloc(move || Node {
            value: MaybeUninit::new(val),
            next: AtomicPtr::new(null_mut()),
        }));

        loop {
            let last = self.tail.load(MemOrdering::Relaxed);
            let next = unsafe { (*last).next.load(MemOrdering::Relaxed) };

            // `last`와 `tail`이 같지 않은 경우 처음 부터 다시 시작합니다.
            if last != self.tail.load(MemOrdering::Relaxed) {
                continue;
            }

            // `next`가 null이 아닌 경우 처음 부터 다시 시작합니다.
            if !next.is_null() {
                let _ = self.tail.compare_exchange(
                    last,
                    next,
                    MemOrdering::SeqCst,
                    MemOrdering::Relaxed,
                );
                continue;
            }

            // `last`의 `next`의 값 변경을 시도하고 성공할 경우 함수를 빠져나옵니다.
            let success = unsafe {
                (*last)
                    .next
                    .compare_exchange(null_mut(), new, MemOrdering::SeqCst, MemOrdering::Relaxed)
                    .is_ok()
            };
            if success {
                let _ = self.tail.compare_exchange(
                    next,
                    new,
                    MemOrdering::SeqCst,
                    MemOrdering::Relaxed,
                );
                self.len.fetch_add(1, MemOrdering::AcqRel);
                return;
            }

            // 스레드를 지정된 값 만큼 대기시킵니다.
            backoff.wait();
        }
    }

    /// `Queue`에서 요소 하나를 꺼내옵니다.
    pub fn pop(&self) -> Option<T> {
        let mut backoff = Backoff::new();
        let guard = self.collector.scope();
        loop {
            let first = self.head.load(MemOrdering::Relaxed);
            let last = self.tail.load(MemOrdering::Relaxed);
            let next = unsafe { (*first).next.load(MemOrdering::Relaxed) };

            // `first`와 `head`가 다른 경우 처음 부터 다시 시작합니다.
            if first != self.head.load(MemOrdering::Relaxed) {
                continue;
            }

            // `next`가 null인 경우 `None`을 반환합니다.
            if next.is_null() {
                return None;
            }

            // `first`와 `last`가 같은 경우 `tail` 변경을 시도하고 처음 부터 다시 시작합니다.
            if first == last {
                let _ = self.tail.compare_exchange(
                    last,
                    next,
                    MemOrdering::SeqCst,
                    MemOrdering::Relaxed,
                );
                continue;
            }

            let value = unsafe { (*next).value.assume_init_read() };
            let mut value = ManuallyDrop::new(value);

            // `head` 변경을 시도하고 성공할 경우 값을 반환합니다.
            let success = self
                .head
                .compare_exchange(first, next, MemOrdering::SeqCst, MemOrdering::Relaxed)
                .is_ok();
            if !success {
                backoff.wait();
                continue;
            }

            self.len.fetch_sub(1, MemOrdering::AcqRel);

            guard.dealloc(unsafe { Box::from_raw(first) });
            let value = unsafe { ManuallyDrop::take(&mut value) };
            return Some(value);
        }
    }
}

impl<T, const M: usize> Default for Queue<T, M> {
    fn default() -> Self {
        let ptr = Box::into_raw(Box::new(Node::default()));
        Self {
            collector: Box::new(Collector::new()),
            head: AtomicPtr::new(ptr),
            tail: AtomicPtr::new(ptr),
            len: AtomicUsize::new(0),
        }
    }
}

impl<T, const M: usize> Drop for Queue<T, M> {
    fn drop(&mut self) {
        let mut curr = self.head.load(MemOrdering::Relaxed);
        while !curr.is_null() {
            let temp = curr;
            curr = unsafe { (*curr).next.load(MemOrdering::Relaxed) };
            unsafe { drop(Box::from_raw(temp)) };
        }
    }
}
