use std::thread::ThreadId;
use lazy_static::lazy_static;

lazy_static! {
    /// `main` 스레드의 스레드 `ID` 입니다.
    pub static ref MAIN_THREAD_ID: ThreadId = std::thread::current().id();

    /// 현재 시스템의 최대 코어의 갯수 입니다.
    pub static ref MAX_CORE_NUM: usize = num_cpus::get_physical();
}
