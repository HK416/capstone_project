use std::{
    cell::{RefCell, RefMut},
    collections::VecDeque,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};

use mod_render::{config_swapchain, init_wgpu, ScreenDescriptor, UiRenderer, SWAPCHAIN_FORMAT};
use rayon::{ThreadPool, ThreadPoolBuilder};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{DeviceEvent, Modifiers, MouseScrollDelta, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::{Fullscreen, Icon, Window, WindowButtons},
};

use crate::{
    asset::AssetManager,
    error::{show_error_msg, Alert},
    etc::{AppEvent, AppFlags, GameTimer, WindowSize},
    ext::AppWindowExt,
    net::{NetManager, NetworkError},
    scene::{GameScene, GameSceneFlow},
};

use super::{builder::AppBuilder, window::AppWindow, AppHandle};

/// 고정 시간 갱신에 사용되는 경과 시간입니다.
pub const FIXED_TIME_SEC: f32 = 1.0 / 60.0;

/// 최대 고정 시간 갱신 횟수입니다.
pub const MAX_FIXED_UPDATE: usize = 8;

/// 애플리케이션 관리 구조체입니다.
pub struct Application {
    /// 사용자 정의 이벤트를 이벤트 루프로 보내는 프록시입니다.
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,

    /// 입/출력 스레드 풀 객체입니다.
    io_threads: ThreadPool,

    /// 애플리케이션 실행 디렉토리 경로입니다.
    current_dir: PathBuf,

    /// 애플리케이션 에셋 관리자입니다.
    asset_manager: AssetManager,

    /// 애플리케이션 네트워크 매니저입니다.
    net_manager: NetManager,

    /// 애플리케이션 플래그 옵션입니다.
    flags: AppFlags,

    /// 애플리케이션 창 제목 텍스트입니다.
    window_title: String,

    /// 애플리케이션 창 아이콘입니다.
    window_icon: Option<Icon>,

    /// 애플리케이션 창 크기입니다.
    window_size: WindowSize,

    /// 애플리케이션 창의 전체화면 여부입니다.
    fullscreen: bool,

    /// 애플리케이션 창의 표시 여부입니다.
    visible: bool,

    /// 애플리케이션 키보드 수정자 상태정보입니다.
    modifier: Modifiers,

    /// 커서의 이전 위치입니다.
    cursor_delta: PhysicalPosition<f64>,

    /// 게임 장면 스택을 제어하는 게임 장면 흐름입니다.
    scene_flow: VecDeque<GameSceneFlow>,

    /// 생성된 게임 장면을 관리하는 게임 장면 스택입니다.
    scene_stack: RefCell<VecDeque<Box<dyn GameScene>>>,

    /// 업데이트 경과 시간을 측정하는 타이머입니다.
    timer: GameTimer,

    /// `egui`의 컨텍스트입니다.
    egui_ctx: egui::Context,

    /// `egui`의 렌더러입니다.
    egui_renderer: RefCell<UiRenderer>,

    /// `wgpu` 렌더링 인스턴스입니다.
    instance: Arc<wgpu::Instance>,

    /// `wgpu` 렌더링 장치 어댑터입니다.
    adapter: Arc<wgpu::Adapter>,

    /// `wgpu` 렌더링 논리적 장치입니다.
    device: Arc<wgpu::Device>,

    /// `wgpu` 렌더링 장치 명령 대기열입니다.
    queue: Arc<wgpu::Queue>,

    /// 애플리케이션 창입니다.
    app_window: Option<AppWindow>,
}

