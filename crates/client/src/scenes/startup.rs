use std::{
    error::Error,
    fmt,
    io::{Cursor, ErrorKind},
};

use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    channel::TaskResultChannel,
    config::{InvalidConfig, UserConfig},
    FONT_STYLE_0, FONT_STYLE_0_BOLD, USER_CONFIG,
};

use super::IntroScene;

/// ## Startup Scene
/// 게임을 실행하면 제일 먼저 진입하는 장면입니다.
///
/// 1. `UserConfig` 파일을 읽고, 애플리케이션 창을 조정합니다.  
/// 시스템에서 파일을 찾을 수 없는 경우 초기 설정 장면으로 전환합니다.
///
/// 2. 시스템 기본 구성 리소스를 로드합니다. (예: 폰트, 자주 사용되는 텍스처, 등)
///
pub struct StartupScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,

    /// 작업 결과 채널
    task_result_channel: TaskResultChannel<()>,

    /// 남은 작업의 개수
    num_tasks: usize,
}

impl StartupScene {
    pub fn new() -> Self {
        Self {
            user_config: None,
            task_result_channel: TaskResultChannel::new(),
            num_tasks: 0,
        }
    }
}

impl GameScene for StartupScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 유저 구성 설정 파일을 로드하고, IO 스레드 풀에서 기본 에셋을 로드합니다.
        //!
        let pool = app.io_threads();
        let channel = self.task_result_channel.clone();
        let asset_manager = app.asset_manager().clone();
        preload_font_style_0(pool, channel, asset_manager);
        self.num_tasks += 1;

        let channel = self.task_result_channel.clone();
        let asset_manager = app.asset_manager().clone();
        preload_font_style_0_bold(pool, channel, asset_manager);
        self.num_tasks += 1;

        // 사용자 구성 설정 파일을 로드합니다.
        let user_config = load_user_configuration(app.asset_manager())?;
        let user_config = Box::new(user_config);

        // 애플리케이션 창을 조정합니다.
        let proxy = app.event_loop_proxy();
        proxy
            .send_event(AppEvent::ResizeRequest(user_config.window_size))
            .unwrap();
        proxy
            .send_event(AppEvent::FullScreenRequest(user_config.fullscreen))
            .unwrap();

        self.user_config = Some(user_config);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_exit(
        &mut self,
        window: Option<&Window>,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 로드된 폰트를 UI 시스템에서 사용할 수 있도록 초기화합니다.
        //!

        // 로드된 폰트 에셋 데이터를 가져옵니다.
        let egui_ctx = app.egui_ctx();
        let asset_manager = app.asset_manager();
        let font_style_0 = asset_manager
            .get_or_init(FONT_STYLE_0)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        let font_style_0_bold = asset_manager
            .get_or_init(FONT_STYLE_0_BOLD)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // UI 폰트 데이터를 생성합니다.
        let font_style_0 = egui::FontData::from_owned(font_style_0.as_bytes().to_vec());
        let font_style_0_bold = egui::FontData::from_owned(font_style_0_bold.as_bytes().to_vec());

        // UI 폰트 데이터를 추가합니다.
        let mut fonts = egui::FontDefinitions::default();
        let font_data = &mut fonts.font_data;
        font_data.insert(FONT_STYLE_0.to_owned(), font_style_0.into());
        font_data.insert(FONT_STYLE_0_BOLD.to_owned(), font_style_0_bold.into());

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, FONT_STYLE_0.to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(FONT_STYLE_0.to_owned());

        egui_ctx.set_fonts(fonts);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 기다립니다.
        if let Some(result) = self.task_result_channel.recv() {
            self.num_tasks -= 1;
            result?;
        }

        // 모든 작업이 끝난 경우 다음 게임 장면으로 전환합니다.
        if self.num_tasks == 0 {
            if let Some(user_config) = self.user_config.take() {
                let next_scene = Box::new(IntroScene::new(user_config));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let proxy = app.event_loop_proxy();
                proxy.send_event(event).unwrap();
            }
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_stencil_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 게임을 초기화 하는 동안 검정색 화면을 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(StartupScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }
}

impl fmt::Debug for StartupScene {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(StartupScene))
    }
}

/// 주어진 스레드 풀에서 "font:style_0" 에셋을 로드합니다.
/// 에셋의 로드 결과를 주어진 작업 결과 채널로 전송합니다.
fn preload_font_style_0(
    pool: &ThreadPool,
    channels: TaskResultChannel<()>,
    asset_manager: AssetManager,
) {
    pool.spawn(move || {
        let result = asset_manager.load(FONT_STYLE_0);
        channels.send(result.map(|_| ()));
    });
}

/// 주어진 스레드 풀에서 "font:style_0_bold" 에셋을 로드합니다.
/// 에셋의 로드 결과를 주어진 작업 결과 채널로 전송합니다.
fn preload_font_style_0_bold(
    pool: &ThreadPool,
    channel: TaskResultChannel<()>,
    asset_manager: AssetManager,
) {
    pool.spawn(move || {
        let result = asset_manager.load(FONT_STYLE_0_BOLD);
        channel.send(result.map(|_| ()));
    });
}

/// 사용자 구성 설정 파일을 로드합니다.
fn load_user_configuration(
    asset_manager: &AssetManager,
) -> Result<UserConfig, Box<dyn Error + Send>> {
    let result = asset_manager.get_or_init(USER_CONFIG);
    match result {
        Ok(cached) => {
            // 사용자 구성 파일 데이터를 구문 분석합니다.
            let reader = Cursor::new(cached.as_bytes());
            let result: Result<UserConfig, _> = serde_json::from_reader(reader);
            result
                .map_err(|e| InvalidConfig(e))
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // 사용자 구성 파일을 생성합니다.
            let user_config = UserConfig::new();
            let data = serde_json::ser::to_vec_pretty(&user_config)
                .map_err(|e| InvalidConfig(e))
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
            asset_manager
                .create(USER_CONFIG, &data)
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

            Ok(user_config)
        }
        Err(e) => Err(Box::new(e)),
    }
}
