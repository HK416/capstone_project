use std::fmt;
use std::thread;

use mod_app::AppBuilder;
use mod_parallelism::MAIN_THREAD_ID;
use mod_scene::GameScene;



/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점입니다.
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
        .with_title("Mollu")
        .build_and_run()
}


pub struct TestScene { }

impl GameScene for TestScene {
    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        window: &winit::window::Window, 
        surface: &wgpu::Surface, 
        world: &hecs::World, 
        app: &dyn mod_util::AppHandle
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl fmt::Debug for TestScene {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestScene))
    }
}
