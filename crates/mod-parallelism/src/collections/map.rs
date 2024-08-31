use std::marker::PhantomData;
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



/// 주소 값과 Marking이 합쳐진 합성 포인터입니다.
struct Stamp<T> {
    inner: AtomicUsize, 
    _phantom: PhantomData<T>
}

impl<T> Stamp<T> {
    /// Marking되지 않은 새로운 합성 포인터를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(ptr: *mut T) -> Self {
        Self { 
            inner: AtomicUsize::new(ptr as usize), 
            _phantom: PhantomData 
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
