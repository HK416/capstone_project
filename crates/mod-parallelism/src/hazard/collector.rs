use std::collections::HashSet;
use std::collections::VecDeque;

use super::{Hazard, HazardCollector, RetireCollector, RetireList};


pub(crate) struct Collector<T> {
    hazard: HazardCollector<T>, 
    retire: RetireCollector<T>, 
}

impl<T> Collector<T> {
    pub const SCAN_POINT: usize = 10;

    /// 새로운 Collector를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { 
            hazard: HazardCollector::new(), 
            retire: RetireCollector::new() 
        }
    }

    /// Hazard Pointer를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_hazard_ptr(&self) -> *mut Hazard<T> {
        self.hazard.alloc()
    }

    #[inline]
    pub fn drop(&self, ptr: *mut T) {
        unsafe {
            // 현재 스레드 Retire List에 추가합니다.
            let retire_list = self.retire.get_list();
            (*retire_list).push_back(ptr as usize);

            // Retire List가 일정 갯수 이상일 경우 메모리 회수를 시작합니다.
            if (*retire_list).len() >= Self::SCAN_POINT {
                self.scan(retire_list);
            }
        }
    }

    fn scan(&self, retire_list: *mut RetireList) {
        unsafe {
            // Hazard Pointer에 등록된 메모리 블록 주소 값을 수집합니다.
            let mut hazard_set = HashSet::new();
            let mut ptr = self.hazard.get_head();
            while !ptr.is_null() {
                let node = (*ptr).get_node();
                if !node.is_null() {
                    hazard_set.insert(node as usize);
                }
                ptr = (*ptr).get_next();
            }

            let mut next = VecDeque::with_capacity(16);
            while let Some(addr) = (*retire_list).pop_front() {
                if !hazard_set.contains(&addr) {
                    drop(Box::from_raw(addr as *mut T));
                } else {
                    next.push_back(addr);
                }
            }
            (*retire_list).append(&mut next);
        }
    }
}
