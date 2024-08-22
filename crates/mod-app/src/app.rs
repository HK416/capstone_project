use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use hecs::World;
use mod_error::alert_error;
use mod_error::err_msg;
use mod_error::RuntimeError;
use mod_render::config_swapchain;
use mod_render::create_surface;
use mod_render::init_wgpu;
use mod_scene::SceneManager;
use mod_util::AppDpi;
use mod_util::AppEvent;
use mod_util::AppFlags;
use mod_util::AppHandle;
use mod_util::AppLocale;
use mod_util::GameTimer;
use winit::application::ApplicationHandler;
use winit::event::KeyEvent;
use winit::event::StartCause;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::Fullscreen;
use winit::window::Icon;
use winit::window::Window;
use winit::window::WindowButtons;

use crate::AppBuilder;
use crate::AppError;

/// 고정 시간 갱신에 사용되는 경과 시간입니다.
pub const FIXED_TIME_SEC: f32 = 1.0 / 60.0;

/// 최대 고정 시간 갱신 횟수입니다.
pub const MAX_FIXED_UPDATE: usize = 8;



/// 애플리케이션 데이터를 가진 구조체입니다.
pub struct Application {
    /// 애플리케이션에서 사용 가능한 최대 스레드 갯수입니다.
    num_threads: usize, 

    /// 애플리케이션 실행 디렉토리 경로입니다.
    current_dir: PathBuf, 

    /// 애플리케이션 플래그 옵션입니다.
    flags: AppFlags, 

    /// 애플리케이션 표시 언어입니다.
    locale: Option<AppLocale>, 

    /// 애플리케이션 창 타이틀 문자입니다.
    title: String, 

    /// 애플리케이션 창 타이틀 아이콘 이미지 데이터입니다.
    icon: Option<Icon>, 

    /// 애플리케이션 창의 해상도입니다.
    dpi: AppDpi, 

    /// 애플리케이션 창의 전체화면 여부입니다.
    fullscreen: bool, 

    /// 게임 장면 관리자입니다.
    scene_manager: RefCell<SceneManager>, 

    /// ECS(Entity Component System)입니다.
    world: RefCell<World>, 

    /// 경과 시간을 측정하는 타이머입니다.
    timer: GameTimer, 

    /// 고정 시간 갱신에 사용되는 총 경과 시간입니다.
    total_time_sec: f32, 


    /// `wgpu` 렌더링 인스턴스입니다.
    instance: Arc<wgpu::Instance>, 

    /// `wgpu` 렌더링 장치 어댑터입니다.
    adapter: Arc<wgpu::Adapter>, 

    /// `wgpu` 렌더링 논리적 장치입니다.
    device: Arc<wgpu::Device>, 

    /// `wgpu` 렌더링 장치 명령 대기열입니다.
    queue: Arc<wgpu::Queue>, 


    /// 생성된 애플리케이션 창입니다.
    window: Option<Arc<Window>>, 

    /// 생성된 `wgpu` 렌더링 장치 표면입니다.
    surface: Option<Arc<wgpu::Surface<'static>>>, 
}

impl Application {
    /// 애플리케이션을 생성합니다.
    pub(crate) async fn new(builder: AppBuilder) -> Result<Self, Box<dyn Error>> {
        let enable_debug_layer = builder.flags.contains(AppFlags::ENABLE_DEBUG_LAYER);
        let (instance, adapter, device, queue) = init_wgpu(enable_debug_layer).await?;

        Ok(Self {
            num_threads: builder.num_threads, 
            current_dir: builder.current_dir.unwrap(), 
            flags: builder.flags, 
            locale: None, 
            title: builder.title.unwrap_or("Hello, World!".to_string()), 
            icon: builder.icon, 
            dpi: builder.dpi.unwrap_or(AppDpi::MAX), 
            fullscreen: builder.fullscreen, 
            scene_manager: RefCell::new(SceneManager::new(builder.start_scene)), 
            world: RefCell::new(World::new()), 
            timer: GameTimer::default(), 
            total_time_sec: 0.0, 
            instance, 
            adapter, 
            device, 
            queue, 
            window: None, 
            surface: None, 
        })
    }



