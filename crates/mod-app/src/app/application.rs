use std::{
    collections::VecDeque,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};

use mod_render::{
    config_swapchain, init_wgpu, ScreenDescriptor, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use rodio::{mixer::Mixer, OutputStream, OutputStreamBuilder};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{DeviceEvent, Modifiers, MouseScrollDelta, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::{Fullscreen, Icon, Window, WindowButtons},
};

use crate::{
    app::render::FrameResource,
    error::{show_error_msg, Alert},
    etc::{AppEvent, AppFlags, GameTimer, Viewport, WindowSize},
    ext::AppWindowExt,
    net::NetManager,
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

    /// 애플리케이션 뷰포트 영역입니다.
    viewport: Viewport,

    /// 애플리케이션 창의 전체화면 여부입니다.
    fullscreen: bool,

    /// 애플리케이션 창의 표시 여부입니다.
    visible: bool,

    /// 애플리케이션 키보드 수정자 상태정보입니다.
    modifier: Modifiers,

    /// 커서의 이전 위치입니다.
    cursor_delta: Option<PhysicalPosition<f64>>,

    /// 게임 장면 스택을 제어하는 게임 장면 흐름입니다.
    scene_flow: VecDeque<GameSceneFlow>,

    /// 생성된 게임 장면을 관리하는 게임 장면 스택입니다.
    scene_stack: Option<VecDeque<Box<dyn GameScene>>>,

    /// 업데이트 경과 시간을 측정하는 타이머입니다.
    timer: GameTimer,

    /// 소리 출력 디바이스의 핸들입니다.
    stream_handle: OutputStream,

    /// `egui`의 컨텍스트입니다.
    egui_ctx: egui::Context,

    /// `egui`의 렌더러입니다.
    egui_renderer: Option<UiRenderer>,

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

    /// 프레임 쉐이더 리소스입니다.
    frame_resource: Option<FrameResource>,
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

        // 오디오 장치를 생성합니다.
        let stream_handle = OutputStreamBuilder::open_default_stream()
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // 최상위 에셋 디렉토리를 가져옵니다.
        let mut root_dir = unsafe { builder.current_dir.clone().unwrap_unchecked() }; // Safe: 빌더를 생성할 때 존재 유무를 확인함.
        root_dir.push("assets");

        Ok(Self {
            event_loop_proxy,
            io_threads,
            current_dir: unsafe { builder.current_dir.unwrap_unchecked() }, // Safe: 빌더 생성 중 확인함.
            net_manager: network,
            flags: builder.flags,
            window_title: builder.title.unwrap_or("Hello to Halo".to_string()),
            window_icon: builder.icon,
            window_size: builder.size.unwrap_or(WindowSize::MAX),
            viewport: Viewport::default(),
            fullscreen: builder.fullscreen,
            visible: builder.visible,
            modifier: Modifiers::default(),
            cursor_delta: None,
            scene_flow: VecDeque::from_iter([GameSceneFlow::Reset(builder.start_scene)]),
            scene_stack: Some(VecDeque::with_capacity(8)),
            timer: GameTimer::start(),
            stream_handle,
            egui_ctx: egui::Context::default(),
            egui_renderer: Some(UiRenderer::new(&device, SWAPCHAIN_FORMAT, None, 1, false)),
            instance,
            adapter,
            device,
            queue,
            app_window: None,
            frame_resource: None,
        })
    }

    /// 뷰포트 영역을 계산하고 설정합니다.
    fn set_viewport_area(
        &mut self,
        window_width: f32,
        window_height: f32,
        content_aspect_ratio: f32,
    ) {
        let window_aspect_ratio = window_width / window_height;
        if window_aspect_ratio > content_aspect_ratio {
            // 창이 가로로 더 넓은 경우: 위-아래 레터박스
            let content_height = window_height;
            let content_width = content_height * content_aspect_ratio;
            let x = ((window_width - content_width) * 0.5).max(0.0);
            let y = 0.0;
            self.viewport = Viewport::new(x, y, content_width, content_height, 0.0, 1.0);
        } else {
            // 창이 세로로 더 넓은 경우: 좌-우 레터박스
            let content_width = window_width;
            let content_height = content_width / content_aspect_ratio;
            let x = 0.0;
            let y = ((window_height - content_height) * 0.5).max(0.0);
            self.viewport = Viewport::new(x, y, content_width, content_height, 0.0, 1.0);
        }
    }

    fn draw(
        &mut self,
        app_window: AppWindow,
        mut scene_stack: VecDeque<Box<dyn GameScene>>,
        mut egui_renderer: UiRenderer,
        egui_raw_input: egui::RawInput,
    ) -> (AppWindow, VecDeque<Box<dyn GameScene>>, UiRenderer, bool) {
        // 현재 프레임 버퍼를 가져옵니다.
        let window = app_window.window.as_ref();
        let surface = app_window.surface.as_ref();
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Timeout) => {
                log::info!("frame skip >> swapchin needs to be refreshed.");
                return (app_window, scene_stack, egui_renderer, true);
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
                        return (app_window, scene_stack, egui_renderer, false);
                    }
                }
            }
        };

        let (width, height): (u32, u32) = window.inner_size().into();
        if width != frame.texture.width() || height != frame.texture.height() {
            // 현재 스왑체인 텍스처 버퍼가 갱신이 필요한 경우 렌더링을 생략합니다.
            log::info!("frame skip >> swapchin needs to be refreshed.");
            return (app_window, scene_stack, egui_renderer, true);
        }

        // 프레임 쉐이더 리소스를 가져옵니다.
        let frame_resource = match self.frame_resource.as_ref() {
            Some(resource) => resource,
            None => {
                // 현재 콘텐츠 렌더 타겟 버퍼가 갱신이 필요한 경우 렌더링을 생략합니다.
                log::info!("frame skip >> content render target buffer needs to be refreshed.");
                return (app_window, scene_stack, egui_renderer, true);
            }
        };

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
            scene_stack[i].on_prepare_draw(window, self);
        }

        // UI 그리기 준비를 합니다.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        // 각 장면의 Ui 콜백 함수를 호출합니다.
        self.egui_ctx.begin_pass(egui_raw_input);
        for i in (begin..scene_stack.len()).rev() {
            scene_stack[i].ui_callback(window, self);
        }
        let egui_full_output = self.egui_ctx.end_pass();

        let egui_primitive = self
            .egui_ctx
            .tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
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
        loop {
            std::hint::spin_loop();
            let result = self.device.poll(wgpu::PollType::Poll);
            match result {
                Ok(status) => {
                    if status.is_queue_empty() {
                        break;
                    } else {
                        continue;
                    }
                }
                Err(e) => match e {
                    wgpu::PollError::Timeout => {
                        log::error!("{e}");
                        let title = "Processing Timeout".into();
                        let message = "Graphics processing timeout exceeded!".into();
                        let alert = Alert { title, message };
                        show_error_msg(alert, Some(window));
                        return (app_window, scene_stack, egui_renderer, false);
                    }
                },
            }
        }

        // 현재 게임 장면에 그리기 콜백 함수를 호출합니다.
        // 현재 게임 장면의 그리기 준비 콜백 함수를 호출합니다.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for i in begin..scene_stack.len() {
            scene_stack[i].on_draw(
                window,
                &mut encoder,
                &frame_resource.get_render_target_view(),
                &frame_resource.get_depth_buffer_view(),
                self,
            );
        }

        // 프레임 버퍼의 렌더 타겟 뷰를 가져옵니다.
        let frame_render_target_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Frame 렌더 패스
        encoder.push_debug_group("frame pass");
        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(Frame)"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        view: &frame_render_target_view,
                        resolve_target: None,
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();

            rpass.set_viewport(
                self.viewport.x,
                self.viewport.y,
                self.viewport.width,
                self.viewport.height,
                0.0,
                1.0,
            );
            frame_resource.process(&self.device, &mut rpass);
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

        (app_window, scene_stack, egui_renderer, true)
    }
}

