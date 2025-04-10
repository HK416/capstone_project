use std::ffi::c_void;


unsafe extern "C" {
    pub fn ebr_new(max_threads: i32) -> *mut c_void;

    fn set_new(max_threads: i32) -> *mut c_void;
    fn set_delete(set: *mut c_void);
    
    fn set_clear(set: *mut c_void);
    fn set_reset_accessor_counter(set: *mut c_void);
    /// thread-safe하지 않은 contains
    fn set_contains(set: *mut c_void, key: i32) -> bool;
    fn set_new_accessor(set: *mut c_void) -> *mut c_void;

    fn set_accessor_add(accessor: *mut c_void, key: i32) -> bool;
    fn set_accessor_remove(accessor: *mut c_void, key: i32) -> bool;
    /// thread-safe한 contains
    fn set_accessor_contains(accessor: *mut c_void, key: i32) -> bool;
    fn set_accessor_delete(accessor: *mut c_void);
}


pub struct Set {
    data: *mut c_void,
}

impl Set {
    pub fn new(max_threads: i32) -> Self {
        Set {
            data: unsafe { set_new(max_threads) },
        }
    }

    pub fn clear(&self) {
        unsafe {
            set_clear(self.data);
        }
    }

    pub fn reset_accessor_counter(&self) {
        unsafe {
            set_reset_accessor_counter(self.data);
        }
    }

    pub fn contains(&self, key: i32) -> bool {
        unsafe { 
            set_contains(self.data, key)
        }
    }

    pub fn new_accessor(&self) -> SetAccessor {
        let accessor = unsafe { set_new_accessor(self.data) };
        if accessor.is_null() {
            panic!("Accessor의 개수는 Set생성시 설정한 max_threads 보다 많을 수 없습니다.");
        }
        
        SetAccessor {
            data: accessor,
        }
    }
}

impl Drop for Set {
    fn drop(&mut self) {
        unsafe {
            set_delete(self.data);
        }
    }
}


pub struct SetAccessor {
    data: *mut c_void,
}

impl SetAccessor {
    pub fn add(&self, key: i32) -> bool {
        unsafe { set_accessor_add(self.data, key) }
    }

    pub fn remove(&self, key: i32) -> bool {
        unsafe { set_accessor_remove(self.data, key) }
    }

    pub fn contains(&self, key: i32) -> bool {
        unsafe { set_accessor_contains(self.data, key) }
    }
}

impl Drop for SetAccessor {
    fn drop(&mut self) {
        unsafe {
            set_accessor_delete(self.data);
        }
    }
}

unsafe impl Send for SetAccessor {}

unsafe impl Sync for SetAccessor {}
