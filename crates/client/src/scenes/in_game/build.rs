use std::{
    error::Error,
    sync::{
        atomic::{self, Ordering as MemOrdering},
        Arc,
    },
};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, UserId},
    protocol::{InitStagePacket, Packet, PacketType, PushSyncPacket, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_render::SkyboxResource;
use winit::window::Window;

use crate::{
    asset::{
        MeshPool, SamplerPool, StageModel, TextureDataPool, TexturePool, TextureViewPool,
        NOTOSANS_BOLD, SKYBOX_URI,
    },
    component::{spawn_player_character, spawn_stage_area_from_root, spawn_stage_prop_from_root},
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
    SERVER_TCP_ADDR,
};

use super::InGameDominationModeScene;

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];
/// 애플리케이션 표시 언어에 따른 로드 텍스트
const WAIT_TEXTS: [&'static str; NUM_LOCALE] = ["다른 플레이어를 기다리는 중"];
/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["오류"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["게임 리소스를 로드하는데 실패했습니다!"];

/// 게임 월드에 필요한 에셋을 생성하는 장면입니다.
pub struct InGameBuildScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 클라이언트 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 초기화 패킷
    packet: Option<InitStagePacket>,
    /// 로드된 에셋 데이터입니다.
    stage_models: Arc<Queue<StageModel>>,

    /// 작업 결과를 저장합니다.
    task_result: Arc<Queue<Result<Box<dyn GameScene>, Box<dyn Error + Send>>>>,
    /// 작업이 완료된 여부
    load_finish: bool,

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

impl InGameBuildScene {
    /// 새로운 `InGameBuildScene`을 생성합니다.
    ///
    /// # Panics
    /// 주어진 `InitStagePacket`은 `None`이 될 수 없습니다.
    ///
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        packet: Option<InitStagePacket>,
        stage_models: Arc<Queue<StageModel>>,
        texture_data_pool: TextureDataPool,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
        mesh_pool: MeshPool,
    ) -> Self {
        assert!(packet.is_some(), "packet must exist!");
        Self {
            locale,
            user_id,
            token,
            packet,
            stage_models,
            task_result: Arc::new(Queue::new()),
            load_finish: false,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
            mesh_pool,
        }
    }

    /// 다음 게임 장면을 생성합니다.
    fn build_next_scene(
        &mut self,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let init_stage_packet = self.packet.take().expect("packet must exits!");
        let task_result = self.task_result.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let stage_models = self.stage_models.clone();
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let token = self.token;
        let user_id = self.user_id;
        let locale = self.locale;
        rayon::spawn(move || {
            let mut world = World::default();
            let player_entities: Arc<Queue<(UserId, Entity)>> = Arc::new(Queue::new());
            let batch_commands: Arc<Queue<Vec<(Entity, EntityBuilder)>>> = Arc::new(Queue::new());

            let device_ref = &device;
            let queue_ref = &queue;
            let world_ref = &world;
            let task_result_ref = task_result.clone();
            let player_entities_ref = player_entities.clone();
            let batch_commands_ref = batch_commands.clone();

            let texture_data_pool_ref = &texture_data_pool;
            let texture_pool_ref = &texture_pool;
            let texture_view_pool_ref = &texture_view_pool;
            let sampler_pool_ref = &sampler_pool;
            let mesh_pool_ref = &mesh_pool;

            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            rayon::scope(move |scope| {
                let players = init_stage_packet.players;
                let task_result_cloned = task_result_ref.clone();
                let player_entities_cloned = player_entities_ref.clone();
                let batch_commands_cloned = batch_commands_ref.clone();
                scope.spawn(move |_| {
                    for player in players {
                        let result = spawn_player_character(
                            texture_data_pool_ref,
                            texture_pool_ref,
                            texture_view_pool_ref,
                            sampler_pool_ref,
                            mesh_pool_ref,
                            &player,
                            &asset_manager,
                            device_ref,
                            queue_ref,
                            &mut encoder,
                            &mut staging_buffers,
                            world_ref,
                        );
                        match result {
                            Ok((root, commands)) => {
                                player_entities_cloned.push((player.account.uid, root));
                                batch_commands_cloned.push(commands);
                            }
                            Err(e) => {
                                log::error!("failed to create player entity! (REASON:{e})");
                                task_result_cloned.push(Err(Box::new(e)));
                            }
                        }
                    }
                });

                let task_result_cloned = task_result_ref.clone();
                let batch_commands_cloned = batch_commands_ref.clone();
                scope.spawn(move |_| {
                    while let Some(model) = stage_models.pop() {
                        let spawn_func = if model.is_terrain {
                            spawn_stage_area_from_root
                        } else {
                            spawn_stage_prop_from_root
                        };

                        let result = spawn_func(
                            model.model_root,
                            model.scale.into(),
                            model.rotation.into(),
                            model.translation.into(),
                            &device_ref,
                            &queue_ref,
                            &world_ref,
                        );

                        match result {
                            Ok((_, commands)) => {
                                batch_commands_cloned.push(commands);
                            }
                            Err(e) => {
                                log::error!("failed to create stage entity! (REASON:{e})");
                                task_result_cloned.push(Err(Box::new(e)));
                            }
                        };
                    }
                });
            });

            // 엔터티 생성 명령어를 실행합니다.
            while let Some(commands) = batch_commands.pop() {
                for (entity, mut builder) in commands {
                    world
                        .insert(entity, builder.build())
                        .expect("no such entity!");
                }
            }

            // 플레이어 엔터티 집합을 생성합니다.
            let mut players = HashMap::default();
            while let Some((user_id, player)) = player_entities.pop() {
                players.insert(user_id, player);
            }

            // 스카이박스 쉐이더 리소스를 생성합니다.
            let t_skybox = texture_pool
                .get(SKYBOX_URI)
                .expect("texture must be pre-registered!");
            let t_skybox = texture_view_pool.get_or_init(
                &t_skybox,
                &wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::Cube),
                    ..Default::default()
                },
            );
            let s_skybox = sampler_pool.get_or_init(&device, &wgpu::SamplerDescriptor::default());
            let skybox_resource =
                SkyboxResource::uninit(Some("Skyxox"), &device, &t_skybox, &s_skybox);

            // 다음 장면을 생성합니다.
            let next_scene = InGameDominationModeScene::new(
                locale,
                user_id,
                token,
                texture_data_pool,
                texture_pool,
                texture_view_pool,
                sampler_pool,
                mesh_pool,
                world,
                players,
                skybox_resource.into(),
            );

            // 결과를 전송합니다.
            task_result.push(Ok(Box::new(next_scene)));
        });
    }
}

