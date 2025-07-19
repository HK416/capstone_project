use std::{path::Path, sync::Arc};

use mod_parallelism::collections::Queue;
use rayon::ThreadPool;
use rodio::{mixer::Mixer, Sink};
use winit::event_loop::EventLoopProxy;

use crate::{
    etc::{AppEvent, AppFlags, GameTimer, Viewport, WindowSize},
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

    /// 애플리케이션 네트워크 매니저를 가져옵니다.
    fn net_manager(&self) -> &NetManager;

    /// 애플리케이션 생성 플래그를 가져옵니다.
    fn flags(&self) -> AppFlags;

    /// 애플리케이션 창 타이틀 텍스트를 가져옵니다.
    fn window_title(&self) -> &str;

    /// 애플리케이션 창의 크기를 가져옵니다.
    fn window_size(&self) -> WindowSize;

    /// 뷰포트 영역을 가져옵니다.
    fn viewport(&self) -> &Viewport;

    /// 애플리케이션 창의 전체화면 여부를 가져옵니다.
    fn is_fullscreen(&self) -> bool;

    /// 애플리케이션 게임 타이머를 가져옵니다.
    fn timer(&self) -> &GameTimer;

    /// 소리 장치 오디오 믹서를 가져옵니다.
    fn audio_mixer(&self) -> &Mixer;

    /// 재생 중인 [`Sink`] 대기열을 가져옵니다.
    fn sink_list(&self) -> &Queue<Sink>;

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
}
