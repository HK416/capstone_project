use std::sync::{Arc, OnceLock};

use ahash::{HashMap, RandomState};
use hecs::World;
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, StageLayoutAttributes, UserId, MAX_IN_GAME_PLAYERS},
    protocol::{InGameDataInitPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use spin::mutex::SpinMutex;
use winit::window::Window;

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, StageBoundingVolumnHierarchy,
        TextureDataPool, TexturePool, TextureViewPool, NOTOSANS_BOLD, SKYBOX_URI,
    },
    component::{build_stage, spawn_player_character, GlobalLight, Skybox},
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
    PACKET_DELAY, SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];
/// 애플리케이션 표시 언어에 따른 로드 텍스트
const WAIT_TEXTS: [&'static str; NUM_LOCALE] = ["다른 플레이어를 기다리는 중"];

/// 게임 월드에 필요한 에셋을 생성하는 장면입니다.
pub struct InGameBuildScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 클라이언트 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 초기화 패킷
    packet: Option<InGameDataInitPacket>,
    /// 스테이지 레이아웃 데이터
    stage_layout_data: Arc<OnceLock<StageLayoutAttributes>>,

    /// 완료된 드로우 콜 명령어입니다.
    commands: Arc<Queue<(wgpu::CommandBuffer, Vec<wgpu::Buffer>)>>,
    /// 작업 결과입니다
    load_finished: bool,
    /// 다음 장면입니다.  
    /// 모든 작업이 완료되지 않았을 경우 `None`입니다.
    next_scene: Arc<SpinMutex<Option<Box<InGameDominationModePrepareScene>>>>,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    /// 메쉬 풀 객체입니다.
    mesh_pool: MeshPool,
    /// 모델 풀 객체입니다.
    model_pool: ModelPool,
    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
    /// 텍스처 데이터 풀 객체입니다.
    texture_data_pool: TextureDataPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
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
        stage_layout_data: Arc<OnceLock<StageLayoutData>>,
        mesh_pool: MeshPool,
        model_pool: ModelPool,
        motion_pool: MotionPool,
        texture_data_pool: TextureDataPool,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
    ) -> Self {
        assert!(packet.is_some(), "packet must exist!");
        Self {
            locale,
            user_id,
            token,
            packet,
            stage_layout_data,
            commands: Arc::new(Queue::new()),
            load_finished: false,
            next_scene: Arc::new(SpinMutex::new(None)),
            elapsed_time_sec: 0.0,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
        }
    }

    /// 다음 게임 장면을 생성합니다.
    fn build_next_scene(&mut self, device: &Arc<wgpu::Device>) {
        let device = device.clone();

        let locale = self.locale;
        let user_id = self.user_id;
        let token = self.token;

        let commands = self.commands.clone();
        let packet = self.packet.take().expect("the packet must exist!");
        let stage_layout_data = self.stage_layout_data.clone();

        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let motion_pool = self.motion_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();

        let result = self.next_scene.clone();

        rayon::spawn(move || {
            let mut world = World::new();
            let batch_commands: Arc<Queue<_>> = Arc::new(Queue::new());

            let device_ref = &device;
            let world_ref = &world;
            let commands_ref = &commands;
            let batch_commands_ref = &batch_commands;
            let model_pool_ref = &model_pool;
            let texture_data_pool_ref = &texture_data_pool;
            let stage_layout_data_ref = stage_layout_data
                .get()
                .expect("the stage layout data must exist!");

            let player_entities: Arc<Queue<_>> = Arc::new(Queue::new());
            let stage_result: Arc<SpinMutex<Option<StageBoundingVolumnHierarchy>>> =
                Arc::new(SpinMutex::new(None));
            rayon::scope(|scope| {
                {
                    // 플레이어 캐릭터 엔터티를 생성합니다.
                    let players = player_entities.clone();
                    scope.spawn(move |_| {
                        let mut staging_buffers = Vec::new();
                        let mut encoder = device_ref
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        for player in packet.players {
                            let user_id = player.account.uid;
                            let (entity, batch_command) = spawn_player_character(
                                world_ref,
                                model_pool_ref,
                                texture_data_pool_ref,
                                &player,
                                device_ref,
                                &mut encoder,
                                &mut staging_buffers,
                            );

                            // 엔터티 생성 명령어를 전송합니다.
                            batch_commands_ref.push(batch_command);

                            // 플레이어 엔터티를 전송합니다.
                            players.push((user_id, entity));
                        }

                        // 렌더링 명령어를 전송합니다.
                        commands_ref.push((encoder.finish(), staging_buffers));
                    });
                }

                // 스테이지 지형 엔터티를 생성합니다.
                {
                    let stage_result = stage_result.clone();
                    scope.spawn(move |_| {
                        let mut staging_buffers = Vec::new();
                        let mut encoder = device_ref
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        let (bvh, batch_commands) = build_stage(
                            world_ref,
                            model_pool_ref,
                            texture_data_pool_ref,
                            stage_layout_data_ref,
                            device_ref,
                            &mut encoder,
                            &mut staging_buffers,
                        );

                        // 엔터티 생성 명령어를 전송합니다.
                        batch_commands_ref.push(batch_commands);

                        // 지형 엔터티를 전송합니다.
                        *stage_result.lock() = Some(bvh);

                        // 렌더링 명령어를 전송합니다.
                        commands_ref.push((encoder.finish(), staging_buffers));
                    });
                }
            });

            // 스카이박스를 생성합니다.
            let skybox = {
                // 텍스처를 가져옵니다.
                let texture = texture_pool
                    .get(SKYBOX_URI)
                    .expect("the skybox texture must exist!")
                    .create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::Cube),
                        ..Default::default()
                    });
                let sampler = sampler_pool.get_or_init(
                    &device,
                    &wgpu::SamplerDescriptor {
                        ..Default::default()
                    },
                );

                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                let skybox = Skybox::new(
                    Some("Skybox"),
                    &device,
                    &texture,
                    &sampler,
                    &mut encoder,
                    &mut staging_buffers,
                );

                // 렌더링 명령어를 전송합니다.
                commands.push((encoder.finish(), staging_buffers));

                skybox
            };

            // 엔터티 생성 명령어를 실행합니다.
            while let Some(batch) = batch_commands.pop() {
                for (entity, mut builder) in batch {
                    world
                        .insert(entity, builder.build())
                        .expect("no such entity!");
                }
            }

            // 플레이어 집합을 생성합니다.
            let mut players =
                HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new());
            while let Some((user_id, entity)) = player_entities.pop() {
                players.insert(user_id, entity);
            }

            // 지형의 Bounding Volumn Hierarchy를 가져옵니다.
            let stages = stage_result.lock().take().unwrap();

            // 조명 집합을 생성합니다
            let global_light = stage_layout_data_ref.global_light.as_ref().map(|light| {
                let (static_shadow_map_view, static_shadow_map_sampler) = texture_data_pool_ref
                    .get(&light.shadow_map)
                    .expect("the static shadow map must exist!");

                GlobalLight {
                    static_shadow_map_view,
                    static_shadow_map_sampler,
                    light_proj_view: light.light_proj_view.into(),
                    direction_w: light.direction_w.into(),
                    color: light.color.into(),
                }
            });

            // 다음 게임 장면을 생성합니다.
            let next_scene = InGameDominationModePrepareScene::new(
                locale,
                user_id,
                token,
                world,
                skybox,
                players,
                stages,
                global_light,
                mesh_pool,
                model_pool,
                motion_pool,
                texture_pool,
                texture_data_pool,
                texture_view_pool,
                sampler_pool,
            );
            *result.lock() = Some(Box::new(next_scene));
        });
    }
}

