use std::{
    error::Error,
    fmt,
    io::{Cursor, ErrorKind},
    sync::Arc,
};

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, NoSuitableWndSize, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use winit::window::Window;

use crate::config::{InvalidConfig, UserConfig};

use super::TestbedTitleScene;

/// ## Startup Scene
/// 게임을 실행하면 제일 먼저 진입하는 장면입니다.
///
/// `UserConfig` 파일을 읽고, 애플리케이션 창을 조정합니다.  
/// 시스템에서 파일을 찾을 수 없는 경우 초기 설정 장면으로 전환합니다.
///
pub struct StartupScene {
    /// 사용자 구성 데이터
    user_config: Option<Box<UserConfig>>,

    /// 작업 결과물 대기열
    queue: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,

    /// 남은 작업의 개수
    num_tasks: usize,
}

impl StartupScene {
    pub fn new() -> Self {
        Self {
            user_config: None,
            queue: Arc::new(Queue::new()),
            num_tasks: 0,
        }
    }
}

impl GameScene for StartupScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 유저 구성 설정 파일을 로드하고, IO 스레드 풀에서 기본 에셋을 로드합니다.
        //!
        let pool = app.io_threads();
        let queue = self.queue.clone();
        let asset_manager = app.asset_manager().clone();
        pool.spawn(move || {
            // `NEXON LV2 gothic` 폰트를 로드합니다.
            let result = asset_manager
                .load("font/NEXON_Lv2_Gothic.ttf")
                .map(|_| {})
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            queue.push(result);
        });
        self.num_tasks += 1;

        // 유저 구성 설정 파일을 읽습니다.
        let asset_manager = app.asset_manager();
        let result = asset_manager.get_or_init("user_config");
        let user_config = Box::new(match result {
            // 유저 구성 설정 파일이 존재하는 경우
            Ok(cached_asset) => {
                let reader = Cursor::new(cached_asset.as_bytes());
                let config: UserConfig = serde_json::from_reader(reader)
                    .map_err(|e| InvalidConfig(e))
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

                config
            }
            // 유저 구성 설정 파일이 존재하지 않는 경우: 기본 설정 파일을 생성한다.
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // 최대 윈도우 크기를 가져옵니다.
                let max_window_size = window
                    .primary_monitor()
                    .map(|monitor| WindowSize::find_maximize_size(monitor))
                    .flatten();
                let max_window_size = match max_window_size {
                    Some(size) => size,
                    None => return Err(Box::new(NoSuitableWndSize)),
                };

                // 사용자 구성 파일을 생성합니다.
                let config = UserConfig::new(max_window_size);
                let data = serde_json::ser::to_vec_pretty(&config)
                    .map_err(|e| InvalidConfig(e))
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
                asset_manager
                    .create("user_config", &data)
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

                config
            }
            Err(e) => return Err(Box::new(e)),
        });

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
        let egui_ctx = app.egui_ctx();
        let asset_manager = app.asset_manager();
        let nexon_lv2_gothic = asset_manager
            .get_or_init("font/NEXON_Lv2_Gothic.ttf")
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // UI 폰트 데이터를 추가합니다.
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NEXON_Lv2_Gothic".to_owned(),
            egui::FontData::from_owned(nexon_lv2_gothic.as_bytes().to_vec()).into(),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "NEXON_Lv2_Gothic".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("NEXON_Lv2_Gothic".to_owned());

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
        // 이미 다른 장면으로 전환된 경우 함수 실행을 생략합니다.
        if self.user_config.is_none() {
            return Ok(());
        }

        // IO 스레드 풀 작업 결과를 기다립니다.
        if let Some(result) = self.queue.pop() {
            self.num_tasks -= 1;
            result?;
        }

        // 모든 작업이 완료된 경우 다음 장면으로 전환합니다.
        if self.num_tasks == 0 {
            let proxy = app.event_loop_proxy();
            proxy
                .send_event(AppEvent::SetGameSceneFlow(GameSceneFlow::Change(Box::new(
                    TestbedTitleScene::new(
                        self.user_config
                            .take()
                            .expect("user configuration must exist"),
                    ),
                ))))
                .unwrap();
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