impl Application {
    /// 애플리케이션을 생성합니다.
    ///
    /// # Errors
    /// 애플리케이션 렌더러를 생성하는 도중 오류가 발생한 경우 `RenderError`를 반환합니다.
    ///
    pub(crate) async fn new(
        event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,
        builder: AppBuilder,
    ) -> Result<Self, Box<dyn Error + Send>> {
        // 작업 스레드 풀 객체를 생성합니다.
        ThreadPoolBuilder::new()
            .num_threads(builder.num_threads.max(1))
            .thread_name(|id| format!("Task_Thread({})", id))
            .build_global()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // 입/출력 스레드 풀 객체를 생성합니다.
        let io_threads = ThreadPoolBuilder::new()
            .num_threads((builder.num_threads / 2).max(1))
            .thread_name(|id| format!("IO_Thread({})", id))
            .build()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // wgpu 렌더러를 생성합니다.
        let (instance, adapter, device, queue) = init_wgpu()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // 네트워크 매니저를 생성합니다.
        let network = NetManager::new(event_loop_proxy.clone());

        // 에셋 관리자를 생성합니다.
        let mut root_dir = unsafe { builder.current_dir.clone().unwrap_unchecked() }; // Safe: 빌더를 생성할 때 존재 유무를 확인함.
        root_dir.push("assets");
        let bundle =
            AssetManager::new(root_dir).map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        Ok(Self {
            event_loop_proxy,
            io_threads,
            current_dir: unsafe { builder.current_dir.unwrap_unchecked() }, // Safe: 빌더 생성 중 확인함.
            asset_manager: bundle,
            net_manager: network,
            flags: builder.flags,
            window_title: builder.title.unwrap_or("Hello to Halo".to_string()),
            window_icon: builder.icon,
            window_size: builder.size.unwrap_or(WindowSize::MAX),
            fullscreen: builder.fullscreen,
            visible: builder.visible,
            modifier: Modifiers::default(),
            cursor_delta: PhysicalPosition::default(),
            scene_flow: VecDeque::from_iter([GameSceneFlow::Reset(builder.start_scene)]),
            scene_stack: RefCell::new(VecDeque::with_capacity(8)),
            timer: GameTimer::start(),
            egui_ctx: egui::Context::default(),
            egui_renderer: RefCell::new(UiRenderer::new(&device, SWAPCHAIN_FORMAT, None, 1, false)),
            instance,
            adapter,
            device,
            queue,
            app_window: None,
        })
    }

