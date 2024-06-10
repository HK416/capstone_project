//! 클라이언트 애플리케이션의 게임 루프와 관련된 코드를 작성합니다.
//!  

use super::error::AppError;
use super::error::show_error_msg;
use super::render::DrawContext;
use super::render::DrawSurface;
use super::render::DrawDevice;

use std::panic;
use std::process;
use std::sync::Arc;
use std::sync::Weak;
use std::marker::PhantomData;
use framework::timer::GameTimer;
use winit::application::ApplicationHandler;
use winit::event::StartCause;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use winit::window::WindowId;



/// 클라이언트 애플리케이션을 생성하는 빌더입니다.
pub struct AppBuilder {
    /// 현재 프레임 레이트 표시 여부입니다.
    /// 
    /// ※ Debug 모드의 경우 기본 값은 `true` 입니다.
    /// 
    show_frame_rate: bool,

    /// 렌더러의 디버깅 레이어 활성화 여부입니다.
    /// 
    /// ※ Debug 모드의 경우 기본 값은 `true` 입니다.
    /// 
    enable_debug_layer: bool,
}

impl AppBuilder {
    /// 애플리케이션 빌더를 생성합니다.
    #[must_use]
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// 창에 현재 프레임 레이트 표시를 설정합니다.
    #[inline]
    #[must_use]
    #[allow(unused)]
    pub fn set_show_frame_rate(mut self, show: bool) -> Self {
        self.show_frame_rate = show;
        self
    }

    /// 렌더러의 디버깅 레이어 활성화를 설정합니다.
    #[inline]
    #[must_use]
    #[allow(unused)]
    pub fn set_enable_debug_layer(mut self, enable: bool) -> Self {
        self.enable_debug_layer = enable;
        self
    }

    /// 애플리케이션을 빌드하고 실행합니다.
    #[inline]
    pub fn build_and_run(self) {
        // 애플리케이션 이벤트 루프를 생성합니다.
        let event_loop: EventLoop<()> = match EventLoop::with_user_event().build() {
            Ok(event_loop) => event_loop,
            Err(err) => {
                show_error_msg(
                    "Application build failed", 
                    &AppError::from(err).to_string(), 
                    None
                );
                process::exit(-1)
            },
        };

        // 애플리케이션 이벤트 루프를 실행합니다.
        #[cfg(not(target_arch = "wasm32"))] {
            if let Err(err) = pollster::block_on(App::run(self, event_loop)) {
                show_error_msg(
                    "Application running failed", 
                    &AppError::from(err).to_string(), 
                    None
                );
                process::exit(-1)
            };
        }
    }
}

impl Default for AppBuilder {
    #[must_use]
    #[inline(always)]
    fn default() -> Self {
        Self { 
            show_frame_rate: if cfg!(debug_assert) { true } else { false },
            enable_debug_layer: if cfg!(debug_assert) { true } else { false }, 
        }
    }
}



/// 클라이언트 애플리케이션을 관리합니다.
#[derive(Debug)]
pub struct App<T: 'static> {
    /// 특정 시각의 경과 시간을 측정하는 타이머 입니다.
    timer: GameTimer,

    /// 렌더링 컨텍스트 입니다.
    context: Arc<DrawContext>,

    /// 렌더링 디바이스 입니다.
    device: Arc<DrawDevice>,

    /// 렌더링 표면 입니다.
    surface: Option<Arc<DrawSurface>>,

    /// 사용자 정의 이벤트의 PhantomData
    _phantom: PhantomData<T>
}

