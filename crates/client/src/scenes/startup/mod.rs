//! 애플리케이션의 최초 진입 장면입니다.
//! 게임에서 사용되는 필수 리소스를 로드합니다.
//!

mod init;

use std::{
    error::Error,
    fs::OpenOptions,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use ahash::HashMap;
use image::{ImageFormat, ImageReader};
use mod_app::{
    app::AppHandle,
    error::Alert,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use mod_render::{UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT};
use rayon::ThreadPool;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
    asset::{
        TexturePool, BG_GROWTH_EFFECT_LABEL_DATA, BG_GROWTH_EFFECT_LABEL_URI,
        BG_LOGIN_TITLE_0_DATA, BG_LOGIN_TITLE_0_URI, BG_LOGIN_TITLE_1_DATA, BG_LOGIN_TITLE_1_URI,
        BG_LOGIN_TITLE_2_DATA, BG_LOGIN_TITLE_2_URI, BG_LOGIN_TITLE_3_DATA, BG_LOGIN_TITLE_3_URI,
        BG_LOGIN_TITLE_4_DATA, BG_LOGIN_TITLE_4_URI, BG_LOGIN_TITLE_5_DATA, BG_LOGIN_TITLE_5_URI,
        GAME_LOGO_DATA, GAME_LOGO_URI, HUD_CANCEL_ICON_DATA, HUD_CANCEL_ICON_URI,
        HUD_EXIT_ICON_DATA, HUD_EXIT_ICON_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR, USER_CONFIG,
    },
    component::{
        BulletRenderPipeline, BulletRenderPipelineTransparency, CharacterBakePipeline,
        CharacterRenderPipeline, DamageFontRenderPipeline, EnergyBulletRenderPipeline,
        EyeMouthBakePipeline, EyeMouthRenderPipeline, HaloRenderPipeline, SkyboxRenderPipeline,
        StageBakePipeline, StageRenderPipeline, TreeRenderPipeline, SHADOW_FORMAT,
    },
    config::UserConfig,
};

pub use self::init::*;

use super::GameIntroNotifyScene;

/// 작업 결과 목록입니다.
#[derive(Debug)]
enum TaskResult {
    Font {
        uri: String,
        bytes: Vec<u8>,
    },
    Texture {
        command: wgpu::CommandBuffer,
        staging_buffers: Vec<wgpu::Buffer>,
    },
    Pipeline,
}

/// 클라이언트 실행시 가장 첫 번째로 진입하는 게임 장면입니다.  
/// 게임 전반적으로 사용되는 에셋이나, 사용자 구성 설정을 로드합니다.
pub struct GameStartupScene {
    /// 초기 설정이 필요한지 여부
    needs_initial_setup: bool,

    /// 남은 작업의 개수
    num_remaining_tasks: usize,
    /// 스테이징(업로드) 버퍼의 집합
    staging_buffers: Vec<wgpu::Buffer>,
    /// 작업 결과를 저장하는 대기열
    task_results: Arc<Queue<Result<TaskResult, Box<dyn Error + Send>>>>,
    /// 로드된 폰트 에셋 데이터 집합
    font_asset_data: HashMap<String, Vec<u8>>,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl GameStartupScene {
    /// 새로운 `GameStartupScene`을 생성합니다.
    pub fn new() -> Self {
        Self {
            needs_initial_setup: false,
            num_remaining_tasks: 0,
            staging_buffers: Vec::default(),
            task_results: Arc::new(Queue::new()),
            font_asset_data: HashMap::default(),
            texture_pool: TexturePool::new(),
        }
    }

    /// 렌더링 파이프라인을 초기화합니다.
    fn init_render_pipeline(&mut self, thread_pool: &ThreadPool, device: &Arc<wgpu::Device>) {
        // 일반 총알을 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            BulletRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 일반 총알을 투명하게 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            BulletRenderPipelineTransparency::get_or_init(&device_cloned, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 에너지 볼 형태의 총알을 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            EnergyBulletRenderPipeline::get_or_init(&device_cloned, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 캐릭터를 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            CharacterRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 캐릭터의 그림자를 생성하는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            CharacterBakePipeline::get_or_init(&device_cloned, SHADOW_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 캐릭터 눈과 입을 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            EyeMouthRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 캐릭터 눈과 입의 그림자를 생성하는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            EyeMouthBakePipeline::get_or_init(&device_cloned, SHADOW_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 캐릭터 헤일로를 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            HaloRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 데미지 폰트를 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            DamageFontRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 스카이 박스를 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            SkyboxRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 지형을 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            StageRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 나무를 그리는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            TreeRenderPipeline::get_or_init(&device_cloned, SWAPCHAIN_FORMAT, DEPTH_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;

        // 지형의 그림자를 생성하는 렌더링 파이프라인을 생성합니다.
        let device_cloned = device.clone();
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            StageBakePipeline::get_or_init(&device_cloned, SHADOW_FORMAT);
            task_results.push(Ok(TaskResult::Pipeline));
        });
        self.num_remaining_tasks += 1;
    }

    /// `NotoSans-Regular` 폰트를 로드합니다.
    fn load_notosans_regular_font<Dir>(&mut self, thread_pool: &ThreadPool, root_dir: Dir)
    where
        Dir: Into<PathBuf>,
    {
        let mut path: PathBuf = root_dir.into();
        path.push(format!("font/{}", NOTOSANS_REGULAR));

        // 스레드 풀에서 에셋을 로드합니다.
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            log::debug!("open font asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to open font asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read font asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read font asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close font asset (PATH:{})", path.display());
            drop(file);

            // 결과를 전송합니다.
            task_results.push(Ok(TaskResult::Font {
                uri: NOTOSANS_REGULAR.into(),
                bytes: buf,
            }));
        });

        // 남은 작업의 수를 증가시킵니다.
        self.num_remaining_tasks += 1;
    }

    /// `NotoSans-Bold` 폰트를 로드합니다.
    fn load_notosans_blod_font<Dir>(&mut self, thread_pool: &ThreadPool, root_dir: Dir)
    where
        Dir: Into<PathBuf>,
    {
        let mut path: PathBuf = root_dir.into();
        path.push(format!("font/{}", NOTOSANS_BOLD));

        // 스레드 풀에서 에셋을 로드합니다.
        let task_results = self.task_results.clone();
        thread_pool.spawn(move || {
            log::debug!("open font asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to open font asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read font asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read font asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close font asset (PATH:{})", path.display());
            drop(file);

            // 결과를 전송합니다.
            task_results.push(Ok(TaskResult::Font {
                uri: NOTOSANS_BOLD.into(),
                bytes: buf,
            }));
        });

        // 남은 작업의 수를 증가시킵니다.
        self.num_remaining_tasks += 1;
    }

    /// 텍스처를 디코드하고, 텍스처 객체를 생성합니다.
    fn create_texture(
        &mut self,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
        uri: &'static str,
        bytes: &'static [u8],
    ) {
        let texture_pool = self.texture_pool.clone();
        let task_results = self.task_results.clone();
        let device = device.clone();
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

            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // 텍스처를 생성합니다.
            let texture = TexturePool::create_texture(
                &format!("Texture({})", &uri),
                &device,
                &mut encoder,
                &mut staging_buffers,
                image.width(),
                image.height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
                1,
                image.to_rgba8().to_vec(),
            );

            // 텍스처 풀 객체에 등록합니다.
            texture_pool.insert(uri, texture.into());

            // 결과를 전송합니다.
            task_results.push(Ok(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            }));
        });
        self.num_remaining_tasks += 1;
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

        // `Notosans` 폰트를 추가합니다.
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

        // `Notosans Bold` 폰트를 추가합니다.
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
            .map(|monitor| WindowSize::find_maximize_size(monitor.size()))
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
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        let thread_pool = app.io_threads();
        let mut root_dir = app.current_dir().to_path_buf();
        root_dir.push("assets");
        self.init_render_pipeline(thread_pool, device);
        self.load_notosans_regular_font(thread_pool, &root_dir);
        self.load_notosans_blod_font(thread_pool, &root_dir);
        self.create_texture(thread_pool, device, GAME_LOGO_URI, GAME_LOGO_DATA);
        self.create_texture(thread_pool, device, HUD_EXIT_ICON_URI, HUD_EXIT_ICON_DATA);
        self.create_texture(
            thread_pool,
            device,
            HUD_CANCEL_ICON_URI,
            HUD_CANCEL_ICON_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_LOGIN_TITLE_0_URI,
            BG_LOGIN_TITLE_0_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_LOGIN_TITLE_1_URI,
            BG_LOGIN_TITLE_1_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_LOGIN_TITLE_2_URI,
            BG_LOGIN_TITLE_2_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_LOGIN_TITLE_3_URI,
            BG_LOGIN_TITLE_3_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_LOGIN_TITLE_4_URI,
            BG_LOGIN_TITLE_4_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_LOGIN_TITLE_5_URI,
            BG_LOGIN_TITLE_5_DATA,
        );
        self.create_texture(
            thread_pool,
            device,
            BG_GROWTH_EFFECT_LABEL_URI,
            BG_GROWTH_EFFECT_LABEL_DATA,
        );
        self.load_user_config(root_dir);
    }

    fn on_exit(
        &mut self,
        window: Option<&Window>,
        app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        self.setup_custom_fonts(app.egui_ctx());
        if let Some(window) = window {
            self.change_window_config(window, app.event_loop_proxy());
        }
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            match result {
                Ok(task) => {
                    self.num_remaining_tasks -= 1;
                    log::info!(
                        "task success (number of tasks remaining: {})",
                        self.num_remaining_tasks
                    );

                    match task {
                        TaskResult::Font { uri, bytes } => {
                            self.font_asset_data.insert(uri, bytes);
                        }
                        TaskResult::Texture {
                            command,
                            mut staging_buffers,
                        } => {
                            app.render_queue().submit(Some(command));
                            self.staging_buffers.append(&mut staging_buffers);
                        }
                        _ => {}
                    };
                }
                Err(_) => {
                    let title = "Initialize failed".into();
                    let message = "Failed to initialize game data.".into();
                    let alert = Alert { title, message };
                    let event = AppEvent::Alert(alert);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
            }
        }

        // 모든 작업이 완료된 경우 다음 게임 장면으로 전환합니다.
        if self.num_remaining_tasks == 0 {
            let next_scene: Box<dyn GameScene> = match self.needs_initial_setup {
                true => Box::new(InitLocaleScene::new(self.texture_pool.clone())),
                false => {
                    let config = UserConfig::get();
                    Box::new(GameIntroNotifyScene::new(
                        config.locale,
                        self.texture_pool.clone(),
                    ))
                }
            };
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({:?})", &self)),
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
    }
}
