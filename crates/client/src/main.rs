mod core;
mod error;
mod render;

use crate::core::app::builder::AppBuilder;

use std::thread;
use std::thread::ThreadId;
use lazy_static::lazy_static;


lazy_static! {
    /// `main` 스레드의 스레드 `ID` 입니다.
    pub static ref MAIN_THREAD_ID: ThreadId = std::thread::current().id();
}



/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점 입니다.
/// 
/// 게임 화면은 16 : 9 비율의 scaled 크기를 가집니다.
/// 
/// `Windows`, `macOS` 플랫폼의 경우 최초 실행시 전체 화면으로 실행됩니다.
/// 
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    assert_eq!(thread::current().id(), *MAIN_THREAD_ID, "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new()
        .set_title("Hello to Halo!")
        .build_and_run()
}
