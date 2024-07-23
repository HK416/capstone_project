pub mod builder;
pub mod cmd;
pub mod config;
pub mod delegate;
pub mod dpi;
pub mod event;
pub mod flag;
pub mod locale;

use crate::app_error;
use crate::handle_error;
use crate::error::AppError;
use crate::scene::manager::SceneManager;
use self::builder::AppBuilder;
use self::dpi::Dpi;
use self::delegate::AppDelegate;
use self::event::AppEvent;
use self::flag::AppFlags;
use self::locale::AppLocale;

use core::cell::RefCell;
use std::sync::Arc;
use std::sync::Once;
use std::path::Path;
use std::path::PathBuf;
use framework::timer::GameTimer;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::StartCause;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Fullscreen;
use winit::window::Icon;
use winit::window::Window;
use winit::event_loop::EventLoop;
use winit::window::WindowButtons;
use winit::window::WindowId;



/// 애플리케이션 인터페이스 `trait` 입니다.
pub trait Application : core::fmt::Debug {
    /// 애플리케이션에서 사용 가능한 최대 스레드 갯수를 반환합니다.
    fn get_num_threads(&self) -> usize;

    /// 애플리케이션 실행 디렉토리 경로를 빌려옵니다.
    fn ref_current_dir(&self) -> &Path;

    /// 애플리케이션 생성 플래그를 반환합니다.
    fn get_flags(&self) -> AppFlags;

    /// 애플리케이션 표시 언어를 빌려옵니다.
    fn ref_locale(&self) -> Option<&AppLocale>;

    /// 애플리케이션 타이머를 빌려옵니다.
    fn ref_timer(&self) -> &GameTimer;

    /// `wgpu` 렌더러의 인스턴스를 빌려옵니다.
    fn ref_render_instance(&self) -> &Arc<wgpu::Instance>;

    /// `wgpu` 렌더러의 장치 어뎁터를 빌려옵니다.
    fn ref_render_adapter(&self) -> &Arc<wgpu::Adapter>;

    /// `wgpu` 렌더러의 논리적 장치를 빌려옵니다.
    fn ref_render_device(&self) -> &Arc<wgpu::Device>;

    /// `wgpu` 렌더러의 명령 대기열을 빌려옵니다.
    fn ref_render_queue(&self) -> &Arc<wgpu::Queue>;

    /// 애플리케이션 창과 `wgpu` 렌더러의 표면을 빌려옵니다.
    fn ref_window_and_render_surface(&self) -> Option<(&Arc<Window>, &Arc<wgpu::Surface>)>;
}



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
    async fn new(builder: AppBuilder) -> Result<Self, AppError> {
        use self::cmd::parse_command_line_args;
        use crate::render::create_renderer;

        // 명령줄 인수를 구문 분석 하고, 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
        let (builder, path) = parse_command_line_args(builder)?;

        // `wgpu` 렌더러 인스턴스를 생성 합니다.
        let enable_debug_layer = builder.flags.contains(AppFlags::ENABLE_DEBUG_LAYER);
        let (instance, adapter, device, queue) = create_renderer(enable_debug_layer).await?;

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
    pub(super) fn run(builder: AppBuilder) {
        // 이벤트 루프를 생성합니다.
        // ※ 이벤트 루프는 재생성 할 수 없습니다.
        let event_loop: EventLoop<AppEvent> = match EventLoop::with_user_event().build() {
            Ok(it) => it,
            Err(e) => handle_error!("Application initialization failure", app_error!(e), None), 
        };

        // 애플리케이션을 생성하고 인스턴스에 등록합니다.
        let mut app = match pollster::block_on(App::new(builder)) {
            Ok(it) => it,
            Err(e) => handle_error!("Application initialization failure", e, None),
        };

        if let Err(e) = event_loop.run_app(&mut app) {
            handle_error!("Application runtime error", e, None);
        }
    }
}

impl App {
    /// 숨겨져 있는 애플리케이션 창을 생성합니다.
    fn create_hide_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Window, AppError> {
        use self::dpi::find_maximize_dpi;

        // 애플리케이션에서 사용 가능한 최대 해상도를 찾습니다.
        let maximize_dpi = find_maximize_dpi(event_loop)
            .ok_or(AppError::NoSuitableResolution)?;
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
            .with_visible(false);
        
        #[cfg(target_os = "windows")] {
            use winit::platform::windows::WindowAttributesExtWindows;
            use winit::platform::windows::CornerPreference;
            attributes = attributes.with_corner_preference(CornerPreference::DoNotRound);
        }