impl ApplicationHandler<AppEvent> for Application {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.timer.tick();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `winit` API는 애플리케이션이 생성되었을 때 `ApplicationHandler::resumed`를 호출합니다.
        // 또한 일부 시스템(예: `Android`)은 애플리케이션 초기화 이전에 창을 생성하는 것이 허용되지 않습니다.
        // 따라서 이 콜백 함수에서 애플리케이션 창을 생성하고, 렌더러 표면을 생성해야 합니다.
        //

        // 주 모니터의 크기를 가져옵니다.
        let result = event_loop.primary_monitor().map(|monitor| monitor.size());
        let screen_size = match result {
            Some(size) => size,
            None => {
                log::error!("no available monitors found!");
                let title = "Window creation failed".into();
                let message = "No available monitor found!".into();
                let alert = Alert { title, message };
                show_error_msg(alert, None);
                return event_loop.exit();
            }
        };

        // 최대 해상도를 계산합니다.
        let result = WindowSize::find_maximize_size(screen_size);
        let max_window_size = match result {
            Some(size) => size,
            None => {
                log::error!("no suitable resolution found!");
                let title = "Window creation failed".into();
                let message = "No suitable resolution found!".into();
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
            .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
            // .with_resizable(false)
            .with_resizable(true)
            .with_visible(self.visible)
            .with_active(true);

        if self.fullscreen {
            attributes = attributes
                .with_inner_size(screen_size)
                .with_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            attributes = attributes
                .with_inner_size(self.window_size.size())
                .with_fullscreen(None);
        }

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::CornerPreference;
            use winit::platform::windows::WindowAttributesExtWindows;
            attributes = attributes.with_corner_preference(CornerPreference::DoNotRound);
        }
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::OptionAsAlt;
            use winit::platform::macos::WindowAttributesExtMacOS;
            attributes = attributes.with_option_as_alt(OptionAsAlt::Both);
        }

