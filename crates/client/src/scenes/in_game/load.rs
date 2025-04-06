use std::{error::Error, io::Cursor, sync::Arc};

use ahash::{HashMap, HashSet};
use ddsfile::Dds;
use image::{ImageFormat, ImageReader};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{BulletKind, Float3, LoginToken, StageLayoutData, UserId},
    protocol::InitStagePacket,
};
use mod_parallelism::collections::Queue;
use rayon::ThreadPool;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    asset::{
        AssetError, MeshPool, ModelHierarchyPool, SamplerPool, StageModel, TextureDataPool,
        TexturePool, TextureViewPool, DAMAGE_FONT_URI, NOTOSANS_BOLD, SKYBOX_URI, STAGE_URIS,
        STAGE_WORKSPACES, UI_GAME_LAYOUT_URI,
    },
    component::{load_bullet_model, load_character_model},
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

use super::InGameBuildScene;

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];
/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["오류"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["게임 리소스를 로드하는데 실패했습니다!"];

/// 게임 월드에 필요한 에셋을 로드하는 장면입니다.
pub struct InGameLoadScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 클라이언트 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 초기화 패킷
    packet: Option<InitStagePacket>,

    /// 그래픽스 명령어 작업
    commands: Arc<Queue<(Vec<wgpu::Buffer>, wgpu::CommandBuffer)>>,
    /// 작업 결과를 저장합니다.
    task_results: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,
    /// 남은 작업의 수
    num_remaining_tasks: usize,

    /// 로드된 에셋 데이터입니다.
    stage_models: Arc<Queue<StageModel>>,

    /// 텍스처 데이터 풀 객체입니다.
    texture_data_pool: TextureDataPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
    /// 메쉬 풀 객체입니다.
    mesh_pool: MeshPool,
}

