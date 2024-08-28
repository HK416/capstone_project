use std::ptr;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering as MemOrdering;



/// ### Hazard Pointer
/// 메모리를 해제할 경우 위험한 객체의 포인터를 저장합니다.
pub(crate) struct Hazard<T> {
    /// 메모리를 해제할 경우 위함한 객체의 주소 값입니다.
    node: AtomicPtr<T>, 

    /// 다음 Hazard Pointer의 주소 값입니다.
    next: AtomicPtr<Hazard<T>>, 

    /// Hazard Pointer의 재사용 가능 여부를 나타냅니다.
    /// 어떤 스레드가 사용 중인 경우 이 값은 `true`입니다.
    is_activate: AtomicBool, 
}

impl<T> Hazard<T> {
    /// 새로운 Hazard Pointer를 메모리에서 할당 받아 생성합니다.
    /// 
    /// # Warning 
    /// 할당 받은 메모리는 자동으로 회수되지 않습니다.
    /// 따라서 사용이 끝난 후 직접 메모리를 반환해야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new() -> *mut Self {
        Box::into_raw(Box::new(Self { 
            node: AtomicPtr::new(ptr::null_mut()), 
            next: AtomicPtr::new(ptr::null_mut()), 
            is_activate: AtomicBool::new(true) 
        }))
    }

    /// `CAS` 연산을 사용하여 Hazard Pointer를 활성 상태로 변경을 시도합니다.
    /// 
    /// 이미 다른 스레드가 활성화를 완료하여 `CAS` 연산이 실패한 경우 `false`를 반환합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn try_activate(&self) -> bool {
        self.is_activate.compare_exchange(
            false, 
            true, 
            MemOrdering::SeqCst, 
            MemOrdering::Relaxed
        ).is_ok()
    }

    /// Hazard Pointer가 활성화 되었는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_activate(&self) -> bool {
        self.is_activate.load(MemOrdering::Relaxed)
    }

    /// 다음 Hazard Pointer의 주소값을 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_next(&self) -> *mut Self {
        self.next.load(MemOrdering::Relaxed)
    }

    /// 다음 Hazard Pointer의 주소값을 설정합니다.
    #[inline]
    pub fn set_next(&self, ptr: *mut Self) {
        self.next.store(ptr, MemOrdering::Relaxed)
    }

    /// Hazard pointer의 메모리 블록 주소 값을 가져옵니다.
    #[inline]
    #[must_use]
    pub(super) fn get_node(&self) -> *mut T {
        self.node.load(MemOrdering::Relaxed)
    }

    /// Hazard Pointer의 메모리 블록을 주소 값을 설정합니다.
    #[inline]
    pub fn set_node(&self, ptr: *mut T) {
        self.node.store(ptr, MemOrdering::Relaxed)
    }

    /// Hazard Pointer에 등록된 메모리 블록을 해제합니다.
    pub fn release(&self) {
        self.node.store(ptr::null_mut(), MemOrdering::Relaxed);
        atomic::fence(MemOrdering::SeqCst);
        self.is_activate.store(false, MemOrdering::Relaxed);
    }
}



/// ### Hazard Collector
/// 스레드간 공유되는 Hazard Pointer를 모아놓는 Linked List 자료구조입니다.
/// 
/// Lock-Free Linked List로 구현되어 있으며, Hazard Pointer를 할당만 합니다.
/// 
/// 한 스레드가 안전하게 액세스하고자 하는 공유 메모리 블록이 있는 경우 Hazard Pointer List에 등록하여 사용하고, 
/// 이후 사용이 끝나고 Hazard Pointer List에서 제거합니다.
/// 
/// 각 스레드는 해제해야할 공유 메모리 블록이 존재할 경우 우선 Hazard Pointer List에 등록되어 있는지 확인해야 합니다.
/// 만약 Hazard Pointer List에 발견되었을 경우 해당 스레드의 Retire List에 저장후 추후에 삭제를 시도합니다.
/// 
pub(super) struct HazardCollector<T> {
    head: AtomicPtr<Hazard<T>>, 
}

impl<T> HazardCollector<T> {
    /// 새로운 HazardCollector를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { head: AtomicPtr::new(ptr::null_mut()) }
    }

    /// Hazard Pointer를 할당합니다.
    /// 
    /// 기존의 Hazard Pointer를 재활용할 수 있는 경우 기존의 Hazard Pointer를 재활용합니다.
    /// 
    /// 만약 모든 Hazard Pointer가 사용중일 경우 새로운 Hazard Pointer를 메모리에서 할당받습니다.
    /// 
    pub fn alloc(&self) -> *mut Hazard<T> {
        unsafe {
            // Retire한 Hazard Pointer 재사용을 시도합니다.
            let mut ptr = self.get_head();
            while !ptr.is_null() {
                if !(*ptr).is_activate() && (*ptr).try_activate() {
                    return ptr;
                }
                ptr = (*ptr).get_next();
            }

            // 재사용 가능한 Hazard Pointer가 없는 경우
            // 새로운 Hazard Pointer를 생성한 후 추가합니다.
            let new_ptr = Hazard::new();
            loop {
                let current_ptr = self.get_head();
                (*new_ptr).set_next(current_ptr);
                if self.try_append(current_ptr, new_ptr) {
                    return new_ptr;
                }
            }
        }
    }

    /// `CAS` 연산을 사용하여 HazardCollector에 새로운 Hazard Pointer 추가를 시도합니다.
    /// 
    /// 이미 다른 스레드가 먼저 Hazard Pointer를 추가하여 `CAS` 연산이 실패한 경우 `false`를 반환합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn try_append(&self, current: *mut Hazard<T>, new: *mut Hazard<T>) -> bool {
        self.head.compare_exchange(
            current, 
            new, 
            MemOrdering::SeqCst, 
            MemOrdering::Relaxed
        ).is_ok()
    }

    /// HazardCollector의 `head` Hazard Pointer를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_head(&self) -> *mut Hazard<T> {
        self.head.load(MemOrdering::Relaxed)
    }

    /// HazardCollector의 `head` Hazard Pointer를 설정합니다.
    #[inline]
    pub fn set_head(&self, ptr: *mut Hazard<T>) {
        self.head.store(ptr, MemOrdering::Relaxed)
    }

    /// Hazard Collector에 들어있는 모든 Node를 제거합니다.
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

impl<T> Drop for HazardCollector<T> {
    #[inline]
    fn drop(&mut self) {
        self.clear()
    }
}