        let app_window = match AppWindow::create(
            event_loop,
            attributes,
            &self.flags,
            &self.egui_ctx,
            &self.instance,
            &self.adapter,
            &self.device,
        ) {
            Ok(app_window) => app_window,
            Err(e) => {
                log::error!("{e}");
                let title = "Window creation failed".into();
                let message = e.to_string();
                let alert = Alert { title, message };
                show_error_msg(alert, None);
                return event_loop.exit();
            }
        };

        // 뷰포트 영역을 설정합니다.
        let (window_width, window_height) = app_window.window.inner_size().into();
        let content_aspect_ratio = self.window_size.aspect_ratio();
        self.set_viewport_area(window_width, window_height, content_aspect_ratio);

        // 프레임 쉐이더 리소스를 생성합니다.
        let (content_width, content_height) = self.window_size.size().into();
        self.frame_resource = Some(FrameResource::new(
            &self.device,
            content_width,
            content_height,
            SWAPCHAIN_FORMAT,
            DEPTH_FORMAT,
        ));

        // 애플리케이션 윈도우를 설정합니다.
        self.app_window = Some(app_window);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // 일반적으로 `Android` 시스템이 아닌경우 이 함수는 호출되지 않습니다.
        drop(self.app_window.take());
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // `winit` 윈도우 핸들을 가져옵니다.
        let window = self
            .app_window
            .as_ref()
            .map(|app_window| app_window.window.as_ref());

        // 게임 장면 스택을 정리합니다.
        let zip = self.scene_stack.take().zip(self.egui_renderer.take());
        if let Some((mut scene_stack, mut ui_renderer)) = zip {
            clear_scene(&mut ui_renderer, &mut scene_stack, window, self);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 애플리케이션 창과 장면 스택, 그리고 Ui 렌더러의 `소유권`을 가져옵니다.
        // 애플리케이션 창 또는 장면 스택 또는 Ui 렌더러가 존재하지 않는 경우 애플리케이션을 종료합니다.
        let zip = self
            .app_window
            .take()
            .zip(self.scene_stack.take())
            .zip(self.egui_renderer.take());
        let ((app_window, mut scene_stack), mut ui_renderer) = match zip {
            Some(it) => it,
            None => return event_loop.exit(),
        };

        // 게임 장면 스택을 갱신합니다.
        let window = app_window.window.as_ref();
        while let Some(flow) = self.scene_flow.pop_front() {
            match flow {
                GameSceneFlow::Clear => {
                    clear_scene(&mut ui_renderer, &mut scene_stack, Some(&window), self)
                }
                GameSceneFlow::Reset(new_scene) => {
                    reset_scene(&mut ui_renderer, &mut scene_stack, &window, self, new_scene)
                }
                GameSceneFlow::Change(new_scene) => {
                    change_scene(&mut ui_renderer, &mut scene_stack, &window, self, new_scene)
                }
                GameSceneFlow::Push(new_scene) => {
                    push_scene(&mut ui_renderer, &mut scene_stack, &window, self, new_scene)
                }
                GameSceneFlow::Pop => pop_scene(&mut ui_renderer, &mut scene_stack, &window, self),
            }
        }

        // 게임 장면이 존재하지 않는 경우 애플리케이션을 종료합니다.
        if scene_stack.is_empty() {
            // 장면 스택과 Ui 렌더러의 소유권을 돌려 놓습니다. (생성된 게임 장면을 정리하기 위함)
            self.scene_stack = Some(scene_stack);
            self.egui_renderer = Some(ui_renderer);

            // 이벤트 루프를 종료합니다.
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

        // 애플리케이션 창과 장면 스택, 그리고 Ui 렌더러의 `소유권`을 돌려놓습니다.
        self.app_window = Some(app_window);
        self.scene_stack = Some(scene_stack);
        self.egui_renderer = Some(ui_renderer);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        // 애플리케이션 창의 `소유권`을 가져옵니다.
        // 애플리케이션 창이 존재하지 않는 경우 함수 실행을 생략합니다.
        let mut app_window = match self.app_window.take() {
            Some(app_window) => app_window,
            None => {
                log::debug!("device event ignored >> the current window is empty.");
                return;
            }
        };

        match event {
            DeviceEvent::MouseMotion { delta } => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
                    app_window.egui_state.on_mouse_motion(delta);

                    match self.cursor_delta.as_mut() {
                        Some(cursor_delta) => *cursor_delta = delta.into(),
                        None => self.cursor_delta = Some(PhysicalPosition::default()),
                    };
                }
            }
            _ => {}
        }