    fn draw(
        &self,
        window: &Window,
        egui_ctx: &egui::Context,
        egui_raw_input: egui::RawInput,
        egui_renderer: &mut UiRenderer,
        surface: &wgpu::Surface<'static>,
        depth_buffer_view: &wgpu::TextureView,
        scene_stack: &mut VecDeque<Box<dyn GameScene>>,
    ) {
        // 그려야 하는 게임 장면의 시작 인덱스를 계산합니다.
        let mut begin = scene_stack.len();
        for scene in scene_stack.iter().rev() {
            begin -= 1;

            if !scene.transparents() {
                break;
            }
        }

        // 현재 게임 장면의 그리기 준비 콜백 함수를 호출합니다.
        for i in begin..scene_stack.len() {
            scene_stack[i].on_prepare_draw(&window, self);
        }

        // UI 그리기 준비를 합니다.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        for i in (begin..scene_stack.len()).rev() {
            scene_stack[i].ui_callback(window, self);
        }
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }
        commands.push(encoder.finish());
        self.queue.submit(commands);

        // 이전 렌더링 작업이 끝날때 까지 대기합니다.
        while !self.device.poll(wgpu::Maintain::Poll).is_queue_empty() {
            std::hint::spin_loop();
            std::thread::yield_now();
        }

        // 현재 프레임 버퍼를 가져옵니다.
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Timeout) => {
                log::info!("frame skip >> swapchin needs to be refreshed.");
                return;
            }
            Err(
                wgpu::SurfaceError::Outdated
                | wgpu::SurfaceError::Lost
                | wgpu::SurfaceError::Other
                | wgpu::SurfaceError::OutOfMemory,
            ) => {
                let vsync = !self.flags.contains(AppFlags::DISABLE_VSYNC);
                let (width, height) = window.inner_size().into();
                config_swapchain(width, height, &self.device, surface, vsync);
                match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        log::error!("failed to acquire next sufrace texture! (REASON:{})", &e);
                        let title = "Runtime error".into();
                        let message = "Failed to acquire next surface texture!".into();
                        let alert = Alert { title, message };
                        show_error_msg(alert, Some(window));
                        std::process::exit(-1);
                    }
                }
            }
        };

        let (width, height): (u32, u32) = window.inner_size().into();
        if width != frame.texture.width() || height != frame.texture.height() {
            // 현재 스왑체인 텍스처 버퍼가 갱신이 필요한 경우 렌더링을 생략합니다.
            log::info!("frame skip >> swapchin needs to be refreshed.");
            return;
        }

        // 렌더 타겟 뷰를 가져옵니다.
        let render_target_view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            ..Default::default()
        });

        // 현재 게임 장면에 그리기 콜백 함수를 호출합니다.
        // 현재 게임 장면의 그리기 준비 콜백 함수를 호출합니다.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for i in begin..scene_stack.len() {
            scene_stack[i].on_draw(
                window,
                &mut encoder,
                &render_target_view,
                depth_buffer_view,
                self,
            );
        }

        // UI 렌더 패스
        encoder.push_debug_group("UI pass");
        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(UI)"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        view: &render_target_view,
                        resolve_target: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();

            egui_renderer.render(&mut rpass, &egui_primitive, &screen_descriptor);
        }
        encoder.pop_debug_group();

        // 그리기 명령을 제출합니다.
        self.queue.submit([encoder.finish()]);

        // `winit` API에 애플리케이션 창을 갱신한다고 알립니다.
        window.pre_present_notify();

        // 프레임 버퍼를 출력합니다.
        frame.present();

        // 현재 게임 장면의 그리기 마침 콜백 함수를 호출합니다.
        for i in begin..scene_stack.len() {
            scene_stack[i].on_finish_draw(&window, self);
        }
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
        // 시스템에서 사용 가능한 창의 최대 크기를 가져옵니다.
        let max_window_size = event_loop
            .primary_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor))
            .flatten();

        let max_window_size = match max_window_size {
            Some(size) => size,
            None => {
                log::error!("no suitable resolution found.");
                let title = "Window creation failed".into();
                let message = "No suitable resolution found.".into();
                let alert = Alert { title, message };
                show_error_msg(alert, None);
                return event_loop.exit();
            }
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
            .with_visible(self.visible)
            .with_active(true);

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::CornerPreference;
            use winit::platform::windows::WindowAttributesExtWindows;
            attributes = attributes.with_corner_preference(CornerPreference::DoNotRound);
        }

        self.app_window = match AppWindow::create(
            event_loop,
            attributes,
            &self.flags,
            &self.egui_ctx,
            &self.instance,
            &self.adapter,
            &self.device,
        ) {
            Ok(app_window) => Some(app_window),
            Err(e) => {
                log::error!("{e}");
                let title = "Window creation failed".into();
                let message = e.to_string();
                let alert = Alert { title, message };
                show_error_msg(alert, None);
                return event_loop.exit();
            }
        };
    }

    /// 일반적으로 `Android` 시스템이 아닌경우 이 함수는 호출되지 않습니다.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        drop(self.app_window.take());
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // `winit` 윈도우 핸들을 가져옵니다.
        let window = self
            .app_window
            .as_ref()
            .map(|app_window| app_window.window.as_ref());

        // 게임 장면 스택을 정리합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        clear_scene(&mut scene_stack, window, self);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 애플리케이션 창을 가져옵니다.
        // 애플리케이션 창이 존재하지 않는 경우 애플리케이션을 종료합니다.
        let app_window = match self.app_window.as_ref() {
            Some(app_window) => app_window,
            None => return event_loop.exit(),
        };

        // 게임 장면 스택을 갱신합니다.
        let window = app_window.window.as_ref();
        let mut scene_stack = self.scene_stack.borrow_mut();
        while let Some(flow) = self.scene_flow.pop_front() {
            match flow {
                GameSceneFlow::Clear => clear_scene(&mut scene_stack, Some(&window), self),
                GameSceneFlow::Reset(new_scene) => {
                    reset_scene(&mut scene_stack, &window, self, new_scene)
                }
                GameSceneFlow::Change(new_scene) => {
                    change_scene(&mut scene_stack, &window, self, new_scene)
                }
                GameSceneFlow::Push(new_scene) => {
                    push_scene(&mut scene_stack, &window, self, new_scene)
                }
                GameSceneFlow::Pop => pop_scene(&mut scene_stack, &window, self),
            }
        }

        // 게임 장면이 존재하지 않는 경우 애플리케이션을 종료합니다.
        if scene_stack.is_empty() {
            return event_loop.exit();
        }

        // 총 경과 시간을 계산합니다.
        let elapsed_time_sec = self.timer.elapsed_time_sec();

        // 게임 장면을 갱신합니다.
        let mut count: usize;
        let mut total_time_sec;
        for scene in scene_stack.iter_mut().rev() {
            // 게임 장면 갱신 시작 콜백 함수를 호출합니다.
            scene.on_pre_update(window, self);

            // 변동 시간 갱신 함수를 호출합니다.
            scene.on_update(elapsed_time_sec, window, self);

            // 고정 시간 갱신 함수를 호출합니다.
            count = 0;
            total_time_sec = elapsed_time_sec;
            while count < MAX_FIXED_UPDATE && FIXED_TIME_SEC <= total_time_sec {
                scene.on_fixed_update(FIXED_TIME_SEC, window, self);
                total_time_sec -= FIXED_TIME_SEC;
                count += 1;
            }
            scene.on_fixed_update(total_time_sec, window, self);

            // 게임 장면 갱신 완료 콜백 함수를 호출합니다.
            scene.on_post_update(window, self);

            if !scene.should_update_subscene() {
                break;
            }
        }

        // 애플리케이션 창을 갱신합니다.
        window.request_redraw();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if let Some(app_window) = self.app_window.as_ref() {
                    let mut state = app_window.egui_state.borrow_mut();
                    state.on_mouse_motion(delta);
                }
                self.cursor_delta = delta.into();
            }
            _ => {}
        }
    }

    /// NOTE: 이 함수에서 이벤트 루프의 종료 함수를 호출할 경우 `panic!`이 발생합니다.
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // 애플리케이션 창을 가져옵니다.
        // 애플리케이션 창이 존재하지 않는 경우 함수 실행을 생략합니다.
        let app_window = match self.app_window.as_ref() {
            Some(app_window) => app_window,
            None => {
                log::debug!("window event ignored >> the current window is empty.");
                return;
            }
        };

        // `winit` 윈도우 식별자가 다른 경우 함수 실행을 생략합니다.
        if window_id != app_window.window.id() {
            return;
        }

        // 게임 장면 스택을 가져옵니다.
        // 게임 장면 스택이 비어있는 경우 함수 실행을 생략합니다.
        let mut scene_stack = self.scene_stack.borrow_mut();
        if scene_stack.is_empty() {
            log::debug!("window event ignored >> the current scene is empty.");
            return;
        }

        // UI 인터페이스 윈도우를 갱신합니다.
        let mut state = app_window.egui_state.borrow_mut();
        let _ = state.on_window_event(&app_window.window, &event);
        drop(state);

        // 윈도우 이벤트를 처리합니다.
        match event {
            WindowEvent::Resized(_) => {
                app_window.on_resized(&self.instance, &self.device);
                for scene in scene_stack.iter_mut().rev() {
                    scene.on_window_resized(&app_window.window, self);
                }
            }
            WindowEvent::Moved(_) => {
                for scene in scene_stack.iter_mut().rev() {
                    scene.on_window_moved(&app_window.window, self);
                }
            }
            WindowEvent::CloseRequested => {
                // Safe: 장면 스택이 비어있는지 확인함.
                let current_scene = unsafe { scene_stack.back_mut().unwrap_unchecked() };
                if current_scene.on_close_request(self) {
                    self.app_window.take();
                    return;
                };
            }
            WindowEvent::Focused(focused) => {
                // Safe: 장면 스택이 비어있는지 확인함.
                let current_scene = unsafe { scene_stack.back_mut().unwrap_unchecked() };
                if focused {
                    current_scene.on_enter_foreground(self);
                } else {
                    current_scene.on_enter_background(self);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state.is_pressed() {
                        for scene in scene_stack.iter_mut().rev() {
                            if scene.on_keyboard_pressed(
                                code,
                                event.location,
                                self.modifier,
                                event.repeat,
                                &app_window.window,
                                self,
                            ) {
                                break;
                            }
                        }
                    } else {
                        for scene in scene_stack.iter_mut().rev() {
                            if scene.on_keyboard_released(
                                code,
                                event.location,
                                self.modifier,
                                event.repeat,
                                &app_window.window,
                                self,
                            ) {
                                break;
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                #[cfg(target_os = "windows")]
                if app_window.get_cursor_disabled() {
                    let (w, h): (u32, u32) = app_window.window.inner_size().into();
                    let _ = app_window
                        .window
                        .set_cursor_position(PhysicalPosition::new(w / 2, h / 2));
                }

                let (dx, dy): (f32, f32) = self.cursor_delta.into();
                for scene in scene_stack.iter_mut().rev() {
                    if scene.on_cursor_moved(
                        position.x as f32,
                        position.y as f32,
                        dx as f32,
                        dy as f32,
                        &app_window.window,
                        self,
                    ) {
                        break;
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let MouseScrollDelta::LineDelta(dx, dy) = delta {
                    for scene in scene_stack.iter_mut().rev() {
                        if scene.on_mouse_wheel(dx, dy, &app_window.window, self) {
                            break;
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let (x, y): (f64, f64) = self.cursor_delta.into();
                if state.is_pressed() {
                    for scene in scene_stack.iter_mut().rev() {
                        if scene.on_mouse_btn_pressed(
                            x as f32,
                            y as f32,
                            button,
                            &app_window.window,
                            self,
                        ) {
                            break;
                        }
                    }
                } else {
                    for scene in scene_stack.iter_mut().rev() {
                        if scene.on_mouse_btn_released(
                            x as f32,
                            y as f32,
                            button,
                            &app_window.window,
                            self,
                        ) {
                            break;
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                app_window.on_resized(&self.instance, &self.device);
                for scene in scene_stack.iter_mut().rev() {
                    scene.on_window_resized(&app_window.window, self);
                }
            }
            WindowEvent::ModifiersChanged(modifier) => {
                self.modifier = modifier;
            }
            WindowEvent::RedrawRequested => {
                let mut state = app_window.egui_state.borrow_mut();
                let egui_raw_input = state.take_egui_input(&app_window.window);
                let mut egui_renderer = self.egui_renderer.borrow_mut();
                let depth_buffer_view = app_window.depth_buffer_view.borrow();
                self.draw(
                    &app_window.window,
                    &self.egui_ctx,
                    egui_raw_input,
                    &mut egui_renderer,
                    &app_window.surface,
                    &depth_buffer_view,
                    &mut scene_stack,
                )
            }
            _ => {}
        };
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        //     // 애플리케이션 창을 가져옵니다.
        //     // 애플리케이션 창이 존재하지 않는 경우 함수 실행을 생략합니다.
        //     let app_window = match self.app_window.as_ref() {
        //         Some(app_window) => app_window,
        //         None => return,
        //     };

        //     // 현재 게임 장면을 가져옵니다.
        //     // 현재 게임 장면이 존재하지 않는 경우 함수 실행을 생략합니다.
        //     let mut scene_stack = self.scene_stack.borrow_mut();
        //     let curr_scene = match scene_stack.back_mut() {
        //         Some(curr_scene) => curr_scene,
        //         None => return,
        //     };

        match event {
            AppEvent::AddGameSceneFlow(flow) => {
                self.scene_flow.push_back(flow);
                return;
            }
            AppEvent::ResizeRequest(request_size) => {
                // 애플리케이션 창을 가져옵니다.
                let app_window = match self.app_window.as_ref() {
                    Some(app_window) => app_window,
                    None => {
                        log::warn!("app event ignored >> the window was not created!");
                        return;
                    }
                };

                // 현재 해상도와 같을 경우 이 이벤트를 무시합니다.
                if self.window_size == request_size {
                    return;
                }

                match app_window.window.request_inner_size(request_size.size()) {
                    Some(result_size) => {
                        if request_size.size() == result_size {
                            // 창의 크기가 즉시 적용됐습니다.
                            self.window_size = request_size;
                            app_window.on_resized(&self.instance, &self.device);
                        } else {
                            log::warn!(
                                "app event ignored >> the current system does not allow resizing the window!"
                            );
                        }
                    }
                    None => {
                        // 윈도우 이벤트를 통해 창의 크기가 조정됩니다.
                    }
                };
            }
            AppEvent::FullScreenRequest(fullscreen) => {
                // 애플리케이션 창을 가져옵니다.
                let app_window = match self.app_window.as_ref() {
                    Some(app_window) => app_window,
                    None => {
                        log::warn!("app event ignored >> the window was not created!");
                        return;
                    }
                };

                if self.fullscreen != fullscreen {
                    self.fullscreen = fullscreen;

                    #[cfg(target_os = "macos")]
                    {
                        use winit::platform::macos::WindowExtMacOS;
                        app_window.window.set_borderless_game(self.fullscreen);
                        app_window.window.set_simple_fullscreen(self.fullscreen);
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        app_window.window.set_fullscreen(
                            self.fullscreen.then_some(Fullscreen::Borderless(None)),
                        );
                    }
                }
            }
            AppEvent::Alert(alert) => {
                let parent = self
                    .app_window
                    .as_ref()
                    .map(|app_wnd| app_wnd.window.as_ref());
                show_error_msg(alert, parent);
            }
            AppEvent::NetworkError(error) => {
                let mut scene_stack = self.scene_stack.borrow_mut();
                match scene_stack.back_mut() {
                    Some(scene) => {
                        scene.handle_network_error(error, self);
                    }
                    None => {
                        let title = String::from("Network error");
                        let message = match error {
                            NetworkError::ClosedSocket(_) => {
                                format!("The connection to the game server was lost.")
                            }
                            NetworkError::IO(e) => {
                                format!("Socket I/O failed for the following reasons: {e}")
                            }
                        };
                        let parent = self
                            .app_window
                            .as_ref()
                            .map(|app_wnd| app_wnd.window.as_ref());
                        show_error_msg(Alert { title, message }, parent);
                    }
                };
            }
            AppEvent::PacketReceived(packet) => {
                // 현재 애플리케이션 장면을 가져옵니다.
                let mut scene_stack = self.scene_stack.borrow_mut();
                if scene_stack.is_empty() {
                    log::warn!("packet ignored >> the current game scene is empty!");
                    return;
                }

                let mut temp = Some(packet);
                for scene in scene_stack.iter_mut().rev() {
                    if let Some(packet) = temp.take() {
                        temp = scene.on_received_packet(packet, self);
                    }
                }
            }
        };
    }
}

impl AppHandle for Application {
    fn enable_cursor(&self) {
        if let Some(app_window) = self.app_window.as_ref() {
            app_window.window.set_cursor_visible(true);
            app_window.window.confine_cursor_to_window(false);
            app_window.set_cursor_disable(false);
        }
    }

    fn disable_cursor(&self) {
        if let Some(app_window) = self.app_window.as_ref() {
            app_window.window.set_cursor_visible(false);
            app_window.window.confine_cursor_to_window(true);
            app_window.set_cursor_disable(true);
        }
    }

    fn event_loop_proxy(&self) -> &Arc<EventLoopProxy<AppEvent>> {
        &self.event_loop_proxy
    }

    fn io_threads(&self) -> &ThreadPool {
        &self.io_threads
    }

    fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    fn asset_manager(&self) -> &AssetManager {
        &self.asset_manager
    }

    fn net_manager(&self) -> &NetManager {
        &self.net_manager
    }

    fn flags(&self) -> AppFlags {
        self.flags
    }

    fn window_title(&self) -> &str {
        &self.window_title
    }

    fn window_size(&self) -> WindowSize {
        self.window_size
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    fn timer(&self) -> &GameTimer {
        &self.timer
    }

    fn render_instance(&self) -> &Arc<wgpu::Instance> {
        &self.instance
    }

    fn render_adapter(&self) -> &Arc<wgpu::Adapter> {
        &self.adapter
    }

    fn render_device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    fn render_queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    fn egui_raw_input(&self) -> egui::RawInput {
        self.app_window
            .as_ref()
            .map(|app_window| {
                let mut state = app_window.egui_state.borrow_mut();
                state.take_egui_input(&app_window.window)
            })
            .unwrap_or_default()
    }

    fn egui_renderer_mut(&self) -> RefMut<'_, UiRenderer> {
        self.egui_renderer.borrow_mut()
    }

    fn window(&self) -> Option<&Arc<Window>> {
        self.app_window
            .as_ref()
            .map(|app_window| &app_window.window)
    }

    fn render_surface(&self) -> Option<&Arc<wgpu::Surface<'static>>> {
        self.app_window
            .as_ref()
            .map(|app_window| &app_window.surface)
    }
}

/// 모든 게임 장면을 제거합니다.
fn clear_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: Option<&Window>,
    app: &dyn AppHandle,
) {
    while let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(window, app);
    }
}

/// 모든 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
fn reset_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: &Window,
    app: &dyn AppHandle,
    mut new_scene: Box<dyn GameScene>,
) {
    while let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(Some(window), app);
    }

    log::info!("Enter GameScene({:?})", &new_scene);
    new_scene.on_enter(window, app);
    stack.push_back(new_scene);
}

/// 현재 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
fn change_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: &Window,
    app: &dyn AppHandle,
    mut new_scene: Box<dyn GameScene>,
) {
    if let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(Some(window), app);
    }

    log::info!("Enter GameScene({:?})", &new_scene);
    new_scene.on_enter(window, app);
    stack.push_back(new_scene);
}

/// 새로운 게임 장면을 초기화하고, 추가합니다.
fn push_scene(
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: &Window,
    app: &dyn AppHandle,
    mut new_scene: Box<dyn GameScene>,
) {
    if let Some(scene) = stack.back_mut() {
        log::info!("Pause GameScene({:?})", &scene);
        scene.on_pause(window, app);
    }

    log::info!("Enter GameScene({:?})", &new_scene);
    new_scene.on_enter(window, app);
    stack.push_back(new_scene);
}

/// 현재 장면을 정리하고, 제거합니다.
fn pop_scene(stack: &mut VecDeque<Box<dyn GameScene>>, window: &Window, app: &dyn AppHandle) {
    if let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(Some(window), app);
    }
    if let Some(scene) = stack.back_mut() {
        log::info!("Resume GameScene({:?})", &scene);
        scene.on_resume(window, app);
    }
}