impl<T: 'static> App<T> {
    /// 애플리케이션 타이틀 문자열 입니댜.
    pub const APP_TITLE: &'static str = "Hello to Halo!";

    /// 애플리케이션 이벤트 루프를 실행합니다.
    async fn run(builder: AppBuilder, event_loop: EventLoop<T>) -> Result<(), AppError> {
        let context = DrawContext::new(builder.enable_debug_layer).await?;
        let device = DrawDevice::new(&context.adapter).await?;

        let mut app = App {
            timer: GameTimer::default(),
            context,
            device,
            surface: None,
            _phantom: PhantomData,
        };
        
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(&mut app).map_err(|e| AppError::from(e))
    }

    /// 애플리케이션이 최초로 시작되었을 때 호출되는 함수입니다.
    /// 
    /// ※ 타이머의 초기화 또는 첫 번째 장면의 초기화를 여기서 진행합니다.
    /// 
    fn on_create(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("애플리케이션 초기화 수행...");

        // 타이머 초기화
        self.timer.reset();
    }

    /// 애플리케이션이 갱신해야 할 때 호출되는 함수입니다.
    /// 
    /// ※ 현재 장면의 갱신을 여기서 진행합니다.
    /// 
    fn on_update(&mut self, event_loop: &ActiveEventLoop, surface: &DrawSurface) {
        log::debug!("애플리케이션 갱신 수행...");

        let frame_rate = self.timer.frame_rate();
        surface.window.set_title(&format!("{} (FPS:{})", Self::APP_TITLE, frame_rate))
    }

    fn on_destroy(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("애플리케이션 정리 수행...");
    }

    /// 애플리케이션에 새로운 윈도우를 추가합니다.
    /// 
    /// ※ 사용하는 윈도우는 1개이지만, 운영체제에 따라 이 함수를 여러번 호출할 수 있습니다. (예: Android)
    /// 
    fn regist_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Weak<Window>, AppError> { 
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes()
            .with_title(Self::APP_TITLE)
            .with_visible(true)
            .with_resizable(false);

        // 창을 생성합니다.
        let window: Arc<Window> = event_loop
            .create_window(attributes)
            .map_err(|e| AppError::from(e))?
            .into();
        log::info!("Created new window (ID: {:?})", window.id());
        
        // 렌더링 표면을 생성합니다.
        self.surface = DrawSurface::new(
            window.clone(), 
            &self.context.instance, 
            &self.context.adapter
        )?.into();
        
        Ok(Arc::downgrade(&window))
    }
}

impl<T: 'static> ApplicationHandler<T> for App<T> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // 경과 시간을 측정합니다.
        match cause {
            StartCause::Init => self.on_create(event_loop),
            _ => self.timer.tick(),
        };
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.on_destroy(event_loop);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("resumed 호출됨!");
        
        let window = match self.regist_window(event_loop) {
            Ok(window) => window,
            Err(e) => {
                show_error_msg("Window creation failed", &e.to_string(), None);
                return event_loop.exit();
            },
        };

        // `panic!` 호출시 처리를 설정합니다.
        // ※ winit에서 `resumed`를 호출하기 전까지 App<T>의 윈도우가 삭제되지 않음
        panic::set_hook(Box::new(move |info| {
            if let Some(location) = info.location() {
                log::debug!("Calling panic at - File:{}, Line:{}, Column:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }

            if let Some(text) = info.payload().downcast_ref::<&str>() {
                log::error!("{}", text.to_string());
                show_error_msg("Runtime Error", text, window.upgrade().as_deref());
            }

            process::exit(-1);
        }))
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // 등록된 윈도우를 가져오고 없을 경우 함수 실행을 생략한다.
        let surface = match &self.surface {
            Some(surface) if window_id == surface.window.id() => surface,
            _ => return,
        };

        match event {
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }

                self.context.instance.poll_all(true);
                surface.config_swapchain(size.width, size.height, &self.device.device);
            },
            WindowEvent::RedrawRequested => {
                
            },
            WindowEvent::CloseRequested => {
                return event_loop.exit();
            },
            _ => { /* empty */ }
        };
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(surface) = self.surface.clone() {
            // 업데이트 함수 호출
            self.on_update(event_loop, &surface);

            // 등록된 윈도우가 존재할 경우 윈도우를 갱신한다.
            surface.window.request_redraw();
        } else {
            // 등록된 윈도우가 없는 경우 애플리케이션을 종료한다.
            event_loop.exit();
        }
    }
}
