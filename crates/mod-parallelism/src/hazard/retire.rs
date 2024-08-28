use std::ptr;
use std::ops::Deref;
use std::ops::DerefMut;
use std::collections::VecDeque;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering as MemOrdering;

use thread_local::ThreadLocal;



/// ### Retire List
/// 스레드간 공유되지 않고 해제해야 하는 메모리 블록을 저장하고 있습니다.
pub(crate) struct RetireList {
    inner: VecDeque<usize>
}

impl RetireList {
    /// 새로운 RetireList를 메모리에서 할당 받아 생성합니다.
    /// 
    /// # Warning 
    /// 할당 받은 메모리는 자동으로 회수되지 않습니다.
    /// 따라서 사용이 끝난 후 직접 메모리를 반환해야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new() -> *mut RetireList {
        Box::into_raw(Box::new(Self { 
            inner:  VecDeque::with_capacity(16)
        }))
    }
}

impl Deref for RetireList {
    type Target = VecDeque<usize>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for RetireList {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}



/// ### Node
/// Retire Collector의 Node입니다.
pub(crate) struct Node<T> {
    ptr: *mut RetireList, 
    next: AtomicPtr<Node<T>>
}

impl<T> Node<T> {
    /// 새로운 Node를 메모리에서 할당 받아 생성합니다.
    /// 
    /// # Warning 
    /// 할당 받은 메모리는 자동으로 회수되지 않습니다.
    /// 따라서 사용이 끝난 후 직접 메모리를 반환해야 합니다.
    /// 
    pub fn new(ptr: *mut RetireList) -> *mut Self {
        Box::into_raw(Box::new(Self {
            ptr, 
            next: AtomicPtr::new(ptr::null_mut())
        }))
    }

    /// 다음 Node의 주소 값을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_next(&self) -> *mut Self {
        self.next.load(MemOrdering::Relaxed)
    }

    /// 다음 Node의 주소 값을 설정합니다.
    #[inline]
    pub fn set_next(&self, ptr: *mut Self) {
        self.next.store(ptr, MemOrdering::Relaxed)
    }
}

impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        unsafe {
            let retire_list = self.ptr;
            while let Some(addr) = (*retire_list).pop_front() {
                drop(Box::from_raw(addr as *mut T));
            }
            drop(Box::from_raw(self.ptr));
        }
    }
}



/// ### Retire Collector
/// RetireList 객체의 포인터를 저장합니다.
pub(super) struct RetireCollector<T> {
    head: AtomicPtr<Node<T>>, 
    tls: ThreadLocal<AtomicPtr<RetireList>>, 
}

impl<T> RetireCollector<T> {
    /// 새로운 RetireCollector를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { 
            head: AtomicPtr::new(ptr::null_mut()), 
            tls: ThreadLocal::new()
        }
    }

    /// 현재 스레드의 RetireList를 가져옵니다.
    pub fn get_list(&self) -> *mut RetireList {
        self.tls.get_or(|| unsafe {
            let ptr = RetireList::new();

            // RetireCollector에 RetireList가 추가되지 않은 경우 (처음 호출된 경우)
            // RetireCollector에 RetireList를 추가합니다.
            let new = Node::new(ptr);
            loop {
                let current = self.get_head();
                (*new).set_next(current);
                if self.try_append(current, new) {
                    break;
                }
            }

            AtomicPtr::new(ptr)
        }).load(MemOrdering::Relaxed)
    }

    /// `CAS` 연산을 사용하여 RetireCollector에 새로운 RetireList 추가를 시도합니다.
    /// 
    /// 이미 다른 스레드가 먼저 RetireList를 추가하여 `CAS` 연산이 실패한 경우 `false`를 반환합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn try_append(&self, current: *mut Node<T>, new: *mut Node<T>) -> bool {
        self.head.compare_exchange(
            current, 
            new, 
            MemOrdering::SeqCst, 
            MemOrdering::Relaxed
        ).is_ok()
    }

    /// RetireCollector의 `head` Node를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_head(&self) -> *mut Node<T> {
        self.head.load(MemOrdering::Relaxed)
    }

    /// RetireCollector의 `head` Node를 설정합니다.
    #[inline]
    fn set_head(&self, ptr: *mut Node<T>) {
        self.head.store(ptr, MemOrdering::Relaxed)
    }

    /// Retire Collector에 들어있는 모든 Node를 제거합니다.
    pub fn clear(&self) {
        unsafe {
            let mut ptr = self.get_head();
            while !ptr.is_null() {
                let temp = ptr;
                ptr = (*ptr).get_next();
                drop(Box::from_raw(temp));
            }
            self.set_head(ptr::null_mut());
        }
    }
}

impl<T> Drop for RetireCollector<T> {
    #[inline]
    fn drop(&mut self) {
        self.clear()
    }
}
