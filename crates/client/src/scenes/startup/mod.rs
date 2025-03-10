mod init;

use std::{error::Error, path::PathBuf, sync::Arc};

use ahash::HashMap;
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rayon::ThreadPool;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
    asset::{NEXON_LV2_GOTHIC, NEXON_LV2_GOTHIC_BOLD, USER_CONFIG},
    config::UserConfig,
};

pub use self::init::*;

use super::GameIntroNotifyScene;

/// 클라이언트 실행시 가장 첫 번째로 진입하는 게임 장면입니다.  
/// 게임 전반적으로 사용되는 에셋이나, 사용자 구성 설정을 로드합니다.
pub struct GameStartupScene {
    /// 초기 설정이 필요한지 여부
    needs_initial_setup: bool,

    /// 남은 작업의 개수
    remaining_task_count: usize,
    /// 작업 결과를 저장하는 대기열
    task_results: Arc<Queue<Result<(String, Vec<u8>), Box<dyn Error + Send>>>>,
    /// 로드된 에셋 데이터 집합
    raw_asset_data: HashMap<String, Vec<u8>>,
}

impl GameStartupScene {
    /// 새로운 `GameStartupScene`을 생성합니다.
    pub fn new() -> Self {
        Self {
            needs_initial_setup: false,
            remaining_task_count: 0,
            task_results: Arc::new(Queue::new()),
            raw_asset_data: HashMap::default(),
        }
    }

    /// `NEXON Lv2 Gothic` 폰트를 로드합니다.
    fn load_nexon_lv2_gothic_font(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
    ) {
        // 스레드 풀에서 에셋을 로드합니다.
        let asset_manager = asset_manager.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            // 에셋 데이터를 로드합니다.
            let result = asset_manager
                .load(NEXON_LV2_GOTHIC)
                .map(|asset| {
                    (
                        asset.filename().to_string_lossy().into_owned(),
                        asset.as_bytes().to_vec(),
                    )
                })
                .map_err(|e| {
                    log::error!("failed to load asset! (REASON:{e})");
                    Box::new(e) as Box<dyn Error + Send>
                });
            // 남은 에셋 데이터를 제거합니다.
            asset_manager.remove(NEXON_LV2_GOTHIC);
            // 결과를 전송합니다.
            task_results.push(result);
        });

        // 남은 작업의 수를 증가시킵니다.
        self.remaining_task_count += 1;
    }

    /// `NEXON Lv2 Gothic Bold` 폰트를 로드합니다.
    fn load_nexon_lv2_gothic_blod_font(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
    ) {
        // 스레드 풀에서 에셋을 로드합니다.
        let asset_manager = asset_manager.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            // 에셋 데이터를 로드합니다.
            let result = asset_manager
                .load(NEXON_LV2_GOTHIC_BOLD)
                .map(|asset| {
                    (
                        asset.filename().to_string_lossy().into_owned(),
                        asset.as_bytes().to_vec(),
                    )
                })
                .map_err(|e| {
                    log::error!("failed to load asset! (REASON:{e})");
                    Box::new(e) as Box<dyn Error + Send>
                });
            // 남은 에셋 데이터를 제거합니다.
            asset_manager.remove(NEXON_LV2_GOTHIC_BOLD);
            // 결과를 전송합니다.
            task_results.push(result);
        });

        // 남은 작업의 수를 증가시킵니다.
        self.remaining_task_count += 1;
    }

    /// 사용자 구성 파일을 로드합니다.
    fn load_user_config<P>(&mut self, root_dir: P)
    where
        P: Into<PathBuf>,
    {
        // 사용자 구성 파일의 경로를 생성합니다.
        let mut path: PathBuf = root_dir.into();
        path.push(USER_CONFIG);

        // 사용자 구성 파일을 로드합니다.
        self.needs_initial_setup = UserConfig::load_from_file(path).is_err();
    }

    /// 폰트를 초기화합니다.
    fn setup_custom_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // `NEXON Lv2 Gothic` 폰트를 추가합니다.
        let font = self
            .raw_asset_data
            .remove(NEXON_LV2_GOTHIC)
            .expect("font data is empty!");
        fonts.font_data.insert(
            NEXON_LV2_GOTHIC.to_owned(),
            egui::FontData::from_owned(font).into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name(NEXON_LV2_GOTHIC.into()),
            vec![NEXON_LV2_GOTHIC.into()],
        );

        // `NEXON Lv2 Gothic Bold` 폰트를 추가합니다.
        let font = self
            .raw_asset_data
            .remove(NEXON_LV2_GOTHIC_BOLD)
            .expect("font data is empty!");
        fonts.font_data.insert(
            NEXON_LV2_GOTHIC_BOLD.to_owned(),
            egui::FontData::from_owned(font).into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name(NEXON_LV2_GOTHIC_BOLD.into()),
            vec![NEXON_LV2_GOTHIC_BOLD.into()],
        );

        // 폰트 설정을 저장합니다.
        ctx.set_fonts(fonts);
    }

    /// 애플리케이션 창의 설정을 변경합니다.
    fn change_window_config(&self, window: &Window, event_loop_proxy: &EventLoopProxy<AppEvent>) {
        // 애플리케이션 창의 최대 크기를 구합니다.
        let max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor))
            .flatten()
            .unwrap_or(WindowSize::MAX);

        // 사용자 설정을 변경합니다.
        let mut config = UserConfig::get();
        config.window_size = config.window_size.min(max_window_size);

        // 애플리케이션 창의 설정을 변경합니다.
        let event = AppEvent::ResizeRequest(config.window_size);
        event_loop_proxy.send_event(event).unwrap();
        let event = AppEvent::FullScreenRequest(config.is_fullscreen);
        event_loop_proxy.send_event(event).unwrap();
    }
}

impl GameScene for GameStartupScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let thread_pool = app.io_threads();
        let asset_manager = app.asset_manager();
        self.load_nexon_lv2_gothic_font(thread_pool, asset_manager);
        self.load_nexon_lv2_gothic_blod_font(thread_pool, asset_manager);
        self.load_user_config(asset_manager.get_root_dir());
        Ok(())
    }

    fn on_exit(
        &mut self,
        window: Option<&Window>,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.setup_custom_fonts(app.egui_ctx());
        if let Some(window) = window {
            self.change_window_config(window, app.event_loop_proxy());
        }
        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            self.remaining_task_count -= 1;
            let (key, value) = result?;
            self.raw_asset_data.insert(key, value);
        }

        // 모든 작업이 완료된 경우 다음 게임 장면으로 전환합니다.
        if self.remaining_task_count == 0 {
            let next_scene: Box<dyn GameScene> = match self.needs_initial_setup {
                true => Box::new(InitLocaleScene::new()),
                false => Box::new(GameIntroNotifyScene::new()),
            };
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        Ok(())
    }

    fn on_draw(
        &self,
        _window: &Window,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임을 초기화 하는 동안 검정색 화면을 출력합니다.
        //
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({})", stringify!(GameStartupScene))),
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