impl GameScene for InGameBuildScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        self.build_next_scene(app.render_device());
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let mut staging_buffers = Vec::new();
        let mut command_buffers = Vec::new();
        while let Some((commmand, buffer)) = self.commands.pop() {
            staging_buffers.push(buffer);
            command_buffers.push(commmand);
        }

        app.render_queue().submit(command_buffers);
        drop(staging_buffers);
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::PrepareStage | PacketType::PullStage => {
                // 다음 게임 장면으로 전환합니다.
                if let Some(next_scene) = self.next_scene.lock().take() {
                    let scene_flow = GameSceneFlow::Change(next_scene);
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
            }
            _ => {
                log::warn!(
                    "ignored >> invalid packet received! (TYPE:{:?})",
                    packet_type
                );
            }
        }

        None
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 현재 상태를 서버에 보고합니다.
        self.load_finished |= self.next_scene.lock().is_some();

        // 작업 완료 패킷을 전송합니다.
        if self.elapsed_time_sec >= PACKET_DELAY {
            self.elapsed_time_sec = 0.0;
            let packet = PushSyncPacket::new(self.user_id, self.token, self.load_finished);
            let net_manager = app.net_manager();
            let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
            socket.push_packet(packet.as_raw());
        }
    }

    fn on_draw(
        &mut self,
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
        let text = if self.load_finished {
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
