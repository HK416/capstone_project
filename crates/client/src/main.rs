mod app;
mod error;
mod render;

use self::app::AppBuilder;

/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점 입니다.
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    // `winit` 이벤트 루프를 생성하고 애플리케이션을 실행합니다.
    AppBuilder::new().build_and_run();
}
