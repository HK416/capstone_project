mod app;
mod cmd;
mod error;
mod render;

use self::cmd::parse_command_line_args;

use std::thread;
use std::thread::ThreadId;
use lazy_static::lazy_static;

lazy_static! {
    /// `main` 스레드의 스레드 `ID` 입니다.
    pub static ref MAIN_THREAD_ID: ThreadId = std::thread::current().id();
}


/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점 입니다.
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    assert_eq!(thread::current().id(), *MAIN_THREAD_ID, "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    // `winit` 이벤트 루프를 생성하고 애플리케이션을 실행합니다.
    parse_command_line_args()
        .set_title("Hello to Halo!")
        .build_and_run();
}
