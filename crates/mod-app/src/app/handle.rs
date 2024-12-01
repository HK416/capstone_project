use std::{path::Path, sync::Arc};

use rayon::ThreadPool;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
    asset::AssetManager,
    etc::{AppEvent, AppFlags, GameTimer, WindowSize},
    net::NetManager,
};

/// 외부에서 애플리케이션에 접근할 수 있는 `trait`입니다.
pub trait AppHandle {
    /// 애플리케이션 이벤트 루프 프록시를 가져옵니다.
    fn event_loop_proxy(&self) -> &Arc<EventLoopProxy<AppEvent>>;

    /// 입/출력 스레드 풀 객체를 가져옵니다.
    fn io_threads(&self) -> &ThreadPool;

    /// 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    fn current_dir(&self) -> &Path;

    /// 애플리케이션 에셋 관리자를 가져옵니다.
    fn asset_manager(&self) -> &AssetManager;

    /// 애플리케이션 네트워크 매니저를 가져옵니다.
    fn net_manager(&self) -> &NetManager;

    /// 애플리케이션 생성 플래그를 가져옵니다.
    fn flags(&self) -> AppFlags;

    /// 애플리케이션 창 타이틀 텍스트를 가져옵니다.
    fn window_title(&self) -> &str;

    /// 애플리케이션 창의 크기를 가져옵니다.
    fn window_size(&self) -> &WindowSize;

    /// 애플리케이션 게임 타이머를 가져옵니다.
    fn timer(&self) -> &GameTimer;

    /// `wgpu` 렌더링 인스턴스를 가져옵니다.
    fn render_instance(&self) -> &Arc<wgpu::Instance>;

    /// `wgpu` 렌더링 장치 어댑터를 가져옵니다.
    fn render_adapter(&self) -> &Arc<wgpu::Adapter>;

    /// `wgpu` 렌더링 논리적 장치를 가져옵니다.
    fn render_device(&self) -> &Arc<wgpu::Device>;

    /// `wgpu` 렌더링 명령 대기열을 가져옵니다.
    fn render_queue(&self) -> &Arc<wgpu::Queue>;

    /// `egui` 컨텍스트를 가져옵니다.
    fn egui_ctx(&self) -> &egui::Context;

    /// `egui` 입력기를 가져옵니다.
    fn egui_raw_input(&self) -> egui::RawInput;

    /// 애플리케이션 창을 가져옵니다.
    fn window(&self) -> Option<&Arc<Window>>;

    /// `wgpu` 렌더링 표면을 가져옵니다.
    fn render_surface(&self) -> Option<&Arc<wgpu::Surface<'static>>>;
}
