mod builder;
pub use self::builder::*;

mod config;
pub use self::config::*;

mod delegate;
pub use self::delegate::*;

mod dpi;
pub use self::dpi::*;

mod error;
pub use self::error::*;

mod event;
pub use self::event::*;

mod flag;
pub use self::flag::*;

mod handler;
pub use self::handler::*;

mod impl_winit;

mod locale;
pub use self::locale::*;

use std::sync::Arc;
use std::cell::RefCell;
use std::path::PathBuf;
use winit::dpi::PhysicalSize;
use winit::window::Fullscreen;
use winit::window::Icon;
use winit::window::Window;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoop;
use framework::timer::GameTimer;
use winit::window::WindowButtons;

use crate::command::parse_command_line_args;
use crate::render::init::init_wgpu_renderer;
use crate::render::targets::config_swapchain;
use crate::err_msg;
use crate::error::success;
use crate::error::set_panic_hooker;
use crate::error::show_error_msg;
use crate::error::DebugInfo;
use crate::error::ErrorMessage;
use crate::scene::SceneManager;



/// 애플리케이션 입니다.
#[derive(Debug)]
pub struct App {
    /// 애플리케이션에서 사용 가능한 최대 스레드 갯수 입니다.
    num_threads: usize,

    /// 애플리케이션 실행 디렉토리 경로 입니다.
    current_dir: PathBuf,

    /// 애플리케이션 플래그 옵션 입니다.
    flags: AppFlags,

    /// 애플리케이션 표시 언어 입니다.
    locale: Option<AppLocale>, 


    /// <b>현재</b> 애플리케이션 창 타이틀 문자열 입니다.
    title: String,

    /// <b>현재</b> 애플리케이션 창 아이콘 이미지 데이터 입니다.
    icon: Option<Icon>,

    /// <b>현재</b> 애플리케이션 창의 크기 입니다.
    dpi: Dpi,

    /// <b>현재</b> 애플리케이션 창의 전체화면 여부입니다.
    fullscreen: bool,

    /// 애플리케이션 `delegate` 입니다.
    delegate: RefCell<Box<dyn AppDelegate>>,

    /// 게임 장면의 관리자 입니다.
    scene_manager: RefCell<SceneManager>,

    /// 특정 시각의 경과 시간을 측정하는 타이머 입니다.
    timer: GameTimer,


    /// `wgpu` 렌더러의 인스턴스 입니다.
    instance: Arc<wgpu::Instance>,

    /// `wgpu` 렌더러의 장치 어뎁터 입니다.
    adapter: Arc<wgpu::Adapter>,

    /// `wgpu` 렌더러의 논리적 장치 입니다.
    device: Arc<wgpu::Device>,

    /// `wgpu` 렌더러의 명령어 대기열 입니다.
    queue: Arc<wgpu::Queue>,


    /// 생성된 애플리케이션 창 입니다.
    window: Option<Arc<Window>>,

    /// 생성된 `wgpu` 렌더러의 장치 표면 입니다.
    surface: Option<Arc<wgpu::Surface<'static>>>,
}

impl App {
    /// 애플리케이션을 생성합니다.
    async fn new(builder: AppBuilder) -> Result<Self, ErrorMessage> {
        // 명령줄 인수를 구문 분석 하고, 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
        let (builder, path) = parse_command_line_args(builder)?;

        // `wgpu` 렌더러 인스턴스를 생성 합니다.
        let enable_debug_layer = builder.flags.contains(AppFlags::ENABLE_DEBUG_LAYER);
        let (
            instance, 
            adapter, 
            device, 
            queue
        ) = init_wgpu_renderer(enable_debug_layer).await?;

        Ok(Self { 
            num_threads: builder.num_threads, 
            current_dir: path, 
            flags: builder.flags, 
            locale: None,
            title: builder.title, 
            icon: builder.icon, 
            dpi: builder.dpi.unwrap_or(Dpi::W3840H2160), 
            fullscreen: builder.fullscreen, 
            delegate: RefCell::new(builder.delegate),
            scene_manager: RefCell::new(SceneManager::new(builder.start_scene)),
            timer: GameTimer::default(), 
            instance, 
            adapter, 
            device, 
            queue, 
            window: None, 
            surface: None, 
        })
    }

    /// 애플리케이션을 실행합니다.
    /// 
    /// - 애플리케이션은 오직 하나만 존재할 수 있습니다.
    /// - 애플리케이션 생성 또는 실행 도중 오류가 발생한 경우 에러 메시지를 출력하고 애플리케이션을 종료시킵니다.
    /// 
    #[cfg(target_pointer_width = "64")]
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) fn run(builder: AppBuilder) {
        // `panic!` 후커를 설정합니다.
        set_panic_hooker(None);

        // 이벤트 루프를 생성합니다.
        // ※ 이벤트 루프는 재생성 할 수 없습니다.
        let event_loop: EventLoop<AppEvent> = success!(
            "Event Loop Creation Failed", 
            EventLoop::with_user_event()
                .build()
                .map_err(|e| err_msg!(WindowError::from(e))), 
            None
        );

        // 애플리케이션을 생성하고 인스턴스에 등록합니다.
        let mut app = success!(
            "Application Creation Failed", 
            pollster::block_on(App::new(builder)), 
            None
        );

        // 애플리케이션을 실행합니다.
        success!(
            "Application Runtime Error", 
            event_loop.run_app(&mut app).map_err(|e| WindowError::from(e)), 
            None
        );
    }
}

