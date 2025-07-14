use std::{error::Error, path::Path, sync::Arc};

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{GameTier, LoginToken, ProfileIcon, UserId, UserName};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::{
        SamplerPool, TextureDataPool, TexturePool, TextureViewPool, ARONA_SAD_URI, BG_DECO_URI,
        BG_FORMATION_URI, BG_MAIN_LOBBY_URI, CHARACTER_IMG_URI, EMBLEM_BG_URI, GAME_LOGO_URI,
        HUD_CANCEL_ICON_URI, HUD_CHANGE_ICON_URI, HUD_DETAIL_ICON_URI, HUD_EXIT_ICON_URI,
        HUD_LAYOUT_URI_00, HUD_LAYOUT_URI_01, HUD_LAYOUT_URI_02, HUD_LAYOUT_URI_03,
        HUD_OPTION_ICON_URI, ICON_WORKSPACE, IMG_FONT_HOST_URI, IMG_FONT_READY_URI,
        IMG_FONT_WORKSPACE, NOTOSANS_BOLD, PROFILE_ICON_URI, RANK_ICON_URI,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
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
    Err(Box<dyn Error>),
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
    /// 프로필 아이콘
    profile_icon: ProfileIcon,
    /// 로그인 토큰
    token: LoginToken,

    /// 스테이징(업로드) 버퍼 집합
    staging_buffers: Vec<wgpu::Buffer>,
    /// 작업 결과
    task_results: Arc<Queue<TaskResult>>,
    /// 남은 작업의 수
    num_remaining_tasks: usize,

    /// 텍스터 풀 객체
    texture_data_pool: TextureDataPool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
}