        event_loop.create_window(attributes)
            .map_err(|e| AppError::from(e))
    }
}

impl App {
    /// 애플리케이션이 최초 초기화 될 때 한번만 호출되는 콜백 함수입니다.
    fn on_launching(&mut self, window: &Window, event_loop: &ActiveEventLoop) {
        static VAL: Once = Once::new();
        VAL.call_once(|| {
            // 애플리케이션 타이머를 초기화 합니다.
            self.timer.reset();

            // 게임 장면을 갱신합니다.
            let result = self.scene_manager.borrow_mut().update(window, self);
            if let Err(e) = result {
                handle_error!("Application launching failure", e, Some(window));
            }

            // 애플리케이션 대리자의 콜백 함수를 호출합니다.
            let result = self.delegate.borrow_mut().on_launching(window, event_loop, self);
            if let Err(e) = result {
                handle_error!("Application launching failure", e, Some(window));
            }
        })
    }

    /// 애플리케이션이 종료될 때 한번만 호출되는 콜백 함수입니다.
    fn on_finish(&mut self, event_loop: &ActiveEventLoop) {
        static VAL: Once = Once::new();
        VAL.call_once(|| {
            // 애플리케이션 대리자를 호출합니다.
            let result = self.delegate.borrow_mut().on_finish(event_loop, self);
            if let Err(e) = result {
                handle_error!("Application finish failure", e, None);
            }

            // 게임 장면을 정리합니다.
            let result = self.scene_manager.borrow_mut().clear(self);
            if let Err(e) = result {
                handle_error!("Application finish failure", e, None);
            }
        })
    }

    /// 애플리케이션이 일시 중단 될 때 (애플리케이션 창이 초점을 잃을 때) 호출되는 콜백 함수 입니다.
    fn on_paused(
        &mut self, 
        window: &Window
    ) -> Result<(), AppError> {
        // 애플리케이션 대리자의 콜백 함수를 호출합니다.
        self.delegate.borrow_mut()
            .on_paused(window, self)?;

        // 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow_mut()
            .scene_handle_paused(self)?;

        Ok(())
    }

    /// 애플리케이션이 재개될 때 (애플리케이션 창이 초점을 가질 때) 호출되는 콜백 함수 입니다.
    fn on_resumed(
        &mut self, 
        window: &Window
    ) -> Result<(), AppError> {
        // 애플리케이션 대리자의 콜백 함수를 호출합니다.
        self.delegate.borrow_mut()
            .on_resumed(window, self)?;

        // 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow_mut()
            .scene_handle_resumed(self)?;

        Ok(())
    }

    /// 애플리케이션 창의 종료 버튼이 눌렸을 때 호출되는 함수입니다.
    /// 
    /// 애플리케이션을 종료하려는 경우 `true`를 반환해야 합니다.
    /// 
    #[inline]
    fn on_close(&mut self) -> Result<bool, AppError> {
        self.scene_manager.borrow_mut()
            .scene_handle_close_request(self)
    }

    /// 애플리케이션 창의 크기가 변경될 때 호출되는 함수입니다.
    fn on_resized(&mut self, window: &Window, surface: &wgpu::Surface) -> Result<(), AppError> {
        use crate::render::config_swapchain;
        
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
    fn on_draw(
        &mut self, 
        window: &Window, 
        surface: &wgpu::Surface
    ) -> Result<(), AppError> {
        // `winit` API에 애플리케이션 창을 갱신한다고 알립니다.
        window.pre_present_notify();

        // 현재 게임 장면의 콜백 함수를 호출합니다.
        self.scene_manager.borrow()
            .scene_draw(window, surface, self)?;

        Ok(())
    }
}

impl Application for App {
    #[inline]
    #[must_use]
    fn get_num_threads(&self) -> usize {
        self.num_threads
    }

    #[inline]
    #[must_use]
    fn ref_current_dir(&self) -> &Path {
        &self.current_dir
    }

    #[inline]
    #[must_use]
    fn get_flags(&self) -> AppFlags {
        self.flags
    }

    #[inline]
    #[must_use]
    fn ref_locale(&self) -> Option<&AppLocale> {
        self.locale.as_ref()
    }

    #[inline]
    #[must_use]
    fn ref_timer(&self) -> &GameTimer {
        &self.timer
    }

    #[inline]
    #[must_use]
    fn ref_render_instance(&self) -> &Arc<wgpu::Instance> {
        &self.instance
    }

