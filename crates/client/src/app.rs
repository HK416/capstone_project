//! 클라이언트 애플리케이션의 게임 루프와 관련된 코드를 작성합니다.
//!  

use super::error::AppError;
use super::error::show_error_msg;
use super::render::DrawContext;
use super::render::DrawSurface;
use super::render::DrawDevice;

use std::num::NonZeroUsize;
use std::panic;
use std::path::PathBuf;
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
use winit::window::Icon;
use winit::window::Window;
use winit::window::WindowId;


/// 기본 애플리케이션 타이틀 문자열 입니다.
const DEF_TITLE_STR: &'static str = "Hello, World";



/// 애플리케이션을 실행할 때 사용할 옵션 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
pub struct AppOptions {
    /// 애플리케이션에서 사용 가능한 최대 스레드의 갯수 입니다.
    /// 
    /// ※ 기본 값은 현재 시스템의 물리적 코어의 갯수입니다.
    /// 
    pub num_threads: NonZeroUsize,

    /// 현재 프레임 레이트 표시 여부입니다.
    /// 
    /// ※ Debug 모드의 경우 기본 값은 `true` 입니다.
    /// 
    pub show_frame_rate: bool, 

    /// 렌더러의 디버깅 레이어 활성화 여부입니다.
    /// 
    /// ※ Debug 모드의 경우 기본 값은 `true` 입니다.
    /// 
    pub enable_debug_layer: bool, 

    /// 애플리케이션 창의 전체 화면 여부입니다.
    /// 
    /// ※ 기본 값은 `false` 입니다.
    /// 
    pub fullscreen: bool, 

    /// 애플리케이션 창의 크기 조절 여부입니다.
    /// 
    /// ※ 기본 값은 `false` 입니다.
    /// 
    pub resizable: bool,
}

impl Default for AppOptions {
    #[inline(always)]
    fn default() -> Self {
        Self { 
            num_threads: NonZeroUsize::new(num_cpus::get_physical()).unwrap(), 
            show_frame_rate: if cfg!(debug_assert) { true } else { false }, 
            enable_debug_layer: if cfg!(debug_assert) { true } else { false }, 
            fullscreen: false, 
            resizable: false 
        }
    }
}



/// 클라이언트 애플리케이션을 생성하는 빌더입니다.
pub struct AppBuilder {
    /// 애플리케이션 생성 옵션입니다.
    pub options: AppOptions,

    /// 애플리케이션의 실행 디렉토리 경로 입니다.
    pub current_dir: PathBuf,

    /// 애플리케이션 타이틀 문자열 입니다.
    /// 
    /// ※ 기본 값은 `DEF_TITLE_STR` 입니다.
    /// 
    pub title: String,

    /// 애플리케이션 아이콘 이미지 입니다.
    /// 
    /// ※ 기본 값은 `None` 입니다.
    /// 
    pub icon: Option<Icon>,
}

impl AppBuilder {
    /// 애플리케이션 빌더를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<P: Into<PathBuf>>(current_dir: P) -> Self {
        Self { 
            options: AppOptions::default(), 
            current_dir: current_dir.into(), 
            title: DEF_TITLE_STR.to_string(), 
            icon: None 
        }
    }

    /// 애플리케이션 타이틀 문자열을 설정합니다.
    #[inline]
    #[must_use]
    #[allow(unused)]
    pub fn set_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = title.into();
        self
    }

    /// 애플리케이션 아이콘 파일 경로를 설정합니다.
    #[inline]
    #[must_use]
    #[allow(unused)]
    pub fn set_icon<I: Into<Icon>>(mut self, icon: I) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// 애플리케이션을 빌드하고 실행합니다.
    #[inline]
    pub fn build_and_run(self) {
        log::info!("----------애플리케이션 옵션 정보----------");
        log::info!("• 애플리케이션 타이틀: {:?}", self.title);
        log::info!("• 애플리케이션 아이콘 경로: {:?}", self.icon);
        log::info!("• 실행 가능한 스레드 수: {}", self.options.num_threads);
        log::info!("• 프레임 레이트 표시: {}", self.options.show_frame_rate);
        log::info!("• 렌더러 디버깅 레이어 활성화: {}", self.options.enable_debug_layer);
        log::info!("• 전체 화면 모드: {}", self.options.fullscreen);
        log::info!("• 창 크기 조절: {}", self.options.resizable);
        log::info!("");

        // 애플리케이션 이벤트 루프를 생성합니다.
        let event_loop: EventLoop<()> = match EventLoop::with_user_event().build() {
            Ok(event_loop) => event_loop,
            Err(err) => {
                show_error_msg(
                    "Application initialize failed", 
                    &AppError::from(err).to_string(), 
                    None
                );
                process::exit(-1)
            },
        };

        // 애플리케이션 이벤트 루프를 실행합니다.
        #[cfg(not(target_arch = "wasm32"))] {
            let result = pollster::block_on(App::run(
                self.current_dir, 
                self.title, 
                self.icon, 
                self.options, 
                event_loop
            ));
            
            if let Err(err) = result {
                show_error_msg(
                    "Application launching failed", 
                    &AppError::from(err).to_string(), 
                    None
                );
                process::exit(-1)
            };
        }
    }
}



/// 클라이언트 애플리케이션을 관리합니다.
#[derive(Debug)]
pub struct App<T: 'static> {
    /// 현재 애플리케이션 실행 경로 입니다.
    current_dir: PathBuf,

    /// 애플리케이션 타이틀 문자열 입니다.
    title: String,

    /// 애플리케이션 아이콘 이미지 입니다.
    icon: Option<Icon>,

    /// 애플리케이션 옵션 입니다.
    options: AppOptions,

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
    /// 애플리케이션 이벤트 루프를 실행합니다.
    async fn run(
        current_dir: PathBuf, 
        title: String,
        icon: Option<Icon>,
        options: AppOptions,
        event_loop: EventLoop<T>
    ) -> Result<(), AppError> {
        let timer = GameTimer::default();
        let context = DrawContext::new(options.enable_debug_layer).await?;
        let device = DrawDevice::new(&context.adapter).await?;

        let mut app = App {
            current_dir,
            title,
            icon,
            options,
            timer,
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
            .with_title(&self.title)
            .with_window_icon(self.icon.clone())
            .with_visible(false)
            .with_resizable(self.options.resizable);

        // 창을 생성합니다.
        let window: Arc<Window> = event_loop
            .create_window(attributes)
            .map_err(|e| AppError::from(e))?
            .into();
        log::info!("Created new window (ID: {:?})", window.id());

        // TODO: 전체 화면 동작 구현.

        // 렌더링 표면을 생성합니다.
        self.surface = DrawSurface::new(
            window.clone(), 
            &self.context.instance, 
            &self.context.adapter
        )?.into();

        // 창을 표시합니다.
        window.set_visible(true);
        
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
                drop(self.surface.take());
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
