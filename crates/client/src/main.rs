
use std::thread;
use client_framework::app::builder::AppBuilder;
use client_framework::scene::GameScene;
use framework::MAIN_THREAD_ID;



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

    AppBuilder::new(Box::new(TestScene {}))
        .set_title("Hello to Halo!")
        .build_and_run()
}


#[derive(Debug)]
pub struct TestScene { }

impl GameScene for TestScene { }
