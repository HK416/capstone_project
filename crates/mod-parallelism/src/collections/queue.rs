use std::fmt;
use std::ptr;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering as MemOrdering;



/// 무잠금 Queue에서 사용하는 노드입니다.
struct Node<T> {
    value: T, 
    next: AtomicPtr<Node<T>>, 
}

impl<T> Node<T> {
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
