use std::ptr;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering as MemOrdering;

use crate::epoch::EBRGuard;
use crate::epoch::EBR;





/// Lock-Free Stack에서 사용하는 노드입니다.
struct Node<T> {
    value: MaybeUninit<T>, 
    next: AtomicPtr<Node<T>>, 
}

impl<T> Node<T> {
    /// 새로운 노드를 생성합니다.
    #[must_use]
    fn new(ebr_pin: &EBRGuard<'_, Self>, val: T) -> *mut Self {
        let mut node = ebr_pin.alloc();
        node.value.write(val);
        node.next.store(ptr::null_mut(), MemOrdering::Relaxed);
        return Box::into_raw(node);
    }

    /// 현재 노드가 가리키는 다음 노드를 가져옵니다.
    #[inline]
    #[must_use]
    fn get_next(&self) -> *mut Self {
        self.next.load(MemOrdering::Relaxed)
    }
}

impl<T> Default for Node<T> {
    #[inline]
    fn default() -> Self {
        Self { 
            value: MaybeUninit::uninit(), 
            next: AtomicPtr::new(ptr::null_mut())
        }
    }
}





/// Lock-Free Stack 자료구조입니다.
#[derive(Debug)]
pub struct Stack<T> {
    ebr: EBR<Node<T>>, 
    top: AtomicPtr<Node<T>>, 
}

impl<T> Stack<T> {
    /// 새로운 Lock-Free Stack을 생성합니다.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { 
            ebr: EBR::new(), 
            top: AtomicPtr::new(ptr::null_mut()) 
        }
    }

    /// 주어진 `val`을 추가합니다.
    pub fn push(&self, val: T) {
        let ebr_pin = self.ebr.pin();
        let new = Node::new(&ebr_pin, val);
        unsafe {
            loop {
                let current = self.top.load(MemOrdering::Relaxed);
                (*new).next.store(current, MemOrdering::Relaxed);
                if current != self.top.load(MemOrdering::Relaxed) { 
                    continue;
                }
                if self.top.compare_exchange(current, new, MemOrdering::SeqCst, MemOrdering::Relaxed).is_ok() {
                    break;
                }
            }
        }
    }

    /// Stack의 가장 최근에 추가된 값을 반환합니다.
    /// Stack이 비어있는 경우 `None`을 반환합니다.
    pub fn pop(&self) -> Option<T> {
        let ebr_pin = self.ebr.pin();
        unsafe {
            loop {
                let current = self.top.load(MemOrdering::Relaxed);
                if current.is_null() {
                    return None;
                }

                let next = (*current).get_next();
                let value = (*current).value.assume_init_read();
                let mut value = ManuallyDrop::new(value);
                
                if current != self.top.load(MemOrdering::Relaxed) {
                    continue;
                }
                if !self.top.compare_exchange(current, next, MemOrdering::SeqCst, MemOrdering::Relaxed).is_ok() {
                    continue;
                }

                ebr_pin.dealloc(Box::from_raw(current));
                return Some(ManuallyDrop::take(&mut value));
            }
        }
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        unsafe {
            let mut ptr = self.top.load(MemOrdering::Relaxed);
            while !ptr.is_null() {
                let temp = ptr;
                ptr = (*ptr).get_next();
                drop(Box::from_raw(temp));
            }
        }
    }
}
