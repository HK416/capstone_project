use std::{
    cell::RefCell, 
    collections::VecDeque, 
    error::Error, 
    net::SocketAddr, 
    path::{Path, PathBuf}, 
    sync::{Arc, Once}
};

use mod_world::render::{
    config_swapchain, 
    create_surface, 
    init_wgpu, 
    RenderError, 
    DEPTH_STENCIL_FORMAT
};
use winit::{
    application::ApplicationHandler, 
    dpi::PhysicalPosition, 
    event::{Modifiers, MouseScrollDelta, StartCause, WindowEvent}, 
    event_loop::{ActiveEventLoop, EventLoopProxy}, 
    keyboard::PhysicalKey, 
    window::{Fullscreen, Icon, Window, WindowButtons}
};

use crate::{
    etc::{AppEvent, AppFlags, GameTimer, Locale, WindowSize}, 
    exception::alert_error, 
    scene::{GameScene, GameSceneFlow}
};

use super::{
    builder::AppBuilder, 
    AppError, 
    AppHandle
};

/// 고정 시간 갱신에 사용되는 경과 시간입니다.
pub const FIXED_TIME_SEC: f32 = 1.0 / 60.0;

/// 최대 고정 시간 갱신 횟수입니다.
pub const MAX_FIXED_UPDATE: usize = 8;




/// 애플리케이션 관리 구조체입니다.
pub struct Application {
    /// 사용자 정의 이벤트를 이벤트 루프로 보내는 프록시입니다.
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>, 

    /// 애플리케이션에서 사용 가능한 최대 스레드의 수입니다.
    num_threads: usize, 

    /// 애플리케이션 실행 디렉토리 경로입니다.
    current_dir: PathBuf, 

    /// 애플리케이션 서버 주소입니다.
    address: SocketAddr, 

    /// 애플리케이션 플래그 옵션입니다.
    flags: AppFlags, 

    /// 현재 애플리케이션 표시 언어입니다.
    locale: Option<Locale>, 

    /// 애플리케이션 창 제목 텍스트입니다.
    window_title: String, 

    /// 애플리케이션 창 아이콘입니다.
    window_icon: Option<Icon>, 

    /// 애플리케이션 창 크기입니다.
    window_size: WindowSize, 

    /// 애플리케이션 창의 전체화면 여부입니다.
    fullscreen: bool, 

    /// 애플리케이션 키보드 수정자 상태정보입니다.
    modifier: Modifiers, 

    /// 커서의 이전 위치입니다.
    prev_cursor_pos: PhysicalPosition<f64>, 

    /// 게임 장면 스택을 제어하는 게임 장면 흐름입니다.
    scene_flow: Option<GameSceneFlow>, 

    /// 생성된 게임 장면을 관리하는 게임 장면 스택입니다.
    scene_stack: RefCell<VecDeque<Box<dyn GameScene>>>, 

    /// 업데이트 경과 시간을 측정하는 타이머입니다.
    timer: GameTimer, 

    /// 고정 시간 갱신에 사용되는 축적된 시간입니다.
    accum_time: f32, 

    /// `wgpu` 렌더링 인스턴스입니다.
    instance: Arc<wgpu::Instance>, 

    /// `wgpu` 렌더링 장치 어댑터입니다.
    adapter: Arc<wgpu::Adapter>, 

    /// `wgpu` 렌더링 논리적 장치입니다.
    device: Arc<wgpu::Device>, 

    /// `wgpu` 렌더링 장치 명령 대기열입니다.
    queue: Arc<wgpu::Queue>, 

    /// 애플리케이션 창 입니다.
    window: Option<Arc<Window>>, 

    /// `wgpu` 렌더링 장치 표면입니다.
    surface: Option<Arc<wgpu::Surface<'static>>>, 

    /// 깊이 버퍼입니다.
    depth_buffer_view: Option<wgpu::TextureView>,
}