impl InGameLoadScene {
    /// 새로운 `InGameLoadScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        packet: InitStagePacket,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            packet: Some(packet),
            commands: Arc::new(Queue::new()),
            task_results: Arc::new(Queue::new()),
            num_remaining_tasks: 0,
            stage_models: Arc::new(Queue::new()),
            texture_data_pool: TextureDataPool::new(),
            texture_pool: TexturePool::new(),
            texture_view_pool: TextureViewPool::new(),
            sampler_pool: SamplerPool::new(),
            mesh_pool: MeshPool::new(),
        }
    }

    /// 사용되는 캐릭터 모델을 로드합니다.
    fn load_character_models(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let character_kinds: Vec<_> = init_stage_packet
            .players
            .iter()
            .map(|player| player.character_kind)
            .collect();
        for character_kind in character_kinds {
            let commands = self.commands.clone();
            let task_results = self.task_results.clone();
            let texture_data_pool = self.texture_data_pool.clone();
            let texture_pool = self.texture_pool.clone();
            let texture_view_pool = self.texture_view_pool.clone();
            let sampler_pool = self.sampler_pool.clone();
            let mesh_pool = self.mesh_pool.clone();
            let asset_manager = asset_manager.clone();
            let device = device.clone();
            let queue = queue.clone();

            thread_pool.spawn(move || {
                // 스레드의 커맨드 버퍼를 생성합니다.
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                // 캐릭터 모델을 로드합니다.
                let result = load_character_model(
                    &texture_data_pool,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                    &mesh_pool,
                    &asset_manager,
                    character_kind,
                    &device,
                    &queue,
                    &mut encoder,
                    &mut staging_buffers,
                )
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
                log::debug!("task finished (TYPE: Load Character Model)");

                // 커맨드 버퍼를 전송합니다.
                commands.push((staging_buffers, encoder.finish()));
                // 결과를 전송합니다.
                task_results.push(result);
            });
            self.num_remaining_tasks += 1;
        }
    }

    /// 사용되는 총알 모델을 로드합니다.
    fn load_bullet_models(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let bullet_kinds: HashSet<BulletKind> = init_stage_packet
            .players
            .iter()
            .map(|player| player.character_kind.into())
            .collect();
        for bullet_kind in bullet_kinds {
            let commands = self.commands.clone();
            let task_results = self.task_results.clone();
            let texture_data_pool = self.texture_data_pool.clone();
            let texture_pool = self.texture_pool.clone();
            let texture_view_pool = self.texture_view_pool.clone();
            let sampler_pool = self.sampler_pool.clone();
            let mesh_pool = self.mesh_pool.clone();
            let asset_manager = asset_manager.clone();
            let device = device.clone();
            let queue = queue.clone();

            thread_pool.spawn(move || {
                // 스레드의 커맨드 버퍼를 생성합니다.
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                // 총알 모델을 로드합니다.
                let result = load_bullet_model(
                    &texture_data_pool,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                    &mesh_pool,
                    &asset_manager,
                    bullet_kind,
                    &device,
                    &queue,
                    &mut encoder,
                    &mut staging_buffers,
                )
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
                log::debug!("task finished (TYPE: Load Bullet Model)");

                // 커맨드 버퍼를 전송합니다.
                commands.push((staging_buffers, encoder.finish()));
                // 결과를 전송합니다.
                task_results.push(result);
            });
            self.num_remaining_tasks += 1;
        }
    }

    /// 지역 모델들을 로드합니다.
    fn load_stage_models(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let i = init_stage_packet.stage_kind() as usize;

        let stage_models = self.stage_models.clone();
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            // 스레드의 커맨드 버퍼를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // 지형 데이터를 로드합니다.
            let result = asset_manager.get_or_init(STAGE_URIS[i]);
            let data = match result {
                Ok(asset) => asset.as_bytes().to_vec(),
                Err(e) => {
                    log::error!("failed to load asset! (URI:{}, REASON:{e})", STAGE_URIS[i]);
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 데이터를 구문분석합니다.
            let layout: StageLayoutData = match serde_json::from_slice(&data) {
                Ok(layout) => layout,
                Err(e) => {
                    log::error!(
                        "failed to parse stage layout! (URI:{}, REASON:{e})",
                        STAGE_URIS[i]
                    );
                    task_results.push(Err(Box::new(AssetError::from(e))));
                    return;
                }
            };

            // 지형 데이터를 구성하는 모델을 로드합니다.
            let mut models = HashMap::default();
            for model_name in layout.models.iter() {
                let result = ModelHierarchyPool::get_or_init(
                    &texture_data_pool,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                    &mesh_pool,
                    &model_name,
                    STAGE_WORKSPACES[i],
                    &asset_manager,
                    &device,
                    &queue,
                    &mut encoder,
                    &mut staging_buffers,
                );

                match result {
                    Ok(root) => models.insert(model_name.clone(), root),
                    Err(e) => {
                        log::error!("failed to load asset! (REASON:{})", e);
                        task_results.push(Err(Box::new(e)));
                        return;
                    }
                };
            }

            for area in layout.area.iter() {
                let model_root = match models.get(&area.model) {
                    Some(root) => root.clone(),
                    None => panic!("stage model not found! (MODEL:{})", &area.model),
                };

                stage_models.push(StageModel {
                    is_terrain: true,
                    model_root,
                    scale: Float3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                    rotation: area.rotation,
                    translation: area.translation,
                });
            }

            for prop in layout.props.iter() {
                let model_root = match models.get(&prop.model) {
                    Some(root) => root.clone(),
                    None => {
                        panic!("stage model not found! (MODEL:{})", &prop.model);
                    }
                };

                stage_models.push(StageModel {
                    is_terrain: false,
                    model_root,
                    scale: prop.scale,
                    rotation: prop.rotation,
                    translation: prop.translation,
                });
            }

            // 캐싱된 에셋을 제거합니다.
            asset_manager.remove(STAGE_URIS[i]);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            log::debug!("task finished (TYPE: Load Stage Model)");
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// `UI_Game_Layout` 텍스처를 생성합니다.
    fn create_ui_game_layout_texture(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            // `UI_Game_Layout` 텍스처를 로드합니다.
            let result = asset_manager.get_or_init(UI_GAME_LAYOUT_URI);
            let bytes = match result {
                Ok(asset) => asset.as_bytes().to_vec(),
                Err(e) => {
                    log::error!("failed to load assets! (Uri:{UI_GAME_LAYOUT_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스처를 로드합니다.
            let reader = Cursor::new(bytes);
            let mut reader = ImageReader::new(reader);
            reader.set_format(ImageFormat::Png);
            let image = match reader.decode() {
                Ok(image) => image,
                Err(e) => {
                    log::error!("failed to load texture! (URI:{UI_GAME_LAYOUT_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let texture = Arc::new(device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", UI_GAME_LAYOUT_URI)),
                    size: wgpu::Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::default(),
                &image.to_rgba8(),
            ));

            // 텍스터를 등록합니다.
            texture_pool.insert(UI_GAME_LAYOUT_URI, texture);

            // 캐시를 지웁니다.
            asset_manager.remove(UI_GAME_LAYOUT_URI);

            // 결과를 전송합니다.
            log::debug!("task finished (TYPE: Load Ui Texture)");
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 스카이박스 텍스처를 생성합니다.
    fn create_skybox_texture(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            // 스카이박스 텍스처를 로드합니다.
            let result = asset_manager.get_or_init(SKYBOX_URI);
            let bytes = match result {
                Ok(asset) => asset.as_bytes().to_vec(),
                Err(e) => {
                    log::error!("failed to load assets! (Uri:{SKYBOX_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스처를 로드합니다.
            let reader = Cursor::new(bytes);
            let dds = match Dds::read(reader) {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!("failed to load texture! (URI:{SKYBOX_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let texture = Arc::new(device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", SKYBOX_URI)),
                    size: wgpu::Extent3d {
                        width: dds.get_width(),
                        height: dds.get_height(),
                        depth_or_array_layers: 6,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bc7RgbaUnorm,
                    mip_level_count: dds.get_num_mipmap_levels(),
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::default(),
                &dds.data,
            ));

            // 텍스터를 등록합니다.
            texture_pool.insert(SKYBOX_URI, texture);

            // 캐시를 지웁니다.
            asset_manager.remove(SKYBOX_URI);

            // 결과를 전송합니다.
            log::debug!("task finished (TYPE: Load Skybox Texture)");
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 데미지 폰트 텍스처를 생성합니다.
    fn create_damage_font(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            // 데미지 폰트 텍스처를 로드합니다.
            let result = asset_manager.get_or_init(DAMAGE_FONT_URI);
            let bytes = match result {
                Ok(asset) => asset.as_bytes().to_vec(),
                Err(e) => {
                    log::error!("failed to load assets! (Uri:{DAMAGE_FONT_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스처를 로드합니다.
            let reader = Cursor::new(bytes);
            let dds = match Dds::read(reader) {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!("failed to load texture! (URI:{DAMAGE_FONT_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let texture = Arc::new(device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", SKYBOX_URI)),
                    size: wgpu::Extent3d {
                        width: dds.get_width(),
                        height: dds.get_height(),
                        depth_or_array_layers: 1,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bc7RgbaUnorm,
                    mip_level_count: dds.get_num_mipmap_levels(),
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::default(),
                &dds.data,
            ));

            // 텍스터를 등록합니다.
            texture_pool.insert(DAMAGE_FONT_URI, texture);

            // 캐시를 지웁니다.
            asset_manager.remove(DAMAGE_FONT_URI);

            // 결과를 전송합니다.
            log::debug!("task finished (TYPE: Load Font Texture)");
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }
}

impl GameScene for InGameLoadScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
        self.load_character_models(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
        self.load_bullet_models(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
        self.load_stage_models(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
        self.create_ui_game_layout_texture(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
        self.create_skybox_texture(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
        self.create_damage_font(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
        let title = ERR_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결이 끊겼습니다!"];
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
        let event = AppEvent::SetGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_exit(&mut self, _window: Option<&Window>, app: &dyn AppHandle) {
        // 커맨드 버퍼를 수집합니다.
        let mut commands = Vec::new();
        let mut staging_buffers: Vec<wgpu::Buffer> = Vec::new();
        while let Some((mut buffers, command)) = self.commands.pop() {
            commands.push(command);
            staging_buffers.append(&mut buffers);
        }

        app.render_queue().submit(commands);
        drop(staging_buffers);
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            match result {
                Ok(()) => {
                    self.num_remaining_tasks -= 1;
                    log::debug!(
                        "task success (number of tasks remaining:{}, queue:{})",
                        self.num_remaining_tasks,
                        self.task_results.len()
                    );
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
                    let event = AppEvent::SetGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
            };
        }

        // 모든 작업이 끝난 경우 다음 장면으로 전환합니다.
        if self.num_remaining_tasks == 0 {
            let next_scene = Box::new(InGameBuildScene::new(
                self.locale,
                self.user_id,
                self.token,
                self.packet.take(),
                self.stage_models.clone(),
                self.texture_data_pool.clone(),
                self.texture_pool.clone(),
                self.texture_view_pool.clone(),
                self.sampler_pool.clone(),
                self.mesh_pool.clone(),
            ));
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_draw(
        &self,
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
                    view: render_target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());

        // 로드 텍스트
        let text = LOAD_TEXTS[i];
        let font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let load_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        egui::Area::new(egui::Id::new("Load_Layout"))
            .anchor(egui::Align2::RIGHT_BOTTOM, (-16.0 * scale, -16.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(load_text)
                });
            });
    }
}
