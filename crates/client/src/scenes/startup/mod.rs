mod init;

use std::{error::Error, io::Cursor, path::PathBuf, sync::Arc};

use ahash::HashMap;
use image::{ImageFormat, ImageReader};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use mod_render::TexturePool;
use rayon::ThreadPool;
use wgpu::util::DeviceExt;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
    asset::{
        BG_LOGIN_TITLE_0_DATA, BG_LOGIN_TITLE_0_URI, BG_LOGIN_TITLE_1_DATA, BG_LOGIN_TITLE_1_URI,
        BG_LOGIN_TITLE_2_DATA, BG_LOGIN_TITLE_2_URI, BG_LOGIN_TITLE_3_DATA, BG_LOGIN_TITLE_3_URI,
        BG_LOGIN_TITLE_4_DATA, BG_LOGIN_TITLE_4_URI, BG_LOGIN_TITLE_5_DATA, BG_LOGIN_TITLE_5_URI,
        GAME_LOGO_DATA, GAME_LOGO_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR, USER_CONFIG,
    },
    config::UserConfig,
};

pub use self::init::*;

use super::GameIntroNotifyScene;

/// 작업 결과 목록입니다.
#[derive(Debug)]
enum TaskResult {
    Font { uri: String, bytes: Vec<u8> },
    Texture,
}

/// 클라이언트 실행시 가장 첫 번째로 진입하는 게임 장면입니다.  
/// 게임 전반적으로 사용되는 에셋이나, 사용자 구성 설정을 로드합니다.
pub struct GameStartupScene {
    /// 초기 설정이 필요한지 여부
    needs_initial_setup: bool,

    /// 남은 작업의 개수
    remaining_task_count: usize,
    /// 작업 결과를 저장하는 대기열
    task_results: Arc<Queue<Result<TaskResult, Box<dyn Error + Send>>>>,
    /// 로드된 폰트 에셋 데이터 집합
    font_asset_data: HashMap<String, Vec<u8>>,
}

impl GameStartupScene {
    /// 새로운 `GameStartupScene`을 생성합니다.
    pub fn new() -> Self {
        Self {
            needs_initial_setup: false,
            remaining_task_count: 0,
            task_results: Arc::new(Queue::new()),
            font_asset_data: HashMap::default(),
        }
    }

