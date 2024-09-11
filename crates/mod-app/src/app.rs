use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::Once;

use hecs::World;
use mod_error::alert_error;
use mod_error::err_msg;
use mod_error::RuntimeError;
use mod_render::config_swapchain;
use mod_render::create_surface;
use mod_render::init_wgpu;
use mod_render::DEPTH_STENCIL_FORMAT;
use mod_scene::AppHandle;
use mod_scene::GameScene;
use mod_scene::GameSceneFlow;
use mod_scene::GameSceneStack;
use mod_util::AppDpi;
use mod_util::AppEvent;
use mod_util::AppFlags;
use mod_util::AppLocale;
use mod_util::GameTimer;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::ElementState;
use winit::event::KeyEvent;
use winit::event::Modifiers;
use winit::event::MouseButton;
use winit::event::MouseScrollDelta;
use winit::event::StartCause;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoopProxy;
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
    /// 사용자 정의 이벤트를 이벤트 루프로 보내는 프록시입니다.
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>, 


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

    /// 키보드 수정자 상태정보입니다.
    modifier: Modifiers, 

    /// 이전 커서의 위치입니다.
    prev_cursor_pos: PhysicalPosition<f64>, 


    /// 게임 장면 흐름입니다.
    /// 게임 장면 스택을 제어합니다.
    scene_flow: RefCell<Option<GameSceneFlow>>, 

    /// 게임 장면 스택입니다.
    /// 생성된 게임 장면을 관리합니다.
    scene_stack: RefCell<GameSceneStack>, 


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

    /// 생성된 깊이-스텐실 버퍼입니다.
    depth_stencil_view: RefCell<Option<wgpu::TextureView>>, 
}

impl Application {
    /// 애플리케이션을 생성합니다.
    pub(crate) async fn new(
        proxy: Arc<EventLoopProxy<AppEvent>>, 
        builder: AppBuilder
    ) -> Result<Self, Box<dyn Error + Send>> {
        let enable_debug_layer = builder.flags.contains(AppFlags::ENABLE_DEBUG_LAYER);
        let (instance, adapter, device, queue) = init_wgpu(enable_debug_layer).await?;

        Ok(Self {
            event_loop_proxy: proxy, 
            num_threads: builder.num_threads, 
            current_dir: builder.current_dir.unwrap(), 
            flags: builder.flags, 
            locale: None, 
            title: builder.title.unwrap_or("Hello, World!".to_string()), 
            icon: builder.icon, 
            dpi: builder.dpi.unwrap_or(AppDpi::MAX), 
            fullscreen: builder.fullscreen, 
            modifier: Modifiers::default(), 
            prev_cursor_pos: PhysicalPosition::default(), 
            scene_flow: RefCell::new(Some(GameSceneFlow::Reset(builder.start_scene))), 
            scene_stack: RefCell::new(GameSceneStack::new()), 
            world: RefCell::new(World::new()), 
            timer: GameTimer::default(), 
            total_time_sec: 0.0, 
            instance, 
            adapter, 
            device, 
            queue, 
            window: None, 
            surface: None, 
            depth_stencil_view: RefCell::new(None), 
        })
    }



    /// 새로운 애플리케이션 창을 생성합니다.
    #[must_use]
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Arc<Window>, Box<dyn Error + Send>> {
        // 시스템에서 사용 가능한 최대 해상도를 가져옵니다.
        let max_dpi = match AppDpi::find_maximize_dpi(event_loop) {
            Some(max_dpi) => max_dpi, 
            None => return Err(err_msg!(AppError::NoSuitableResolution)), 
        };
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

        let window = match event_loop.create_window(attributes) {
            Ok(window) => window.into(), 
            Err(e) => return Err(err_msg!(AppError::System(e.to_string()))), 
        };

        return Ok(window);
    }



