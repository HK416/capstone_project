use std::{
    error::Error,
    fs::OpenOptions,
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use image::{ImageFormat, ImageReader};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{CharacterKind, GameTier, LoginToken, UserId, UserName};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::{TexturePool, BG_MAIN_LOBBY_URI, NOTOSANS_BOLD},
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

use super::MainLobbyScene;

/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["오류"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["게임 리소스를 로드하는데 실패했습니다!"];

/// 작업 결과 목록입니다.
#[derive(Debug)]
enum TaskResult {
    Texture {
        command: wgpu::CommandBuffer,
        staging_buffers: Vec<wgpu::Buffer>,
    },
}

pub struct MainLobbyEnterScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 사용자 이름 (게임 장면이 유지되는 동안 존재합니다)
    name: Option<UserName>,
    /// 사용자 게임 티어
    tier: GameTier,
    /// 프로필 대표 캐릭터
    profile_character: Option<CharacterKind>,
    /// 로그인 토큰
    token: LoginToken,

    /// 스테이징(업로드) 버퍼 집합
    staging_buffers: Vec<wgpu::Buffer>,
    /// 작업 결과
    task_results: Arc<Queue<Result<TaskResult, Box<dyn Error + Send>>>>,
    /// 남은 작업의 수
    num_remaining_tasks: usize,

    /// 이전 텍스처 풀 객체
    previous_texture_pool: TexturePool,
    /// 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl MainLobbyEnterScene {
    /// 새로운 `MainLobbyEnterScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_character: Option<CharacterKind>,
        texture_pool: TexturePool,
        token: LoginToken,
    ) -> Self {
        Self {
            locale,
            uid,
            name: Some(name),
            tier,
            profile_character,
            token,
            staging_buffers: Vec::default(),
            task_results: Arc::new(Queue::new()),
            num_remaining_tasks: 0,
            previous_texture_pool: texture_pool,
            texture_pool: TexturePool::new(),
        }
    }

    /// `MainLobby`의 배경 텍스처를 풀 객체에 등록합니다.
    fn load_background_texture<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) where
        Dir: Into<PathBuf>,
    {
        let mut path: PathBuf = root_dir.into();
        path.push(format!("ui/{}", BG_MAIN_LOBBY_URI));

        // 스레드 풀에서 에셋을 로드합니다.
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();
        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
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

            // 이미지를 디코딩합니다.
            let reader = Cursor::new(buf);
            let mut reader = ImageReader::new(reader);
            reader.set_format(ImageFormat::Png);

            let image = match reader.decode() {
                Ok(image) => image,
                Err(e) => {
                    log::error!("failed to load texture! (PATH:{BG_MAIN_LOBBY_URI}, REASON:{e}");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // 텍스처를 생성합니다.
            let texture = TexturePool::create_texture(
                &format!("Texture({})", &BG_MAIN_LOBBY_URI),
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
            texture_pool.insert(BG_MAIN_LOBBY_URI, texture.into());

            // 결과를 전송합니다.
            task_results.push(Ok(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            }));
        });
        self.num_remaining_tasks += 1;
    }
}

impl GameScene for MainLobbyEnterScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let mut root_dir = app.current_dir().to_path_buf();
        root_dir.push("assets");
        self.load_background_texture(root_dir, app.io_threads(), app.render_device());
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
        let title = ERR_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결이 끊어졌습니다!"];
                ERR_MSG_TEXTS[i]
            }
            NetworkError::IO(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] =
                    ["패킷을 읽는 도중 오류가 발생했습니다!"];
                ERR_MSG_TEXTS[i]
            }
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            match result {
                Ok(task) => {
                    self.num_remaining_tasks -= 1;
                    log::info!(
                        "task success (number of tasks remaining:{})",
                        self.num_remaining_tasks
                    );

                    match task {
                        TaskResult::Texture {
                            command,
                            mut staging_buffers,
                        } => {
                            app.render_queue().submit(Some(command));
                            self.staging_buffers.append(&mut staging_buffers);
                        }
                    }
                }
                Err(_) => {
                    // 다음 게임 장면으로 전환합니다.
                    let i = self.locale as usize;
                    let next_scene = FatalErrorSceneLayer::new(
                        self.locale,
                        ERR_TITLE_TEXTS[i],
                        ERR_MSG_TEXTS[i],
                    );
                    let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
            };
        }

        // 다음 게임 장면으로 전환합니다.
        if self.num_remaining_tasks == 0 && self.name.is_some() {
            let name = self.name.take().unwrap();
            let next_scene = MainLobbyScene::new(
                self.locale,
                self.uid,
                name,
                self.tier,
                self.profile_character,
                self.token,
                self.texture_pool.clone(),
            );
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
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

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 폰트 속성

        // 텍스트
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let loading_text = egui::RichText::new("Now Loading")
            .font(font_id)
            .color(egui::Color32::WHITE);

        let offset = clip_rect.max - egui::vec2(32.0, 32.0) * scale;
        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::RIGHT_BOTTOM, offset.to_vec2())
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.label(loading_text)
            });
    }
}
