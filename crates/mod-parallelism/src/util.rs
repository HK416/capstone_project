use std::thread; 
use std::thread::ThreadId;

use lazy_static::lazy_static;

lazy_static! {
    /// 메인 스레드의 스레드 식별자 입니다.
    pub static ref MAIN_THREAD_ID: ThreadId = thread::current().id();

    /// 현재 시스템의 물리적 코어 갯수입니다.
    pub static ref NUM_SYSTEM_CORE: usize = num_cpus::get_physical();
}



/// 현재 스레드가 메인 스레드인 경우 `true`를 반환합니다.
#[inline]
#[must_use]
pub fn is_main_thread() -> bool {
    thread::current().id() == *MAIN_THREAD_ID
}