impl Application {
    /// 애플리케이션을 생성합니다.
    /// 
    /// # Errors
    /// 애플리케이션 렌더러를 생성하는 도중 오류가 발생한 경우 `RenderError`를 반환합니다.
    /// 
    pub(crate) async fn new(
        event_loop_proxy: Arc<EventLoopProxy<AppEvent>>, 
        builder: AppBuilder
    ) -> Result<Self, RenderError> {
        // wgpu 렌더러를 생성합니다.
        let enable_debug_layer = builder.flags.contains(AppFlags::ENABLE_DEBUG_LAYER);
        let (instance, adapter, device, queue) = init_wgpu(enable_debug_layer).await?;

        Ok(Self {
            event_loop_proxy, 
            num_threads: builder.num_threads, 
            current_dir: unsafe { builder.current_dir.unwrap_unchecked() }, // Safe: 빌더 생성 중 확인함.
            address: builder.address, 
            flags: builder.flags, 
            locale: None, 
            window_title: builder.title.unwrap_or("Hello to Halo".to_string()), 
            window_icon: builder.icon, 
            window_size: builder.size.unwrap_or(WindowSize::MAX), 
            fullscreen: builder.fullscreen, 
            modifier: Modifiers::default(), 
            prev_cursor_pos: PhysicalPosition::default(), 
            scene_flow: Some(GameSceneFlow::Reset(builder.start_scene)), 
            scene_stack: RefCell::new(VecDeque::with_capacity(8)), 
            timer: GameTimer::start(), 
            accum_time: 0.0, 
            instance, 
            adapter, 
            device, 
            queue, 
            window: None, 
            surface: None, 
            depth_buffer_view: None, 
        })
    }

    /// 새로운 애플리케이션 창을 생성합니다.
    #[must_use]
    fn create_window(
        &mut self, 
        event_loop: &ActiveEventLoop
    ) -> Result<Arc<Window>, AppError> {
        // 시스템에서 사용 가능한 창의 최대 크기를 가져옵니다.
        let max_window_size = match WindowSize::find_maximize_size(event_loop) {
            Some(size) => size, 
            None => return Err(AppError::NoSuitableResolution)
        };

        // 창의 최대 크기를 조정합니다.
        self.window_size = self.window_size.min(max_window_size);

        // 애플리케이션 창을 생성합니다.
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes()
            .with_title(self.window_title.as_str())
            .with_window_icon(self.window_icon.clone())
            .with_inner_size(self.window_size.size())
            .with_fullscreen(self.fullscreen.then_some(Fullscreen::Borderless(None)))
            .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
            .with_resizable(false)
            .with_visible(true)
            .with_active(true);

        #[cfg(target_os = "macos")] {
            use winit::platform::macos::WindowAttributesExtMacOS;
            attributes = attributes.with_movable_by_window_background(true);
        }

        #[cfg(target_os = "windows")] {
            use winit::platform::windows::WindowAttributesExtWindows;
            use winit::platform::windows::CornerPreference;
            attributes = attributes.with_corner_preference(CornerPreference::DoNotRound);
        }

        let window = match event_loop.create_window(attributes) {
            Ok(window) => window.into(), 
            Err(e) => return Err(AppError::from(e)),
        };

        Ok(window)
    }

    fn draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface<'static>, 
        curr_scene: &Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        // 현재 게임 장면의 그리기 준비 콜백 함수를 호출합니다.
        curr_scene.on_prepare_draw(&window, &surface, self)?;
        
        // 이전 렌더링 작업이 끝날때 까지 대기합니다.
        self.device.poll(wgpu::Maintain::Wait);

        // 현재 프레임 버퍼를 가져옵니다.
        let frame = surface.get_current_texture()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // 렌더 타겟 뷰를 가져옵니다.
        let render_target_view = frame.texture.create_view(
            &wgpu::TextureViewDescriptor {
                ..Default::default()
            }
        );

        // 깊이 - 스텐실 뷰를 가져옵니다.
        let depth_stencil_view = self.depth_buffer_view.as_ref().unwrap();

        // `winit` API에 애플리케이션 창을 갱신한다고 알립니다.
        window.pre_present_notify();

        // 현재 게임 장면에 그리기 콜백 함수를 호출합니다.
        curr_scene.on_draw(&render_target_view, depth_stencil_view, self)?;

        // 프레임 버퍼를 출력합니다.
        frame.present();

        Ok(())
    }
}