        // 애플리케이션 창의 `소유권`을 돌려놓습니다.
        self.app_window = Some(app_window);
    }

    // NOTE: 이 함수에서 이벤트 루프의 종료 함수를 호출할 경우 `panic!`이 발생합니다.
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // 애플리케이션 창과 장면 스택의 `소유권`을 가져옵니다.
        // 애플리케이션 창 또는 장면 스택이 존재하지 않는 경우 함수 실행을 생략합니다.
        let zip = self.app_window.take().zip(self.scene_stack.take());
        let (mut app_window, mut scene_stack) = match zip {
            Some(it) => it,
            None => {
                log::debug!("window event ignored >> the current window or scene stack is empty.");
                return;
            }
        };

        // `winit` 윈도우 식별자가 다른 경우 함수 실행을 생략합니다.
        if window_id != app_window.window.id() {
            // 애플리케이션 창과 장면 스택의 `소유권`을 돌려 놓습니다.
            self.app_window = Some(app_window);
            self.scene_stack = Some(scene_stack);
            return;
        }

        // 게임 장면 스택이 비어있거나, 장면 흐름이 비어있지 않은 경우 함수 실행을 생략합니다.
        if scene_stack.is_empty() || !self.scene_flow.is_empty() {
            // 애플리케이션 창과 장면 스택의 `소유권`을 돌려 놓습니다.
            self.app_window = Some(app_window);
            self.scene_stack = Some(scene_stack);
            return;
        }

        // UI 인터페이스 윈도우를 갱신합니다.
        let _ = app_window
            .egui_state
            .on_window_event(&app_window.window, &event);

        // 윈도우 이벤트를 처리합니다.
        match event {
            WindowEvent::Resized(_) => {
                app_window.on_resized(&self.instance, &self.device);
                let (window_width, window_height) = app_window.window.inner_size().into();
                let content_aspect_ratio = self.window_size.aspect_ratio();
                self.set_viewport_area(window_width, window_height, content_aspect_ratio);
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
                // Safety: 장면 스택이 비어있는지 확인함.
                let current_scene = unsafe { scene_stack.back_mut().unwrap_unchecked() };
                if current_scene.on_close_request(self) {
                    // 장면 스택의 `소유권`을 돌려 놓습니다. (생성된 장면을 정리하기 위함)
                    // 애플리케이션 창의 `소유권`은 돌려 놓지 않습니다. (현재 장면이 없는 경우 종료처리)
                    self.scene_stack = Some(scene_stack);
                    return;
                };
            }
            WindowEvent::Focused(focused) => {
                // 애플리케이션 창의 주목 여부를 설정합니다.
                app_window.focused = focused;
                let window = app_window.window.as_ref();

                // Safety: 장면 스택이 비어있는지 확인함.
                let current_scene = unsafe { scene_stack.back_mut().unwrap_unchecked() };
                if focused {
                    current_scene.on_enter_foreground(window, self);
                } else {
                    current_scene.on_enter_background(window, self);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
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
            }
            WindowEvent::CursorMoved { position, .. } => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
                    #[cfg(target_os = "windows")]
                    if app_window.get_cursor_disabled() {
                        let window = app_window.window.as_ref();
                        let (w, h): (u32, u32) = window.inner_size().into();
                        let _ = window.set_cursor_position(PhysicalPosition::new(w / 2, h / 2));
                    }

                    let (dx, dy): (f32, f32) = self.cursor_delta.unwrap_or_default().into();
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
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
                    if let MouseScrollDelta::LineDelta(dx, dy) = delta {
                        for scene in scene_stack.iter_mut().rev() {
                            if scene.on_mouse_wheel(dx, dy, &app_window.window, self) {
                                break;
                            }
                        }
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
                    if state.is_pressed() {
                        for scene in scene_stack.iter_mut().rev() {
                            if scene.on_mouse_btn_pressed(button, &app_window.window, self) {
                                break;
                            }
                        }
                    } else {
                        for scene in scene_stack.iter_mut().rev() {
                            if scene.on_mouse_btn_released(button, &app_window.window, self) {
                                break;
                            }
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
                // Ui 렌더러의 `소유권`을 가져옵니다.
                let egui_renderer = match self.egui_renderer.take() {
                    Some(it) => it,
                    None => {
                        log::debug!(
                            "drawing event ignored >> the user interface renderer is empty."
                        );
                        // 장면 스택의 `소유권`을 돌려 놓습니다. (생성된 장면을 정리하기 위함)
                        // 애플리케이션 창의 `소유권`은 돌려 놓지 않습니다. (현재 장면이 없는 경우 종료처리)
                        self.scene_stack = Some(scene_stack);
                        return;
                    }
                };

                // egui::RawInput을 가져옵니다.
                let egui_raw_input = app_window.egui_state.take_egui_input(&app_window.window);

                // 게임 장면 그리기를 수행합니다.
                let (app_window, scene_stack, egui_renderer, success) =
                    self.draw(app_window, scene_stack, egui_renderer, egui_raw_input);

                // 게임 장면 그리기에 성공한 경우
                // 애플리케이션 창, 장면 스택, Ui 렌더러의 `소유권`을 돌려 놓습니다.
                if success {
                    self.app_window = Some(app_window);
                    self.scene_stack = Some(scene_stack);
                    self.egui_renderer = Some(egui_renderer);
                }

                return;
            }
            _ => {}
        };

        // 애플리케이션 창, 장면 스택의 `소유권`을 돌려 놓습니다.
        self.app_window = Some(app_window);
        self.scene_stack = Some(scene_stack);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        // 애플리케이션 창과 장면 스택, 그리고 프레임 쉐이더 리소스의 `소유권`을 가져옵니다.
        // 애플리케이션 창 또는 장면 스택 또는 프레임 쉐이더 리소스가 존재하지 않는 경우 함수 실행을 생략합니다.
        let zip = self
            .app_window
            .take()
            .zip(self.scene_stack.take())
            .zip(self.frame_resource.take());
        let ((mut app_window, mut scene_stack), mut frame_resource) = match zip {
            Some(it) => it,
            None => {
                log::debug!("window event ignored >> the current window or scene stack is empty.");
                return;
            }
        };

        // 게임 장면 스택이 비어있는 경우 함수 실행을 생략합니다.
        if scene_stack.is_empty() {
            log::debug!("window event ignored >> the current scene is empty.");
            // 애플리케이션 창, 장면 스택, 그리고 프레임 쉐이더 리소스의 `소유권`을 돌려 놓습니다.
            self.app_window = Some(app_window);
            self.scene_stack = Some(scene_stack);
            self.frame_resource = Some(frame_resource);
            return;
        }

        match event {
            AppEvent::AddGameSceneFlow(flow) => {
                self.scene_flow.push_back(flow);
            }
            AppEvent::ResizeRequest(request_size) => {
                // 요청 해상도가 현재 해상도와 다른 경우
                // 윈도우 크기 변경 이벤트를 수행합니다.
                if self.window_size != request_size {
                    // 프레임 쉐이더 리소스의 크기를 변경합니다.
                    let (content_width, content_height) = request_size.size().into();
                    frame_resource = frame_resource.renew(
                        &self.device,
                        content_width,
                        content_height,
                        SWAPCHAIN_FORMAT,
                        DEPTH_FORMAT,
                    );

                    // 애플리케이션 창이 전체 창이 아닌 경우
                    // 애플리케이션 창을 조정합니다.
                    if !self.fullscreen {
                        match app_window.window.request_inner_size(request_size.size()) {
                            Some(result_size) => {
                                if request_size.size() == result_size {
                                    // 창의 크기가 즉시 적용됐습니다.
                                    self.window_size = request_size;
                                    app_window.on_resized(&self.instance, &self.device);
                                    let (window_width, window_height) = result_size.into();
                                    let content_aspect_ratio = self.window_size.aspect_ratio();
                                    self.set_viewport_area(
                                        window_width,
                                        window_height,
                                        content_aspect_ratio,
                                    );
                                } else {
                                    log::warn!(
                                        "app event ignored >> the current system does not allow resizing the window!"
                                    );
                                }
                            }
                            None => {
                                self.window_size = request_size;
                            }
                        }
                    } else {
                        self.window_size = request_size;
                    }
                }
            }
            AppEvent::FullScreenRequest(fullscreen) => {
                // 요청 전체 화면 여부가 현재 전체 화면 여부와 다른 경우
                // 윈도우 전체 화면 변경 이벤트를 수행합니다.
                if self.fullscreen != fullscreen {
                    self.fullscreen = fullscreen;

                    if self.fullscreen {
                        // 전체 화면 요청인 경우 애플리케이션 창의 크기를 화면 크기로 변경합니다.
                        let window = app_window.window.as_ref();
                        let result = window.current_monitor().map(|monitor| monitor.size());
                        let screen_size = match result {
                            Some(size) => size,
                            None => {
                                log::error!("no available monitors found!");
                                let title = "Window creation failed".into();
                                let message = "No available monitor found!".into();
                                let alert = Alert { title, message };
                                show_error_msg(alert, None);

                                // 장면 스택의 `소유권`을 돌려 놓습니다. (생성된 장면을 정리하기 위함)
                                // 애플리케이션 창의 `소유권`은 돌려 놓지 않습니다. (현재 장면이 없는 경우 종료처리)
                                self.scene_stack = Some(scene_stack);
                                return;
                            }
                        };

                        // 애플리케이션 창의 크기를 변경합니다.
                        match app_window.window.request_inner_size(screen_size) {
                            Some(result_size) => {
                                if screen_size == result_size {
                                    // 창의 크기가 즉시 적용됐습니다.
                                    app_window.on_resized(&self.instance, &self.device);
                                    let (window_width, window_height) = result_size.into();
                                    let content_aspect_ratio = self.window_size.aspect_ratio();
                                    self.set_viewport_area(
                                        window_width,
                                        window_height,
                                        content_aspect_ratio,
                                    );
                                } else {
                                    log::warn!(
                                        "app event ignored >> the current system does not allow resizing the window!"
                                    );
                                }
                            }
                            None => {
                                // 윈도우 이벤트를 통해 창의 크기가 조정됩니다.
                            }
                        }

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
                    } else {
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

                        // 전체 화면에서 창 화면으로 되돌아가는 경우
                        // 프레임 쉐이더 리소스의 크기로 애플리케이션 창의 크기를 변경합니다.
                        match app_window
                            .window
                            .request_inner_size(self.window_size.size())
                        {
                            Some(result_size) => {
                                if self.window_size.size() == result_size {
                                    // 창의 크기가 즉시 적용됐습니다.
                                    app_window.on_resized(&self.instance, &self.device);
                                    let (window_width, window_height) = result_size.into();
                                    let content_aspect_ratio = self.window_size.aspect_ratio();
                                    self.set_viewport_area(
                                        window_width,
                                        window_height,
                                        content_aspect_ratio,
                                    );
                                } else {
                                    log::warn!(
                                        "app event ignored >> the current system does not allow resizing the window!"
                                    );
                                }
                            }
                            None => {
                                // 윈도우 이벤트를 통해 창의 크기가 조정됩니다.
                            }
                        }
                    }
                }
            }
            AppEvent::Alert(alert) => {
                let parent = self
                    .app_window
                    .as_ref()
                    .map(|app_wnd| app_wnd.window.as_ref());
                show_error_msg(alert, parent);

                // 장면 스택의 `소유권`을 돌려 놓습니다. (생성된 장면을 정리하기 위함)
                // 애플리케이션 창의 `소유권`은 돌려 놓지 않습니다. (현재 장면이 없는 경우 종료처리)
                self.scene_stack = Some(scene_stack);
                return;
            }
            AppEvent::NetworkError(error) => {
                // Safety: 장면 스택이 비어있는지 확인함.
                let current_scene = unsafe { scene_stack.back_mut().unwrap_unchecked() };
                current_scene.handle_network_error(error, self);
            }
            AppEvent::PacketReceived(time_stamp, packet) => {
                let mut temp = Some(packet);
                for scene in scene_stack.iter_mut().rev() {
                    if let Some(packet) = temp.take() {
                        temp = scene.on_received_packet(time_stamp, packet, self);
                    }
                }
            }
            AppEvent::CursorDisable => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
                    let window = app_window.window.as_ref();
                    let (w, h): (f64, f64) = window.inner_size().into();
                    let _ = window.set_cursor_position(PhysicalPosition::new(w * 0.5, h * 0.5));
                    self.cursor_delta = None;
                    window.set_cursor_visible(false);
                    window.confine_cursor_to_window(true);
                    app_window.set_cursor_disable(true);
                }
            }
            AppEvent::CursorEnable => {
                // 현재 창이 주목받고 있는 경우 이벤트를 처리합니다.
                if app_window.focused {
                    let window = app_window.window.as_ref();
                    window.set_cursor_visible(true);
                    window.confine_cursor_to_window(false);
                    app_window.set_cursor_disable(false);
                }
            }
        };

        // 애플리케이션 창, 장면 스택, 그리고 프레임 쉐이더 리소스의 `소유권`을 돌려 놓습니다.
        self.app_window = Some(app_window);
        self.scene_stack = Some(scene_stack);
        self.frame_resource = Some(frame_resource);
    }
}