impl App {
    /// 숨겨져 있는 애플리케이션 창을 생성합니다.
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Arc<Window>, ErrorMessage> {
        // 애플리케이션에서 사용 가능한 최대 해상도를 찾습니다.
        let maximize_dpi = find_maximize_dpi(event_loop)
            .ok_or(err_msg!(WindowError::NoSuitableResolution))?;
        self.dpi = self.dpi.min(maximize_dpi);
        let px_size: PhysicalSize<u32> = self.dpi.into();

        // 애플리케이션 창을 생성합니다.
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes()
            .with_title(self.title.as_str())
            .with_window_icon(self.icon.clone())
            .with_inner_size(px_size)
            .with_fullscreen(self.fullscreen.then_some(Fullscreen::Borderless(None)))
            .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
            .with_resizable(false)
            .with_visible(true)
            .with_active(true);
        
        #[cfg(target_os = "windows")] {
            use winit::platform::windows::WindowAttributesExtWindows;
            use winit::platform::windows::CornerPreference;
            attributes = attributes.with_corner_preference(CornerPreference::DoNotRound);
        }

        event_loop.create_window(attributes)
            .map(|window| window.into())
            .map_err(|e| err_msg!(WindowError::from(e)))
    }
}

impl App {
    /// 애플리케이션이 최초 초기화 될 때 한번만 호출되는 콜백 함수입니다.
    fn on_launching(&mut self, window: &Window, event_loop: &ActiveEventLoop) {
        // 애플리케이션 타이머를 초기화 합니다.
        self.timer.reset();

        // 게임 장면을 갱신합니다.
        if let Err(e) = self.scene_manager.borrow_mut().update(window, self) {
            show_error_msg(
                "Application Launching Failed", 
                e.to_string(), 
                self.window.as_deref()
            );
            return event_loop.exit();
        }

        // 애플리케이션 대리자의 콜백 함수를 호출합니다.
        if let Err(e) = self.delegate.borrow_mut().on_launching(window, event_loop, self) {
            show_error_msg(
                "Application Launching Failed", 
                e.to_string(), 
                self.window.as_deref()
            );
            return event_loop.exit();
        }
    }

    /// 애플리케이션이 종료될 때 한번만 호출되는 콜백 함수입니다.
    fn on_finish(&mut self, event_loop: &ActiveEventLoop) {
        // 애플리케이션 대리자를 호출합니다.
        success!(
            "Application Finish Failure", 
            self.delegate.borrow_mut().on_finish(event_loop, self), 
            self.window.as_deref()
        );

        // 게임 장면을 정리합니다.
        success!(
            "Application Finish Failure", 
            self.scene_manager.borrow_mut().clear(self), 
            self.window.as_deref()
        );
    }

    /// 애플리케이션이 일시 중단 될 때 (애플리케이션 창이 초점을 잃을 때) 호출되는 콜백 함수 입니다.
    fn on_paused(&mut self, window: &Window) -> Result<(), ErrorMessage> {
        // 애플리케이션 대리자의 콜백 함수를 호출합니다.
        self.delegate.borrow_mut().on_paused(window, self)?;

        // 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow_mut().scene_handle_paused(self)?;

        Ok(())
    }

    /// 애플리케이션이 재개될 때 (애플리케이션 창이 초점을 가질 때) 호출되는 콜백 함수 입니다.
    fn on_resumed(&mut self, window: &Window) -> Result<(), ErrorMessage> {
        // 애플리케이션 대리자의 콜백 함수를 호출합니다.
        self.delegate.borrow_mut().on_resumed(window, self)?;

        // 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow_mut().scene_handle_resumed(self)?;

        Ok(())
    }

    /// 애플리케이션 창의 종료 버튼이 눌렸을 때 호출되는 함수입니다.
    /// 
    /// 애플리케이션을 종료하려는 경우 `true`를 반환해야 합니다.
    /// 
    #[inline]
    fn on_close(&mut self) -> Result<bool, ErrorMessage> {
        self.scene_manager.borrow_mut().scene_handle_close_request(self)
    }

    /// 애플리케이션 창의 크기가 변경될 때 호출되는 함수입니다.
    fn on_resized(&mut self, window: &Window, surface: &wgpu::Surface) -> Result<(), ErrorMessage> {
        // 애플리케이션 창의 가로 또는 세로 크기가 0인 경우 함수 실행을 생략합니다.
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(())
        }

        // 이전 모든 렌더링 작업이 끝날 때 까지 대기합니다.
        self.instance.poll_all(true);

        // 변경된 크기로 스왑체인을 설정합니다.
        config_swapchain(size.width, size.height, &self.device, &surface);

        // 현재 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow_mut()
            .scene_handle_window_resized(window, self)?;

        Ok(())
    }

    /// 애플리케이션 그리기 명령이 호출되었을 때 호출되는 함수입니다.
    fn on_draw(&mut self, window: &Window, surface: &wgpu::Surface) -> Result<(), ErrorMessage> {
        // `winit` API에 애플리케이션 창을 갱신한다고 알립니다.
        window.pre_present_notify();

        // 현재 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow().scene_draw(window, surface, self)?;

        Ok(())
    }
}
