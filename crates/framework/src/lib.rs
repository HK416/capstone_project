pub mod timer;

use std::thread::ThreadId;
use lazy_static::lazy_static;

lazy_static! {
    /// `main` 스레드의 스레드 `ID` 입니다.
    pub static ref MAIN_THREAD_ID: ThreadId = std::thread::current().id();
}