impl MainLobbyEnterScene {
    /// 새로운 `MainLobbyEnterScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_icon: ProfileIcon,
        previous_texture_pool: &TexturePool,
        token: LoginToken,
    ) -> Self {
        // 새로운 텍스처 풀을 생성하고 이전 텍스처 풀에서 필요한 데이터를 취합니다.
        let texture_pool = TexturePool::new();

        // Game_Logo
        let texture = previous_texture_pool
            .remove(GAME_LOGO_URI)
            .expect("Game_Logo texture must be preloaded!");
        texture_pool.insert(GAME_LOGO_URI, texture);

        // HUD_Exit_Icon
        let texture = previous_texture_pool
            .remove(HUD_EXIT_ICON_URI)
            .expect("HUD_Exit_Icon texture must be preloaded!");
        texture_pool.insert(HUD_EXIT_ICON_URI, texture);

        // HUD_Detail_Icon
        let texture = previous_texture_pool
            .remove(HUD_DETAIL_ICON_URI)
            .expect("HUD_Detail_Icon texture must be preloaded!");
        texture_pool.insert(HUD_DETAIL_ICON_URI, texture);

        // HUD_Option_Icon
        let texture = previous_texture_pool
            .remove(HUD_OPTION_ICON_URI)
            .expect("HUD_Option_Icon texture must be preloaded!");
        texture_pool.insert(HUD_OPTION_ICON_URI, texture);

        // HUD_Change_Icon
        let texture = previous_texture_pool
            .remove(HUD_CHANGE_ICON_URI)
            .expect("HUD_Change_Icon texture must be preloaded!");
        texture_pool.insert(HUD_CHANGE_ICON_URI, texture);

        // HUD_Cancel_Icon
        let texture = previous_texture_pool
            .remove(HUD_CANCEL_ICON_URI)
            .expect("HUD_Cancel_Icon texture must be preloaded!");
        texture_pool.insert(HUD_CANCEL_ICON_URI, texture);

        Self {
            locale,
            uid,
            name: Some(name),
            tier,
            profile_icon,
            token,
            staging_buffers: Vec::default(),
            task_results: Arc::new(Queue::new()),
            num_remaining_tasks: 0,
            texture_data_pool: TextureDataPool::new(),
            texture_view_pool: TextureViewPool::new(),
            texture_pool,
            sampler_pool: SamplerPool::new(),
        }
    }

    /// 레이아웃 이미지 텍스처를 생성합니다.
    fn create_layout_textures<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let uris = vec![
            HUD_LAYOUT_URI_00,
            HUD_LAYOUT_URI_01,
            HUD_LAYOUT_URI_02,
            HUD_LAYOUT_URI_03,
        ];
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            for uri in uris {
                let result = texture_data_pool.get_or_init(
                    &workspace,
                    uri,
                    &device,
                    &mut encoder,
                    &mut staging_buffers,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                );

                if let Err(e) = result {
                    task_results.push(TaskResult::Err(Box::new(e)));
                    return;
                }
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// 준비 이미지 폰트 텍스처를 생성합니다.
    fn create_img_font_ready_textures<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push(IMG_FONT_WORKSPACE);

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                IMG_FONT_READY_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// Host 이미지 폰트 텍스처를 생성합니다.
    fn create_img_font_host_textures<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push(IMG_FONT_WORKSPACE);

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                IMG_FONT_HOST_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// 파일로부터 프로필 아이콘 텍스처를 생성합니다.
    fn create_profile_bg_textures<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                EMBLEM_BG_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// 파일로부터 프로필 아이콘 텍스처를 생성합니다.
    fn create_profile_icon_textures<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push(ICON_WORKSPACE);

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                PROFILE_ICON_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// 파일로부터 프로필 아이콘 텍스처를 생성합니다.
    fn create_rank_icon_textures<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push(ICON_WORKSPACE);

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                RANK_ICON_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    fn create_character_img_texture<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                CHARACTER_IMG_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// `MainLobby`의 배경 텍스처를 풀 객체에 등록합니다.
    fn create_background_texture<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                BG_MAIN_LOBBY_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// 캐릭터 편성 장면의 배경 텍스처를 풀 객체에 등록합니다.
    fn create_formation_background_texture<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                BG_FORMATION_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// `MainLobby`의 배경 텍스처를 풀 객체에 등록합니다.
    fn create_background_deco_texture<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                BG_DECO_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }

    /// 종료 대화상자 모달에 사용되는 텍스처를 풀 객체에 등록합니다.
    fn create_arona_sad_texture<Dir>(
        &mut self,
        root_dir: Dir,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) where
        Dir: AsRef<Path>,
    {
        let mut workspace = root_dir.as_ref().to_path_buf();
        workspace.push("ui");

        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let mut staging_buffers = Vec::new();

            let result = texture_data_pool.get_or_init(
                &workspace,
                ARONA_SAD_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
            );

            if let Err(e) = result {
                task_results.push(TaskResult::Err(Box::new(e)));
                return;
            }

            task_results.push(TaskResult::Texture {
                command: encoder.finish(),
                staging_buffers,
            });
        });

        self.num_remaining_tasks += 1;
    }
}

impl GameScene for MainLobbyEnterScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        let io_thread_pool = app.io_threads();
        let mut root_dir = app.current_dir().to_path_buf();
        root_dir.push("assets");

        self.create_layout_textures(&root_dir, io_thread_pool, device.clone());
        self.create_rank_icon_textures(&root_dir, io_thread_pool, device.clone());
        self.create_character_img_texture(&root_dir, io_thread_pool, device.clone());
        self.create_background_texture(&root_dir, io_thread_pool, device.clone());
        self.create_background_deco_texture(&root_dir, io_thread_pool, device.clone());
        self.create_formation_background_texture(&root_dir, io_thread_pool, device.clone());
        self.create_profile_bg_textures(&root_dir, io_thread_pool, device.clone());
        self.create_profile_icon_textures(&root_dir, io_thread_pool, device.clone());
        self.create_img_font_ready_textures(&root_dir, io_thread_pool, device.clone());
        self.create_img_font_host_textures(&root_dir, io_thread_pool, device.clone());
        self.create_arona_sad_texture(&root_dir, io_thread_pool, device.clone());
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
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
            self.num_remaining_tasks -= 1;
            log::info!("number of remaining tasks: {}", self.num_remaining_tasks);

            match result {
                TaskResult::Texture {
                    command,
                    mut staging_buffers,
                } => {
                    app.render_queue().submit(Some(command));
                    self.staging_buffers.append(&mut staging_buffers);
                }
                TaskResult::Err(_e) => {
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
                self.profile_icon,
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
                    depth_slice: None,
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
