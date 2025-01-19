use std::{
    ptr::null_mut,
    sync::atomic::{AtomicPtr, Ordering as MemOrdering},
};

use crate::{backoff::Backoff, epoch::Collector};

/// Lock-Free Stack에서 사용하는 노드입니다.
#[derive(Debug)]
struct Node<T> {
    value: Option<T>,
    next: AtomicPtr<Node<T>>,
}

/// Lock-Free Stack 자료구조입니다.
#[derive(Debug)]
pub struct Stack<T> {
    collector: Box<Collector<Node<T>>>,
    top: AtomicPtr<Node<T>>,
}

impl<T> Stack<T> {
    /// 새로운 Lock-Free Stack을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }

    /// 주어진 `val`을 추가합니다.
    pub fn push(&self, val: T) {
        let mut backoff = Backoff::new();
        let scope = self.collector.scope();
        let new = Box::into_raw(scope.alloc(move || Node {
            value: Some(val),
            next: AtomicPtr::new(null_mut()),
        }));

        loop {
            let current = self.top.load(MemOrdering::Relaxed);
            unsafe { (*new).next.store(current, MemOrdering::Relaxed) };

            if current != self.top.load(MemOrdering::Relaxed) {
                continue;
            }

            let success = self
                .top
                .compare_exchange(current, new, MemOrdering::SeqCst, MemOrdering::Relaxed)
                .is_ok();
            if success {
                break;
            }

            backoff.wait();
        }
    }

    /// Stack의 가장 최근에 추가된 값을 반환합니다.
    /// Stack이 비어있는 경우 `None`을 반환합니다.
    pub fn pop(&self) -> Option<T> {
        let mut backoff = Backoff::new();
        let scope = self.collector.scope();
        loop {
            let current = self.top.load(MemOrdering::Relaxed);
            if current.is_null() {
                return None;
            }

            let next = unsafe { (*current).next.load(MemOrdering::Relaxed) };
            if current != self.top.load(MemOrdering::Relaxed) {
                continue;
            }

            let success = self
                .top
                .compare_exchange(current, next, MemOrdering::SeqCst, MemOrdering::Relaxed)
                .is_ok();
            if !success {
                backoff.wait();
                continue;
            }

            scope.dealloc(unsafe { Box::from_raw(current) });
            return Some(unsafe { (*current).value.take().unwrap_unchecked() });
        }
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self {
            collector: Box::new(Collector::new()),
            top: AtomicPtr::new(null_mut()),
        }
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        let mut ptr = self.top.load(MemOrdering::Relaxed);
        while !ptr.is_null() {
            let temp = ptr;
            ptr = unsafe { (*ptr).next.load(MemOrdering::Relaxed) };
            unsafe { drop(Box::from_raw(temp)) };
        }
    }
}