    /// `NotoSans-Regular` 폰트를 로드합니다.
    fn load_notosans_regular_font(
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
                .load(NOTOSANS_REGULAR)
                .map(|asset| TaskResult::Font {
                    uri: NOTOSANS_REGULAR.into(),
                    bytes: asset.as_bytes().to_vec(),
                })
                .map_err(|e| {
                    log::error!("failed to load asset! (REASON:{e})");
                    Box::new(e) as Box<dyn Error + Send>
                });

            // 남은 에셋 데이터를 제거합니다.
            asset_manager.remove(NOTOSANS_REGULAR);
            // 결과를 전송합니다.
            task_results.push(result);
        });

        // 남은 작업의 수를 증가시킵니다.
        self.remaining_task_count += 1;
    }

    /// `NotoSans-Bold` 폰트를 로드합니다.
    fn load_notosans_blod_font(&mut self, thread_pool: &ThreadPool, asset_manager: &AssetManager) {
        // 스레드 풀에서 에셋을 로드합니다.
        let asset_manager = asset_manager.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            // 에셋 데이터를 로드합니다.
            let result = asset_manager
                .load(NOTOSANS_BOLD)
                .map(|asset| TaskResult::Font {
                    uri: NOTOSANS_BOLD.into(),
                    bytes: asset.as_bytes().to_vec(),
                })
                .map_err(|e| {
                    log::error!("failed to load asset! (REASON:{e})");
                    Box::new(e) as Box<dyn Error + Send>
                });

            // 남은 에셋 데이터를 제거합니다.
            asset_manager.remove(NOTOSANS_BOLD);
            // 결과를 전송합니다.
            task_results.push(result);
        });

        // 남은 작업의 수를 증가시킵니다.
        self.remaining_task_count += 1;
    }

    /// 텍스처를 디코드하고, 텍스처 풀 객체에 등록합니다.
    fn regist_texture(
        &mut self,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
        uri: &'static str,
        bytes: &'static [u8],
    ) {
        let task_results = self.task_results.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            // 텍스처 데이터를 디코딩합니다.
            let pixels = Cursor::new(bytes);
            let mut reader = ImageReader::new(pixels);
            reader.set_format(ImageFormat::Png);

            let image = match reader.decode() {
                Ok(image) => image,
                Err(e) => {
                    log::error!("failed to load texture! (REASON:{e}");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스처를 생성합니다.
            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", &uri)),
                    size: wgpu::Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    dimension: wgpu::TextureDimension::D2,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::default(),
                &image.to_rgba8(),
            );

            // 텍스처 풀 객체에 등록합니다.
            TexturePool::register(uri.into(), texture.into());

            // 결과를 전송합니다.
            task_results.push(Ok(TaskResult::Texture));
        });
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
            .font_asset_data
            .remove(NOTOSANS_REGULAR)
            .expect("font data is empty!");
        fonts.font_data.insert(
            NOTOSANS_REGULAR.to_owned(),
            egui::FontData::from_owned(font).into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name(NOTOSANS_REGULAR.into()),
            vec![NOTOSANS_REGULAR.into()],
        );

        // `NEXON Lv2 Gothic Bold` 폰트를 추가합니다.
        let font = self
            .font_asset_data
            .remove(NOTOSANS_BOLD)
            .expect("font data is empty!");
        fonts.font_data.insert(
            NOTOSANS_BOLD.to_owned(),
            egui::FontData::from_owned(font).into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name(NOTOSANS_BOLD.into()),
            vec![NOTOSANS_BOLD.into()],
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
        let device = app.render_device();
        let queue = app.render_queue();
        let thread_pool = app.io_threads();
        let asset_manager = app.asset_manager();
        self.load_notosans_regular_font(thread_pool, asset_manager);
        self.load_notosans_blod_font(thread_pool, asset_manager);
        self.regist_texture(thread_pool, device, queue, GAME_LOGO_URI, GAME_LOGO_DATA);
        self.regist_texture(
            thread_pool,
            device,
            queue,
            BG_LOGIN_TITLE_0_URI,
            BG_LOGIN_TITLE_0_DATA,
        );
        self.regist_texture(
            thread_pool,
            device,
            queue,
            BG_LOGIN_TITLE_1_URI,
            BG_LOGIN_TITLE_1_DATA,
        );
        self.regist_texture(
            thread_pool,
            device,
            queue,
            BG_LOGIN_TITLE_2_URI,
            BG_LOGIN_TITLE_2_DATA,
        );
        self.regist_texture(
            thread_pool,
            device,
            queue,
            BG_LOGIN_TITLE_3_URI,
            BG_LOGIN_TITLE_3_DATA,
        );
        self.regist_texture(
            thread_pool,
            device,
            queue,
            BG_LOGIN_TITLE_4_URI,
            BG_LOGIN_TITLE_4_DATA,
        );
        self.regist_texture(
            thread_pool,
            device,
            queue,
            BG_LOGIN_TITLE_5_URI,
            BG_LOGIN_TITLE_5_DATA,
        );
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
            match result? {
                TaskResult::Font { uri, bytes } => {
                    self.font_asset_data.insert(uri, bytes);
                }
                TaskResult::Texture => {}
            }
        }

        // 모든 작업이 완료된 경우 다음 게임 장면으로 전환합니다.
        if self.remaining_task_count == 0 {
            let next_scene: Box<dyn GameScene> = match self.needs_initial_setup {
                true => Box::new(InitLocaleScene::new()),
                false => {
                    let config = UserConfig::get();
                    Box::new(GameIntroNotifyScene::new(config.locale))
                }
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
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임을 초기화 하는 동안 검정색 화면을 출력합니다.
        //
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

        Ok(())
    }
}