    /// 새로운 애플리케이션 창을 생성합니다.
    #[must_use]
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Arc<Window>, Box<dyn Error>> {
        // 시스템에서 사용 가능한 최대 해상도를 가져옵니다.
        let max_dpi = AppDpi::find_maximize_dpi(event_loop)
            .ok_or(err_msg!(AppError::NoSuitableResolution))?;
        self.dpi = self.dpi.min(max_dpi);

        // 애플리케이션 창을 생성합니다.
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes()
            .with_title(self.title.as_str())
            .with_window_icon(self.icon.clone())
            .with_inner_size(self.dpi.size())
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

        let window = event_loop.create_window(attributes)
            .map(|window| window.into())
            .map_err(|e| err_msg!(AppError::System(e.to_string())))?;

        return Ok(window);
    }



    /// 애플리케이션 종료 버튼이 눌렸을 때 호출되는 함수입니다.
    #[must_use]
    fn on_close(&mut self) -> Result<(), Box<dyn Error>> {
        // 현재 장면을 가져옵니다.
        // 이때 현재 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut scene_manager = self.scene_manager.borrow_mut();
        let current_scene = match scene_manager.top() {
            Some(current_scene) => current_scene, 
            None => return Ok(()),
        };

        // 현재 장면이 종료를 원할 경우 애플리케이션 창과 렌더링 표면을 제거합니다.
        if current_scene.on_close(self) {
            drop(self.window.take());
            drop(self.surface.take());
        }

        Ok(())
    }

    /// 애플리케이션이 재개될 때 호출되는 함수입니다.
    #[must_use]
    fn on_resumed(&mut self) -> Result<(), Box<dyn Error>> {
        // 현재 장면을 가져옵니다.
        // 이때 현재 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_manager = self.scene_manager.borrow_mut();
        let current_scene = match scene_manager.top() {
            Some(current_scene) => current_scene, 
            None => return Ok(()),
        };

        // 현재 장면의 콜백 함수를 호출합니다.
        current_scene.on_resume(&mut world, self)
    }

    /// 애플리케이션이 일시정지될 때 호출되는 함수입니다.
    #[must_use]
    fn on_paused(&mut self) -> Result<(), Box<dyn Error>> {
        // 현재 장면을 가져옵니다.
        // 이때 현재 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_manager = self.scene_manager.borrow_mut();
        let current_scene = match scene_manager.top() {
            Some(current_scene) => current_scene, 
            None => return Ok(()),
        };

        // 현재 장면의 콜백 함수를 호출합니다.
        current_scene.on_pause(&mut world, self)
    }

    /// 애플리케이션 키보드 이벤트가 발생했을 때 호출되는 함수입니다.
    #[must_use]
    fn on_keyboard_event(
        &mut self, 
        event: KeyEvent, 
        window: &Window
    ) -> Result<(), Box<dyn Error>> {
        // 현재 장면을 가져옵니다.
        // 이때 현재 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_manager = self.scene_manager.borrow_mut();
        let current_scene = match scene_manager.top() {
            Some(current_scene) => current_scene, 
            None => return Ok(()),
        };

        // 키 코드를 가져옵니다.
        let code = match event.physical_key {
            PhysicalKey::Code(code) if !event.repeat => code,
            _ => return Ok(()),
        };
        let location = event.location;

        // 현재 장면의 콜백 함수를 호출합니다.
        if event.state.is_pressed() {
            current_scene.on_keyboard_pressed(code, location, window, &mut world, self)
        } else {
            current_scene.on_keyboard_released(code, location, window, &mut world, self)
        }
    }

    /// 애플리케이션 그리기 명령이 호출되었을 때 호출되는 함수입니다.
    #[must_use]
    fn on_draw(
        &mut self, 
        window: &Window, 
        surface: &wgpu::Surface<'static>
    ) -> Result<(), Box<dyn Error>> {
        // 현재 장면을 가져옵니다.
        // 이때 현재 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_manager = self.scene_manager.borrow_mut();
        let current_scene = match scene_manager.top() {
            Some(current_scene) => current_scene, 
            None => return Ok(()),
        };

        // `winit` API에 애플리케이션 창을 갱신한다고 알립니다.
        window.pre_present_notify();

        // 현재 게임 장면의 그리기 준비 함수를 호출합니다.
        current_scene.on_prepare_draw(window, surface, &mut world, self)?;

        // 현재 게임 장면의 콜백 함수를 호출합니다.
        current_scene.on_draw(window, surface, &world, self)
    }

