use std::ffi::c_void;


unsafe extern "C" {
    pub fn ebr_new(max_threads: i32) -> *mut c_void;

    fn set_new() -> *mut c_void;
    fn set_delete(set: *mut c_void);
    
    fn set_clear(set: *mut c_void);
    
    fn set_add(set: *mut c_void, key: i32);
    fn set_remove(set: *mut c_void, key: i32);
    fn set_contains(set: *mut c_void, key: i32) -> bool;

    pub fn set_thread_local_id(id: i32);
}


pub struct Set {
    data: *mut c_void,
}

impl Set {
    pub fn new() -> Self {
        Set {
            data: unsafe { set_new() },
        }
    }

    pub fn clear(&self) {
        unsafe {
            set_clear(self.data);
        }
    }

    pub fn add(&self, key: i32) {
        unsafe {
            set_add(self.data, key);
        }
    }

    pub fn remove(&self, key: i32) {
        unsafe {
            set_remove(self.data, key);
        }
    }

    pub fn contains(&self, key: i32) -> bool {
        unsafe { 
            set_contains(self.data, key)
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

unsafe impl Send for Set {}

unsafe impl Sync for Set {}