impl AppHandle for Application {
    fn event_loop_proxy(&self) -> &Arc<EventLoopProxy<AppEvent>> {
        &self.event_loop_proxy
    }

    fn io_threads(&self) -> &ThreadPool {
        &self.io_threads
    }

    fn current_dir(&self) -> &Path {
        &self.current_dir
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

    fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    fn timer(&self) -> &GameTimer {
        &self.timer
    }

    fn audio_mixer(&self) -> &Mixer {
        self.stream_handle.mixer()
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
}

/// 모든 게임 장면을 제거합니다.
fn clear_scene(
    ui_renderer: &mut UiRenderer,
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: Option<&Window>,
    app: &dyn AppHandle,
) {
    while let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(window, app, ui_renderer);
    }
}

/// 모든 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
fn reset_scene(
    ui_renderer: &mut UiRenderer,
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: &Window,
    app: &dyn AppHandle,
    mut new_scene: Box<dyn GameScene>,
) {
    while let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(Some(window), app, ui_renderer);
    }

    log::info!("Enter GameScene({:?})", &new_scene);
    new_scene.on_enter(window, app, ui_renderer);
    stack.push_back(new_scene);
}

/// 현재 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
fn change_scene(
    ui_renderer: &mut UiRenderer,
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: &Window,
    app: &dyn AppHandle,
    mut new_scene: Box<dyn GameScene>,
) {
    if let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(Some(window), app, ui_renderer);
    }

    log::info!("Enter GameScene({:?})", &new_scene);
    new_scene.on_enter(window, app, ui_renderer);
    stack.push_back(new_scene);
}

/// 새로운 게임 장면을 초기화하고, 추가합니다.
fn push_scene(
    ui_renderer: &mut UiRenderer,
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
    new_scene.on_enter(window, app, ui_renderer);
    stack.push_back(new_scene);
}

/// 현재 장면을 정리하고, 제거합니다.
fn pop_scene(
    ui_renderer: &mut UiRenderer,
    stack: &mut VecDeque<Box<dyn GameScene>>,
    window: &Window,
    app: &dyn AppHandle,
) {
    if let Some(mut scene) = stack.pop_back() {
        log::info!("Exit GameScene({:?})", &scene);
        scene.on_exit(Some(window), app, ui_renderer);
    }
    if let Some(scene) = stack.back_mut() {
        log::info!("Resume GameScene({:?})", &scene);
        scene.on_resume(window, app);
    }
}