    /// 애플리케이션 창의 크기가 변경될 때 호출되는 함수입니다.
    #[must_use]
    fn on_resized(
        &mut self, 
        window: &Window, 
        surface: &wgpu::Surface<'static>
    ) -> Result<(), Box<dyn Error>> {
        // 현재 장면을 가져옵니다.
        // 이때 현재 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_manager = self.scene_manager.borrow_mut();
        let current_scene = match scene_manager.top() {
            Some(current_scene) => current_scene, 
            None => return Ok(()),
        };

        // 창의 크기를 가져옵니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        
        // 애플리케이션 창의 가로 또는 세로 크기가 0인 경우 실행을 생략합니다.
        if width == 0 || height == 0 {
            return Ok(());
        }

        // 이전 모든 렌더링 작업이 끝날 때 까지 대기합니다.
        self.instance.poll_all(true);

        // 변경된 크기로 스왑체인을 재설정합니다.
        let disable_vsync = self.flags.contains(AppFlags::DISABLE_VSYNC);
        config_swapchain(width, height, &self.device, &surface, disable_vsync);

        // 현재 장면의 콜백 함수를 호출합니다.
        current_scene.on_resized(&window, &mut world, self)
    }
}

impl ApplicationHandler<AppEvent> for Application {
    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        self.timer.tick()
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `winit` API는 애플리케이션이 생성되었을 때 `ApplicationHandler::resumed`를 호출합니다.
        // 또한 일부 시스템은 애플리케이션 초기화 이전에 창을 생성하는 것이 허용되지 않습니다.
        // 따라서 이 콜백 함수에서 애플리케이션 창을 생성하고, 렌더러 표면을 생성해야 합니다.
        //
        // 애플리케이션 창을 생성합니다.
        let result = self.create_window(event_loop);
        let window = match result {
            Ok(window) => window, 
            Err(e) => {
                alert_error("Application window creation failed", e.to_string(), None);
                return event_loop.exit();
            }, 
        };

        // `wgpu` 장치 표면을 생성합니다.
        let result = create_surface(window.clone(), &self.instance, &self.adapter);
        let surface = match result {
            Ok(surface) => surface, 
            Err(e) => {
                alert_error("Render surface creation failed", e.to_string(), Some(&window));
                return event_loop.exit();
            }
        };