impl GameScene for InGameBuildScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
        self.build_next_scene(app.asset_manager(), app.render_device(), app.render_queue());
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::PullStage => {
                // 다음 게임 장면으로 전환합니다.
                assert!(self.load_finish, "loading did not complete!");
                if let Some(next_scene) = self.task_result.pop() {
                    let next_scene = next_scene.unwrap();
                    let scene_flow = GameSceneFlow::Change(next_scene);
                    let event = AppEvent::SetGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
            }
            _ => {
                log::warn!("")
            }
        }
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 모든 작업이 끝난 경우 다른 플레이어를 기다립니다.
        if self.load_finish {
            return;
        }

        // 결과를 확인합니다.
        if let Some(result) = self.task_result.pop() {
            let next_scene = match result {
                Ok(scene) => {
                    log::info!("GameScene build success");
                    scene
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
                    return;
                }
            };

            // 다음 게임 장면을 임시 저장합니다.
            self.task_result.push(Ok(next_scene));
            self.load_finish = true;

            atomic::fence(MemOrdering::SeqCst);

            // 작업 완료 패킷을 전송합니다.
            let packet = PushSyncPacket::new(self.user_id, self.token, self.load_finish);

            let net_manager = app.net_manager();
            let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
            socket.push_packet(packet.as_raw());
        }
    }

    fn on_draw(
        &self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn mod_app::app::AppHandle,
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
        let text = if self.load_finish {
            WAIT_TEXTS[i]
        } else {
            LOAD_TEXTS[i]
        };
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