impl ApplicationHandler<AppEvent> for Application {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.timer.tick();
    }

    /// `winit` API는 애플리케이션이 생성되었을 때 `ApplicationHandler::resumed`를 호출합니다. </br>
    /// 또한 일부 시스템(예: `Android`)은 애플리케이션 초기화 이전에 창을 생성하는 것이 허용되지 않습니다. </br>
    /// 따라서 이 콜백 함수에서 애플리케이션 창을 생성하고, 렌더러 표면을 생성해야 합니다. </br>
    ///
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // 애플리케이션 창을 생성합니다.
        let window = match self.create_window(event_loop) {
            Ok(window) => window, 
            Err(e) => {
                alert_error("Application window creation failed", e.to_string(), None);
                return event_loop.exit();
            }
        };

        // `wgpu` 장치 표면을 생성합니다.
        let surface = match create_surface(window.clone(), &self.instance, &self.adapter) {
            Ok(surface) => surface, 
            Err(e) => {
                alert_error("Render surface creation failed", e.to_string(), Some(&window));
                return event_loop.exit();
            }
        };

        // 애플리케이션 창의 가로와 세로 크기를 가져옵니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        if width != 0 && height != 0 {
            // 변경된 크기로 스왑체인을 재설정합니다.
            let disable_vsync = self.flags.contains(AppFlags::DISABLE_VSYNC);
            config_swapchain(width, height, &self.device, &surface, disable_vsync);

            // 변경된 크기로 깊이 버퍼를 재설정합니다.
            self.depth_buffer_view = create_depth_buffer(width, height, &self.device).into();
        }

        // 생성한 윈도우와 `wgpu` 장치 표면을 설정합니다.
        self.window = window.into();
        self.surface = surface.into();
    }

    /// 일반적으로 `Android` 시스템이 아닌경우 이 함수는 호출되지 않습니다.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        drop(self.window.take());
        drop(self.surface.take());
        drop(self.depth_buffer_view.take());
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // 게임 장면 스택을 정리합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        if let Err(e) = clear_scene(&mut scene_stack, self.window.as_deref(), self) {
            alert_error("Runtime error", e.to_string(), self.window.as_deref());
            std::process::exit(-1);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 애플리케이션 창을 가져옵니다. 
        // 애플리케이션 창이 존재하지 않는 경우 애플리케이션을 종료합니다.
        let window = match self.window.clone() {
            Some(window) => window, 
            None => return event_loop.exit(), 
        };

        // 게임 장면 스택을 갱신합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        if let Some(flow) = self.scene_flow.take() {
            if let Err(e) = match flow {
                GameSceneFlow::Clear => clear_scene(&mut scene_stack, Some(&window), self), 
                GameSceneFlow::Reset(new_scene) => reset_scene(&mut scene_stack, &window, self, new_scene), 
                GameSceneFlow::Change(new_scene) => change_scene(&mut scene_stack, &window, self, new_scene), 
                GameSceneFlow::Push(new_scene) => push_scene(&mut scene_stack, &window, self, new_scene), 
                GameSceneFlow::Pop => pop_scene(&mut scene_stack, &window, self)
            } {
                alert_error("Runtime error", e.to_string(), Some(&window));
                return event_loop.exit();
            }
        }

        // 현재 게임 장면을 가져옵니다.
        // 현재 게임 장면이 존재하지 않는 경우 애플리케이션을 종료합니다.
        let curr_scene = match scene_stack.back_mut() {
            Some(curr_scene) => curr_scene, 
            None => return event_loop.exit(),
        };

        // 총 경과 시간을 갱신합니다.
        let elapsed_time_sec = self.timer.elapsed_time_sec();
        self.accum_time += elapsed_time_sec;

        // 변동 시간 갱신 함수를 호출합니다.
        if let Err(e) = curr_scene.on_update(elapsed_time_sec, &window, self) {
            alert_error("Runtime error", e.to_string(), Some(&window));
            return event_loop.exit();
        }

        // 고정 시간 갱신 함수를 호출합니다.
        let mut count = 0;
        while count < MAX_FIXED_UPDATE 
        && FIXED_TIME_SEC <= self.accum_time {
            if let Err(e) = curr_scene.on_fixed_update(FIXED_TIME_SEC, &window, self) {
                alert_error("Runtime error", e.to_string(), Some(&window));
                return event_loop.exit();
            }
            self.accum_time -= FIXED_TIME_SEC;
            count += 1;
        }

        // 최대 갱신 횟수를 초과할 경우 변동 시간 간격을 전달합니다.
        if count >= MAX_FIXED_UPDATE {
            log::warn!("성능 저하로 인해 고정 시간 갱신 횟수를 초과했습니다!");
            if let Err(e) = curr_scene.on_fixed_update(self.accum_time, &window, self) {
                alert_error("Runtime error", e.to_string(), Some(&window));
                return event_loop.exit();
            }
            self.accum_time = 0.0;
        }

        // 애플리케이션 창을 갱신합니다.
        window.request_redraw();
    }

    /// NOTE: 이 함수에서 이벤트 루프의 종료 함수를 호출할 경우 `panic!`이 발생합니다.
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // 애플리케이션 창과 렌더링 장치 표면을 가져옵니다.
        // 애플리케이션 창 또는 렌더링 장치 표면이 없는 경우 함수 실행을 생략합니다.
        let window = self.window.clone();
        let surface = self.surface.clone();
        let (window, surface) = match window.zip(surface) {
            Some(tuple) => tuple, 
            None => return,
        };

        // 애플리케이션 창 식별자가 다른 경우 함수 실행을 생략합니다.
        if window_id != window.id() {
            return;
        }

        // 현재 게임 장면을 가져옵니다.
        // 현재 게임 장면이 존재하지 않는 경우 함수 실행을 생략합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        let curr_scene = match scene_stack.back_mut() {
            Some(curr_scene) => curr_scene, 
            None => return,
        };

        // 윈도우 이벤트를 처리합니다.
        if let Err(e) = match event {
            WindowEvent::Resized(_) => {
                // 애플리케이션 창의 가로와 세로 크기를 가져옵니다.
                // 가로 또는 세로의 크기가 0인 경우 함수 실행을 중단합니다.
                let (width, height): (u32, u32) = window.inner_size().into();
                if width == 0 || height == 0 {
                    return;
                }

                // 이전에 제출한 모든 렌더링 작업이 끝날 때 까지 대기합니다.
                self.instance.poll_all(true);

                // 변경된 크기로 스왑체인을 재설정합니다.
                let disable_vsync = self.flags.contains(AppFlags::DISABLE_VSYNC);
                config_swapchain(width, height, &self.device, &surface, disable_vsync);

                // 변경된 크기로 깊이 버퍼를 재설정합니다.
                self.depth_buffer_view = create_depth_buffer(width, height, &self.device).into();

                curr_scene.on_window_resized(&window, self)
            }, 
            WindowEvent::Moved(_) => {
                curr_scene.on_window_moved(&window, self)
            }, 
            WindowEvent::CloseRequested => {
                if curr_scene.on_close_request(self) {
                    drop(self.window.take());
                    drop(self.surface.take());
                };
                Ok(())
            },
            WindowEvent::Focused(focused) => {
                if focused {
                    curr_scene.on_resumed(self)
                } else {
                    curr_scene.on_paused(self)
                }
            }, 
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state.is_pressed() {
                        curr_scene.on_keyboard_pressed(
                            code, 
                            event.location, 
                            self.modifier, 
                            event.repeat, 
                            &window, 
                            self
                        )
                    } else {
                        curr_scene.on_keyboard_released(
                            code, 
                            event.location, 
                            self.modifier, 
                            event.repeat, 
                            &window, 
                            self
                        )
                    }
                } else {
                    Ok(())
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                static INIT: Once = Once::new();
                INIT.call_once(|| {
                    self.prev_cursor_pos = position;
                });

                let dx = position.x - self.prev_cursor_pos.x;
                let dy = position.y - self.prev_cursor_pos.y;
                self.prev_cursor_pos = position;

                curr_scene.on_cursor_moved(
                    position.x as f32, 
                    position.y as f32, 
                    dx as f32, 
                    dy as f32, 
                    &window, 
                    self
                )
            }, 
            WindowEvent::MouseWheel { delta, .. } => {
                if let MouseScrollDelta::LineDelta(dx, dy) = delta {
                    curr_scene.on_mouse_wheel(dx, dy, &window, self)
                } else {
                    return;
                }
            }, 
            WindowEvent::MouseInput { state, button, .. } => {
                let (x, y): (f64, f64) = self.prev_cursor_pos.into();
                if state.is_pressed() {
                    curr_scene.on_mouse_btn_pressed(x as f32, y as f32, button, &window, self)
                } else {
                    curr_scene.on_mouse_btn_released(x as f32, y as f32, button, &window, self)
                }
            }, 
            WindowEvent::ScaleFactorChanged { .. } => {
                // 애플리케이션 창의 가로와 세로 크기를 가져옵니다.
                // 가로 또는 세로의 크기가 0인 경우 함수 실행을 중단합니다.
                let (width, height): (u32, u32) = window.inner_size().into();
                if width == 0 || height == 0 {
                    return;
                }

                // 이전에 제출한 모든 렌더링 작업이 끝날 때 까지 대기합니다.
                self.instance.poll_all(true);

                // 변경된 크기로 스왑체인을 재설정합니다.
                let disable_vsync = self.flags.contains(AppFlags::DISABLE_VSYNC);
                config_swapchain(width, height, &self.device, &surface, disable_vsync);

                // 변경된 크기로 깊이 버퍼를 재설정합니다.
                self.depth_buffer_view = create_depth_buffer(width, height, &self.device).into();

                curr_scene.on_window_resized(&window, self)
            }, 
            WindowEvent::ModifiersChanged(modifier) => {
                self.modifier = modifier;
                return;
            }, 
            WindowEvent::RedrawRequested => {
                self.draw(&window, &surface, curr_scene)
            }, 
            _ => { return }
        } {
            alert_error("Runtime error", e.to_string(), Some(&window));
            drop(self.window.take());
            drop(self.surface.take());
            drop(self.depth_buffer_view.take());
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        // 현재 게임 장면을 가져옵니다.
        // 현재 게임 장면이 존재하지 않는 경우 함수 실행을 생략합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        let curr_scene = match scene_stack.back_mut() {
            Some(curr_scene) => curr_scene, 
            None => return,
        };

        match event {
            AppEvent::SetGameSceneFlow(flow) => {
                self.scene_flow = flow.into();
                return;
            }, 
            AppEvent::ClosedSocket => {
                // FIXME: 현재는 애플리케이션을 종료시킵니다.
                alert_error("Runtime error", "서버와 연결이 끊어졌습니다.", self.window.as_deref());
                return event_loop.exit();
            }, 
            AppEvent::NetworkIOError(e) => {
                alert_error("Runtime error", e.to_string(), self.window.as_deref());
                return event_loop.exit();
            }, 
            AppEvent::PacketReceived(packet) => {
                log::debug!("received packet: {:?}", packet);
                if let Err(e) = curr_scene.on_received_packet(packet, self) {
                    alert_error("Runtime error", e.to_string(), self.window.as_deref());
                    return event_loop.exit();
                }
            }
        };
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
    fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    fn address(&self) -> &SocketAddr {
        &self.address
    }

    #[inline]
    #[must_use]
    fn flags(&self) -> AppFlags {
        self.flags
    }

    #[inline]
    #[must_use]
    fn locale(&self) -> Option<Locale> {
        self.locale
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
    fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }

    #[inline]
    #[must_use]
    fn render_surface(&self) -> Option<&Arc<wgpu::Surface<'static>>> {
        self.surface.as_ref()
    }
}