        // 스왑체인을 설정합니다.
        let disable_vsync = self.flags.contains(AppFlags::DISABLE_VSYNC);
        let (width, height): (u32, u32) = window.inner_size().into();
        config_swapchain(width, height, &self.device, &surface, disable_vsync);

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        // 장면 관리자를 정리합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_manager = self.scene_manager.borrow_mut();
        let result = scene_manager.clear(self.window.as_deref(), &mut world,  self);
        if let Err(e) = result {
            alert_error("Runtime error", e.to_string(), self.window.as_deref());
            process::exit(-1);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.clone() {
            // 장면 관리자를 갱신합니다.
            let mut world = self.world.borrow_mut();
            let mut scene_manager = self.scene_manager.borrow_mut();
            let result = scene_manager.flush(&window, &mut world, self);
            if let Err(e) = result {
                alert_error("Runtime error", e.to_string(), Some(&window));
                return event_loop.exit();
            }

            // 현제 게임 장면을 가져옵니다.
            // 현제 게임 장면이 존재하지 않는 경우 애플리케이션을 종료합니다.
            let current_scene = match scene_manager.top() {
                Some(current_scene) => current_scene, 
                None => return event_loop.exit(), 
            };

            // 총 경과 시간을 갱신합니다.
            let elapsed_time_sec = self.timer.elapsed_time_sec();
            self.total_time_sec += elapsed_time_sec;

            // 변동 시간 갱신 함수를 호출합니다.
            let result = current_scene.on_update(elapsed_time_sec, &window, &mut world, self);
            if let Err(e) = result {
                alert_error("Runtime error", e.to_string(), Some(&window));
                return event_loop.exit();
            }

            // 고정 시간 갱신 함수를 호출합니다.
            let mut count = 0;
            while FIXED_TIME_SEC <= self.total_time_sec && count < MAX_FIXED_UPDATE {
                let result = current_scene.on_fixed_update(FIXED_TIME_SEC, &window, &mut world, self);
                if let Err(e) = result {
                    alert_error("Runtime error", e.to_string(), Some(&window));
                    return event_loop.exit();
                }
                self.total_time_sec -= FIXED_TIME_SEC;
                count += 1;
            }

            // 이떄 최대 갱신 횟수를 초과할 경우 변동 시간 간격을 전달합니다.
            if count == MAX_FIXED_UPDATE {
                let result = current_scene.on_fixed_update(self.total_time_sec, &window, &mut world, self);
                if let Err(e) = result {
                    alert_error("Runtime error", e.to_string(), Some(&window));
                    return event_loop.exit();
                }
                self.total_time_sec = 0.0;
            }

            // 애플리케이션 창을 갱신합니다.
            window.request_redraw();
        } else {
            // 애플리케이션 창이 없는 경우 애플리케이션을 종료합니다.
            return event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        _: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // NOTE: 이 함수 안에서 이벤트 루트를 통한 종료를 할 경우 에러가 발생합니다.
        //

        // 애플리케이션 창과 렌더링 장치 표면을 가져옵니다.
        // 애플리케이션 창과 렌더링 장치 표면이 없는 경우 함수 실행을 생략합니다.
        let window = self.window.clone();
        let surface = self.surface.clone();
        let (window, surface) = match window.zip(surface) {
            Some(tuple) => tuple, 
            None => return, 
        };

        // 애플리케이션 창 식별자가 다를 경우 함수 실행을 생략합니다.
        if window_id != window.id() {
            return;
        }

        // 윈도우 이벤트를 처리합니다.
        let result = match event {
            WindowEvent::CloseRequested => {
                self.on_close()
            },
            WindowEvent::Focused(focused) => match focused {
                true => self.on_resumed(), 
                false => self.on_paused(), 
            }, 
            WindowEvent::KeyboardInput { event, .. } => {
                self.on_keyboard_event(event, &window)
            },
            WindowEvent::RedrawRequested => {
                self.on_draw(&window, &surface)
            },
            WindowEvent::Resized(_) => {
                self.on_resized(&window, &surface)
            }, 
            WindowEvent::ScaleFactorChanged { .. } => {
                self.on_resized(&window, &surface)
            }, 
            _ => return, 
        };
        if let Err(e) = result {
            alert_error("Runtime error", e.to_string(), Some(&window));
            drop(self.window.take());
            drop(self.surface.take());
        }
    }
}

impl AppHandle for Application {
    #[inline]
    #[must_use]
    fn num_threads(&self) -> usize {
        self.num_threads
    }

    #[inline]
    #[must_use]
    fn current_dir(&self) -> &std::path::Path {
        &self.current_dir
    }

    #[inline]
    #[must_use]
    fn flags(&self) -> AppFlags {
        self.flags.clone()
    }

    #[inline]
    #[must_use]
    fn locale(&self) -> Option<AppLocale> {
        self.locale.clone()
    }

    #[inline]
    #[must_use]
    fn timer(&self) -> &GameTimer {
        &self.timer
    }

    #[inline]
    #[must_use]
    fn render_instance(&self) -> &Arc<wgpu::Instance> {
        &self.instance
    }

    #[inline]
    #[must_use]
    fn render_adapter(&self) -> &Arc<wgpu::Adapter> {
        &self.adapter
    }

    #[inline]
    #[must_use]
    fn render_device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    #[inline]
    #[must_use]
    fn render_queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    #[inline]
    #[must_use]
    fn render_surface(&self) -> Option<&Arc<wgpu::Surface<'static>>> {
        self.surface.as_ref()
    }

    #[inline]
    #[must_use]
    fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }
}