    /// 애플리케이션 창의 크기가 변경되었을 때 호출되는 함수입니다.
    fn on_resized(
        &self, 
        width: u32, 
        height: u32, 
        window: &Window, 
        surface: &wgpu::Surface<'static>, 
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 애플리케이션 창의 가로 또는 세로 크기가 0인 경우 함수 실행을 생략합니다.
        if width == 0 || height == 0 {
            return Ok(());
        }

        // 이전에 제출한 모든 렌더링 작업이 끝날 때 까지 대기합니다.
        self.instance.poll_all(true);

        // 변경된 크기로 스왑체인을 재설정합니다.
        let disable_vsync = self.flags.contains(AppFlags::DISABLE_VSYNC);
        config_swapchain(width, height, &self.device, surface, disable_vsync);

        // 깊이 버퍼의 크기를 재조정합니다.
        let mut depth_stencil_view = self.depth_stencil_view.borrow_mut();
        *depth_stencil_view = Some(self.device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Depth-Stencil"), 
                dimension: wgpu::TextureDimension::D2, 
                format: DEPTH_STENCIL_FORMAT, 
                mip_level_count: 1, 
                sample_count: 1, 
                size: wgpu::Extent3d {
                    width, 
                    height, 
                    depth_or_array_layers: 1, 
                }, 
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
                view_formats: &[]
            }
        ).create_view(&wgpu::TextureViewDescriptor { ..Default::default() }));

        // 현재 게임 장면의 콜백함수를 호출합니다.
        let mut world = self.world.borrow_mut();
        current_scene.on_resized(window, &mut world, self)?;

        Ok(())
    }

    /// 애플리케이션 창이 이동되었을 때 호출되는 함수입니다.
    fn on_moved(
        &self, 
        x: i32, 
        y: i32, 
        window: &Window,
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 현재 게임 장면의 콜백함수를 호출합니다.
        let mut world = self.world.borrow_mut();
        current_scene.on_moved(x, y, window, &mut world, self)?;
        Ok(())
    }

    /// 애플리케이션 창의 초점 변화 이벤트가 발생했을 때 호출되는 함수입니다.
    fn on_focused(
        &self, 
        focused: bool, 
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 장면의 콜백함수를 호출합니다.
        let mut world = self.world.borrow_mut();
        if focused {
            current_scene.on_resume(&mut world, self)
        } else {
            current_scene.on_pause(&mut world, self)
        }
    }

    /// 애플리케이션 창의 키보드 입력 이벤트가 발생했을 때 호출되는 함수입니다.
    fn on_keyboard_input(
        &self, 
        event: KeyEvent, 
        window: &Window,
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 반복된 이벤트의 경우 함수 실행을 생략합니다.
        if event.repeat {
            return Ok(());
        }

        // 게임 장면의 콜백함수를 호출합니다.
        let mut world = self.world.borrow_mut();
        if let PhysicalKey::Code(keycode) = event.physical_key {
            if event.state.is_pressed() {
                current_scene.on_keyboard_pressed(
                    keycode, 
                    event.location, 
                    self.modifier, 
                    window, 
                    &mut world, 
                    self
                )
            } else {
                current_scene.on_keyboard_released(
                    keycode, 
                    event.location, 
                    self.modifier, 
                    window, 
                    &mut world, 
                    self
                )
            }
        } else {
            Ok(())
        }
    }

    /// 애플리케이션 창의 커서 이동 이벤트가 발생했을 때 호출되는 함수입니다.
    fn on_cursor_moved(
        &self, 
        x: f32, 
        y: f32, 
        delta_x: f32, 
        delta_y: f32, 
        window: &Window,
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 장면의 콜백함수를 호출합니다.
        let mut world = self.world.borrow_mut();
        current_scene.on_cursor_moved(
            x, 
            y, 
            delta_x, 
            delta_y, 
            window, 
            &mut world, 
            self
        )
    }

    /// 애플리케이션 창의 마우스 휠 이벤트가 발생했을 때 호출되는 함수입니다.
    fn on_mouse_wheel(
        &self, 
        delta: MouseScrollDelta, 
        window: &Window,
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        if let MouseScrollDelta::LineDelta(delta_x, delta_y) = delta {
            // 게임 장면의 콜백함수를 호출합니다.
            let mut world = self.world.borrow_mut();
            current_scene.on_mouse_wheel(
                delta_x, 
                delta_y, 
                window, 
                &mut world, 
                self
            )
        } else {
            Ok(())
        }
    }

    /// 애플리케이션 창의 마우스 버튼 입력 이벤트가 발생했을 때 호출되는 함수입니다.
    fn on_mouse_input(
        &self, 
        state: ElementState, 
        button: MouseButton, 
        window: &Window,
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 장면의 콜백함수를 호출합니다.
        let (x, y): (f64, f64) = self.prev_cursor_pos.into();
        let mut world = self.world.borrow_mut();
        if state.is_pressed() {
            current_scene.on_mouse_pressed(
                x as f32, 
                y as f32, 
                button, 
                window, 
                &mut world, 
                self
            )
        } else {
            current_scene.on_mouse_released(
                x as f32, 
                y as f32, 
                button, 
                window, 
                &mut world, 
                self
            )
        }
    }

    /// 애플리케이션 창이 그려질 때 호출되는 함수입니다.
    fn on_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface<'static>, 
        current_scene: &mut Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 이전 렌더링 작업이 끝났는지 확인합니다. (blocking)
        self.device.poll(wgpu::Maintain::Wait);

        // 현재 프레임버퍼를 가져옵니다.
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame, 
            Err(e) => return Err(err_msg!(e)), 
        };

        // 렌더 타겟 뷰를 가져옵니다.
        let render_target_view = frame.texture.create_view(
            &wgpu::TextureViewDescriptor { ..Default::default() }
        );

        // 깊이-스탠실 뷰를 가져옵니다.
        let depth_stencil_view = self.depth_stencil_view.borrow();
        let depth_stencil_view = depth_stencil_view.as_ref().unwrap();

        // 현재 게임 장면의 그리기 준비 함수를 호출합니다.
        let mut world = self.world.borrow_mut();
        current_scene.on_prepare_draw(window, surface, &mut world, self)?;

        // 현재 게임 장면의 그리기 함수를 호출합니다.
        current_scene.on_draw(&render_target_view, &depth_stencil_view, &mut world, self)?;

        // `winit` API에 애플리케이션 창을 갱신한다고 알립니다.
        window.pre_present_notify();

        // 프레임버퍼를 출력합니다.
        frame.present();

        Ok(())
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

        // 깊이-스탠실 버퍼를 생성합니다.
        let mut depth_stencil_view = self.depth_stencil_view.borrow_mut();
        *depth_stencil_view = Some(self.device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Depth-Stencil"), 
                dimension: wgpu::TextureDimension::D2, 
                format: DEPTH_STENCIL_FORMAT, 
                mip_level_count: 1, 
                sample_count: 1, 
                size: wgpu::Extent3d {
                    width, 
                    height, 
                    depth_or_array_layers: 1,
                }, 
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
                view_formats: &[]
            }
        ).create_view(&wgpu::TextureViewDescriptor { ..Default::default() }));

        self.window = Some(window);
        self.surface = Some(surface);
    }



    fn suspended(&mut self, _: &ActiveEventLoop) {
        drop(self.window.take());
        drop(self.surface.take());
        let mut depth_stencil_view = self.depth_stencil_view.borrow_mut();
        *depth_stencil_view = None;
    }



    fn exiting(&mut self, _: &ActiveEventLoop) {
        // 게임 장면 스택을 정리합니다.
        let mut world = self.world.borrow_mut();
        let mut scene_stack = self.scene_stack.borrow_mut();
        let result = scene_stack.clear(self.window.as_deref(), &mut world, self);
        if let Err(e) = result {
            alert_error("Runtime error", e.to_string(), self.window.as_deref());
            process::exit(-1);
        }
    }



    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.clone() {
            // 게임 장면 스택을 갱신합니다.
            let mut world = self.world.borrow_mut();
            let mut scene_stack = self.scene_stack.borrow_mut();
            {
                let mut scene_flow = self.scene_flow.borrow_mut();
                let result = scene_stack.flush(&mut scene_flow, &window, &mut world, self);
                if let Err(e) = result {
                    alert_error("Runtime error", e.to_string(), Some(&window));
                    return event_loop.exit();
                }
            }

            // 현제 게임 장면을 가져옵니다.
            // 현제 게임 장면이 존재하지 않는 경우 애플리케이션을 종료합니다.
            let current_scene = match scene_stack.top_mut() {
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
                log::warn!("성능 저하로 인해 고정 시간 갱신 횟수를 초과함!");
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

        // 현재 게임 장면을 가져옵니다.
        // 현재 게임 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        let current_scene = match scene_stack.top_mut() {
            Some(current_scene) => current_scene, 
            None => return, 
        };

        // 윈도우 이벤트를 처리합니다.
        let result = match event {
            WindowEvent::Resized(size) => {
                self.on_resized(size.width, size.height, &window, &surface, current_scene)
            }, 
            WindowEvent::Moved(position) => {
                self.on_moved(position.x, position.y, &window, current_scene)
            }, 
            WindowEvent::CloseRequested => {
                if current_scene.on_close_request(self) {
                    drop(self.window.take());
                    drop(self.surface.take());
                }
                Ok(())
            }, 
            WindowEvent::Focused(focused) => {
                self.on_focused(focused, current_scene)
            }, 
            WindowEvent::KeyboardInput { event, .. } => {
                self.on_keyboard_input(event, &window, current_scene)
            }, 
            WindowEvent::CursorMoved { position, .. } => {
                static INIT: Once = Once::new();
                INIT.call_once(|| {
                    self.prev_cursor_pos = position;
                });
        
                // 커서 위치의 변동 거리를 계산합니다.
                let delta_x = position.x - self.prev_cursor_pos.x;
                let delta_y = position.y - self.prev_cursor_pos.y;
                self.prev_cursor_pos = position;
        
                self.on_cursor_moved(
                    position.x as f32, 
                    position.y as f32, 
                    delta_x as f32, 
                    delta_y as f32, 
                    &window, 
                    current_scene
                )
            }, 
            WindowEvent::MouseWheel { delta, .. } => {
                self.on_mouse_wheel(delta, &window, current_scene)
            }, 
            WindowEvent::MouseInput { state, button, .. } => {
                self.on_mouse_input(state, button, &window, current_scene)
            }, 
            WindowEvent::ScaleFactorChanged { .. } => {
                let (width, height): (u32, u32) = window.inner_size().into();
                self.on_resized(width, height, &window, &surface, current_scene)
            }, 
            WindowEvent::RedrawRequested => {
                self.on_draw(&window, &surface, current_scene)
            },
            _ => Ok(())
        };

        if let Err(e) = result {
            alert_error("Runtime error", e.to_string(), Some(&window));
            drop(self.window.take());
            drop(self.surface.take());
        }
    }



    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        // 현재 게임 장면을 가져옵니다.
        // 현재 게임 장면이 비어있는 경우 함수 실행을 생략합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        let current_scene = match scene_stack.top_mut() {
            Some(current_scene) => current_scene, 
            None => return, 
        };

        let result = match event {
            AppEvent::PacketReceived(raw_packet) => {
                let mut world = self.world.borrow_mut();
                current_scene.on_received_packet(raw_packet, &mut world, self)
            }, 
            AppEvent::ClosedConnection => {
                // 현재는 네트워크 연결 끊김 에러를 애플리케이션 종료 처리합니다.
                return event_loop.exit();
            },
            AppEvent::NetworkIOError(e) => {
                // 현재는 모든 입/출력 에러를 런타임 에러로 처리합니다.
                Err(err_msg!(e))
            }, 
        };

        if let Err(e) = result {
            alert_error("Runtime error", e.to_string(), self.window.as_deref());
            event_loop.exit();
        }
    }
}

impl AppHandle for Application {
    #[inline]
    #[must_use]
    fn event_loop_proxy(&self) -> &Arc<EventLoopProxy<AppEvent>> {
        &self.event_loop_proxy
    }

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
    
    #[inline]
    fn set_scene_flow(&self, flow: GameSceneFlow) {
        let mut scene_flow = self.scene_flow.borrow_mut();
        *scene_flow = Some(flow);
    }
}