/// 주어진 크기의 깊이 버퍼를 생성합니다.
fn create_depth_buffer(
    width: u32, 
    height: u32, 
    device: &wgpu::Device
) -> wgpu::TextureView {
    debug_assert!(width != 0 && height != 0);
    device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some("Depth-Buffer"), 
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
    ).create_view(
        &wgpu::TextureViewDescriptor { 
            ..Default::default()
        }
    )
}





/// 모든 게임 장면을 제거합니다.
fn clear_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>, 
    window: Option<&Window>, 
    app: &dyn AppHandle
) -> Result<(), Box<dyn Error + Send>> {
    while let Some(mut scene) = stack.pop_back() {
        scene.on_exit(window, app)?;
    }
    Ok(())
}

/// 모든 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
fn reset_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>, 
    window: &Window, 
    app: &dyn AppHandle, 
    new_scene: Box<dyn GameScene>
) -> Result<(), Box<dyn Error + Send>> {
    clear_scene(stack, Some(window), app)?;
    push_scene(stack, window, app, new_scene)?;
    Ok(())
}

/// 현재 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
fn change_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>, 
    window: &Window, 
    app: &dyn AppHandle, 
    new_scene: Box<dyn GameScene>
) -> Result<(), Box<dyn Error + Send>> {
    pop_scene(stack, window, app)?;
    push_scene(stack, window, app, new_scene)?;
    Ok(())
}

/// 새로운 게임 장면을 초기화하고, 추가합니다.
fn push_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>, 
    window: &Window, 
    app: &dyn AppHandle, 
    mut new_scene: Box<dyn GameScene>
) -> Result<(), Box<dyn Error + Send>> {
    new_scene.on_enter(window, app)?;
    stack.push_back(new_scene);
    Ok(())
}

/// 현재 장면을 정리하고, 제거합니다.
fn pop_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>, 
    window: &Window, 
    app: &dyn AppHandle
) -> Result<(), Box<dyn Error + Send>> {
    if let Some(mut scene) = stack.pop_back() {
        scene.on_exit(Some(window), app)?;
    }
    Ok(())
}
