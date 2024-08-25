use std::path::Path;
use std::sync::Arc;

use mod_util::AppFlags;
use mod_util::AppLocale; 
use mod_util::GameTimer;
use winit::window::Window;

use crate::GameSceneFlow;



/// 애플리케이션 접근 인터페이스입니다.
pub trait AppHandle {
    /// 애플리케이션에서 사용 가능한 최대 스레드 갯수를 가져옵니다.
    fn num_threads(&self) -> usize;

    /// 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    fn current_dir(&self) -> &Path;

    /// 애플리케이션 생성 플래그 옵션을 가져옵니다.
    fn flags(&self) -> AppFlags;

    /// 애플리케이션 표시 언어를 가져옵니다.
    fn locale(&self) -> Option<AppLocale>;

    /// 애플리케이션 타이머를 가져옵니다.
    fn timer(&self) -> &GameTimer;

    /// `wgpu` 렌더링 인스턴스를 가져옵니다.
    fn render_instance(&self) -> &Arc<wgpu::Instance>;

    /// `wgpu` 렌더링 장치 어댑터를 가져옵니다.
    fn render_adapter(&self) -> &Arc<wgpu::Adapter>;

    /// `wgpu` 렌더링 논리적 장치를 가져옵니다.
    fn render_device(&self) -> &Arc<wgpu::Device>;

    /// `wgpu` 렌더링 명령 대기열을 가져옵니다.
    fn render_queue(&self) -> &Arc<wgpu::Queue>;

    /// `wgpu` 렌더링 장치 표면을 가져옵니다.
    fn render_surface(&self) -> Option<&Arc<wgpu::Surface<'static>>>;

    /// 애플리케이션 창을 가져옵니다.
    fn window(&self) -> Option<&Arc<Window>>;

    /// 장면 흐름을 설정합니다.
    /// 이미 설정된 장면 흐름이 있는 경우 새로운 장면 흐름으로 덮어씁니다.
    /// 
    /// 설정된 장면 흐름은 바로 반영되지 않습니다.
    /// 게임 장면 스택의 `flush`함수가 호출될 때 장면 흐름이 반영됩니다.
    /// 
    fn set_scene_flow(&self, flow: GameSceneFlow);
}