    #[inline]
    #[must_use]
    fn ref_render_adapter(&self) -> &Arc<wgpu::Adapter> {
        &self.adapter
    }

    #[inline]
    #[must_use]
    fn ref_render_device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    #[inline]
    #[must_use]
    fn ref_render_queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    #[inline]
    #[must_use]
    fn ref_window_and_render_surface(&self) -> Option<(&Arc<Window>, &Arc<wgpu::Surface>)> {
        self.window.as_ref().zip(self.surface.as_ref())
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        // 타이머를 갱신합니다.
        self.timer.tick();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `winit` API는 애플리케이션이 생성되었을 때 `ApplicationHandler::resumed`를 호출합니다.
        // 또한 일부 시스템은 애플리케이션 초기화 이전에 창을 생성하는 것이 허용되지 않습니다.
        // 따라서 이 콜백 함수에서 애플리케이션 창을 생성하고, 렌더러 표면을 생성해야 합니다.
        //
        use crate::render::config_swapchain;
        use crate::render::create_wgpu_surface;

        // 애플리케이션 창을 생성합니다.
        let window = match self.create_hide_window(event_loop) {
            Ok(it) => Arc::new(it),
            Err(e) => handle_error!("Application window creation failure", e, None),
        };

        // `wgpu` 렌더링 표면을 생성합니다.
        let surface = match create_wgpu_surface(window.clone(), &self.instance, &self.adapter) {
            Ok(it) => it,
            Err(e) => handle_error!("Application render surface creation failure", e, Some(&window)),
        };

        // 스왑체인을 설정합니다.
        let size = window.inner_size();
        config_swapchain(size.width, size.height, &self.device, &surface);

        // 애플리케이션 시작 콜백 함수를 호출합니다.
        self.on_launching(&window, event_loop);

        // 애플리케이션 창을 보여줍니다.
        window.set_visible(true);

        // 애플리케이션에 저장합니다.
        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        // 애플리케이션 종료 콜백 함수를 호출합니다.
        self.on_finish(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.window.clone();
        let surface = self.surface.clone();
        if let Some((window, _)) = window.zip(surface) {
            // 게임 장면이 비어있는 경우 애플리케이션을 종료합니다.
            if self.scene_manager.borrow().is_empty() {
                return event_loop.exit();
            } 

            if let Err(e) = self.scene_manager.borrow_mut().scene_update(&window, self) {
                handle_error!("Application runtime error", e, self.window.as_deref());
            }

            // 등록된 애플리케이션 창이 존재할 경우 애플리케이션 창을 갱신합니다.
            window.request_redraw();
        } else {
            // 등록된 애플리케이션 창이 없는 경우 애플리케이션을 종료합니다.
            event_loop.exit();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: AppEvent) {
        /* empty */
    }

    fn window_event(
        &mut self,
        _: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // MEMO: 이 콜백 함수 안에서 이벤트 루프를 통한 종료를 하면 에러가 발생합니다.
        // 
        use crate::error::show_error_msg;

        // 애플리케이션 창과 렌더링 표면을 가져옵니다.
        // 애플리케이션 창 또는 렌더링 표면이 없는 경우 (애플리케이션의 종료) 함수 실행을 생략합니다.
        let window = self.window.clone();
        let surface = self.surface.clone();
        let (window, surface) = match window.zip(surface) {
            Some(it) => it,
            None => return,
        };

        // 애플리케이션 창 식별자가 다른 경우 함수 실행을 생략합니다.
        if window_id != window.id() {
            return;
        }

        // 애플리케이션 창 이벤트를 처리합니다.
        if let Err(e) = match event {
            WindowEvent::Focused(focused) => match focused {
                true => self.on_resumed(&window),
                false => self.on_paused(&window)
            }, 
            WindowEvent::Resized(_) => self.on_resized(&window, &surface),
            WindowEvent::RedrawRequested => self.on_draw(&window, &surface),
            WindowEvent::CloseRequested => match self.on_close() {
                Ok(exiting) => {
                    if exiting {
                        drop(self.window.take());
                        drop(self.surface.take());
                    }
                    Ok(())
                },
                Err(e) => Err(e),
            },
            _ => { Ok(()) }
        } {
            show_error_msg(
                "Application runtime error", 
                e.to_string(), 
                self.window.as_deref()
            );
            drop(self.window.take());
            drop(self.surface.take());
        }
    }
}
