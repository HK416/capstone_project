use std::{
    sync::{Arc, OnceLock},
    time::Instant,
};

use hecs::{Entity, EntityBuilder, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        CharacterKind, InGamePlayerInitData, LoginToken, StageAttributes, StageKind, Team, UserId,
    },
    protocol::{InGameDataInitPacket, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rayon::Scope;
use rodio::Sink;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, SoundDataPool, StageBoundingVolumnHierarchy,
        TextureDataPool, TexturePool, TextureViewPool, BG_SKY_URI, NOTOSANS_BOLD, UI_NOTICE,
    },
    component::{
        build_stage, spawn_player, DirectionLight, Player0, Player1, Player2, Player3, Player4,
        Player5, Player6, Player7, Player8, Player9, PlayerArchetype, Skybox,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, InGameReadySceneBuilder, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS,
        ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
    },
};

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];

/// 작업 결과 데이터입니다.
enum TaskResult {
    Players {
        uid: UserId,
        entity: Entity,
        archetype: PlayerArchetype,
        batch_commands: Vec<(Entity, EntityBuilder)>,
    },
    Stage {
        stage: StageBoundingVolumnHierarchy,
        batch_commands: Vec<(Entity, EntityBuilder)>,
    },
    Skybox(Skybox),
    DirectionLight(DirectionLight),
    Graphics {
        command_buffer: wgpu::CommandBuffer,
        staging_buffers: Vec<wgpu::Buffer>,
    },
}

/// 게임 월드의 구성 요소를 생성하는 장면입니다.
pub struct InGameBuildScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,
    /// 시야 조작 민감도입니다.
    control_sensitivity: f32,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 플레이어 캐릭터 종류
    player_character: CharacterKind,
    /// 플레이어가 속한 팀
    player_team: Team,

    /// 초기화 패킷
    packet: Option<InGameDataInitPacket>,
    /// 스테이지 레이아웃 데이터
    stage_layout_data: Arc<OnceLock<Arc<StageAttributes>>>,

    /// 메쉬 풀 객체입니다.
    mesh_pool: MeshPool,
    /// 모델 풀 객체입니다.
    model_pool: ModelPool,
    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 데이터 풀 객체입니다.
    texture_data_pool: TextureDataPool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InGameBuildScene {
    /// 새로운 `InGameBuildScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        control_sensitivity: f32,
        flip_horizontal: bool,
        flip_vertical: bool,
        player_character: CharacterKind,
        player_team: Team,
        packet: InGameDataInitPacket,
        stage_layout_data: Arc<OnceLock<Arc<StageAttributes>>>,
        mesh_pool: MeshPool,
        model_pool: ModelPool,
        motion_pool: MotionPool,
        texture_pool: TexturePool,
        texture_data_pool: TextureDataPool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            background_volume,
            effect_volume,
            voice_volume,
            control_sensitivity,
            flip_horizontal,
            flip_vertical,
            player_character,
            player_team,
            packet: Some(packet),
            stage_layout_data,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
            sound_data_pool,
        }
    }
}

impl GameScene for InGameBuildScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let packet = self.packet.take().expect("the packet must be exists!");
        let device = app.render_device().clone();
        let queue = app.render_queue().clone();
        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let motion_pool = self.motion_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let sound_data_pool = self.sound_data_pool.clone();
        let stage_attributes = self.stage_layout_data.clone();
        let event_loop_proxy = app.event_loop_proxy().clone();

        build_next_scene(
            self.locale,
            self.uid,
            self.token,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            self.control_sensitivity,
            self.flip_horizontal,
            self.flip_vertical,
            self.player_character,
            self.player_team,
            packet,
            device,
            queue,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_pool,
            texture_data_pool,
            texture_view_pool,
            sampler_pool,
            sound_data_pool,
            stage_attributes,
            event_loop_proxy,
        );
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            title,
            message,
            self.sound_data_pool.clone(),
        );
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 효과음을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(UI_NOTICE)
            .expect("UI_Notice sound must be preloaded!");
        let source = decoded.as_source();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.effect_volume as f32 / 255.0);
        sink.append(source);
        sink.play();
        sink.detach();
    }

    fn on_received_packet(
        &mut self,
        time_stamp: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::InGameEnterNotify => {
                let event = AppEvent::PacketReceived(time_stamp, packet);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            _ => {}
        }

        None
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
                    view: render_target_view,
                    depth_slice: None,
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
        let i = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 로드 텍스트
        let text = LOAD_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);

        let ctx = app.egui_ctx();
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(clip_rect);

                let width = clip_rect.width() * 0.3;
                let height = width * 0.25;
                let size = egui::vec2(width, height);
                let max = clip_rect.max;
                let min = max - size;
                let rect = egui::Rect::from_min_max(min, max);
                ui.put(rect, label);
            });
    }
}

/// 다음 게임 장면을 빌드합니다.
fn build_next_scene(
    locale: Locale,
    uid: UserId,
    token: LoginToken,
    background_volume: u8,
    effect_volume: u8,
    voice_volume: u8,
    control_sensitivity: f32,
    flip_horizontal: bool,
    flip_vertical: bool,
    player_character: CharacterKind,
    player_team: Team,
    packet: InGameDataInitPacket,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    mesh_pool: MeshPool,
    model_pool: ModelPool,
    motion_pool: MotionPool,
    texture_pool: TexturePool,
    texture_data_pool: TextureDataPool,
    texture_view_pool: TextureViewPool,
    sampler_pool: SamplerPool,
    sound_data_pool: SoundDataPool,
    stage_attributes: Arc<OnceLock<Arc<StageAttributes>>>,
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,
) {
    rayon::spawn(move || {
        let mut world = World::new();
        let stage_attributes = stage_attributes
            .get()
            .expect("the stage attribute data must be preloaded!");

        let task_result = Arc::new(Queue::new());
        let task_result_cloned = task_result.clone();
        rayon::scope(|scope| {
            // 플레이어를 생성합니다.
            let world_ref = &world;
            let device_cloned = device.clone();
            let model_pool_cloned = model_pool.clone();
            let texture_data_pool_cloned = texture_data_pool.clone();
            let players = packet.players;
            let task_result = task_result_cloned.clone();
            create_player_entities(
                scope,
                world_ref,
                device_cloned,
                model_pool_cloned,
                texture_data_pool_cloned,
                task_result,
                players,
            );

            // 지형을 생성합니다.
            let device_cloned = device.clone();
            let model_pool_cloned = model_pool.clone();
            let texture_data_pool_cloned = texture_data_pool.clone();
            let task_result: Arc<Queue<TaskResult>> = task_result_cloned.clone();
            create_stage_entities(
                scope,
                world_ref,
                device_cloned,
                model_pool_cloned,
                texture_data_pool_cloned,
                task_result,
                stage_attributes,
            );

            // 스카이박스를 생성합니다.
            let stage_kind = packet.stage_kind;
            let device_cloned = device.clone();
            let texture_data_pool_cloned = texture_data_pool.clone();
            let task_result: Arc<Queue<TaskResult>> = task_result_cloned.clone();
            create_skybox(
                scope,
                stage_kind,
                device_cloned,
                texture_data_pool_cloned,
                task_result,
            );

            // 조명을 생성합니다.
            let texture_data_pool_cloned = texture_data_pool.clone();
            let task_result: Arc<Queue<TaskResult>> = task_result_cloned.clone();
            create_stage_lights(
                scope,
                texture_data_pool_cloned,
                task_result,
                stage_attributes,
            )
        });

        // 빌더를 생성합니다.
        let mut builder = InGameReadySceneBuilder::new(
            locale,
            uid,
            token,
            background_volume,
            effect_volume,
            voice_volume,
            control_sensitivity,
            flip_horizontal,
            flip_vertical,
            stage_attributes.clone(),
            packet.max_game_play_time_ms,
            packet.half_size_x,
            packet.half_size_y,
            packet.half_size_z,
            player_character,
            player_team,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_pool,
            texture_data_pool,
            texture_view_pool,
            sampler_pool,
            sound_data_pool,
        );

        // 결과를 확인합니다.
        while let Some(result) = task_result.pop() {
            match result {
                TaskResult::Players {
                    uid,
                    entity,
                    archetype,
                    batch_commands,
                } => {
                    builder.insert_player(uid, entity, archetype);
                    for (entity, mut builder) in batch_commands {
                        world
                            .insert(entity, builder.build())
                            .expect("no such entity!");
                    }
                }
                TaskResult::Stage {
                    stage,
                    batch_commands,
                } => {
                    builder.set_stage(stage);
                    for (entity, mut builder) in batch_commands {
                        world
                            .insert(entity, builder.build())
                            .expect("no such entity!");
                    }
                }
                TaskResult::Skybox(skybox) => {
                    builder.set_skybox(skybox);
                }
                TaskResult::DirectionLight(direction_light) => {
                    builder.set_direction_light(direction_light);
                }
                TaskResult::Graphics {
                    command_buffer,
                    staging_buffers,
                } => {
                    queue.submit(Some(command_buffer));
                    drop(staging_buffers);
                }
            }
        }

        // 다음 게임 장면으로 전환합니다.
        let scene = builder.build(world);
        let flow = GameSceneFlow::Change(Box::new(scene));
        let event = AppEvent::AddGameSceneFlow(flow);
        event_loop_proxy.send_event(event).unwrap();
    });
}

/// 플레이어 엔터티를 생성합니다.
fn create_player_entities<'a>(
    scope: &Scope<'a>,
    world: &'a World,
    device: Arc<wgpu::Device>,
    model_pool: ModelPool,
    texture_data_pool: TextureDataPool,
    task_result: Arc<Queue<TaskResult>>,
    players: Vec<InGamePlayerInitData>,
) {
    scope.spawn(move |_| {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut staging_buffers = Vec::new();

        for (i, data) in players.into_iter().enumerate() {
            let uid = data.uid;
            let (archetype, (entity, batch_commands)) = match i {
                0 => (
                    PlayerArchetype::Player0,
                    spawn_player(
                        Player0,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                1 => (
                    PlayerArchetype::Player1,
                    spawn_player(
                        Player1,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                2 => (
                    PlayerArchetype::Player2,
                    spawn_player(
                        Player2,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                3 => (
                    PlayerArchetype::Player3,
                    spawn_player(
                        Player3,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                4 => (
                    PlayerArchetype::Player4,
                    spawn_player(
                        Player4,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                5 => (
                    PlayerArchetype::Player5,
                    spawn_player(
                        Player5,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                6 => (
                    PlayerArchetype::Player6,
                    spawn_player(
                        Player6,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                7 => (
                    PlayerArchetype::Player7,
                    spawn_player(
                        Player7,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                8 => (
                    PlayerArchetype::Player8,
                    spawn_player(
                        Player8,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                9 => (
                    PlayerArchetype::Player9,
                    spawn_player(
                        Player9,
                        world,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &texture_data_pool,
                        &model_pool,
                        data,
                    ),
                ),
                _ => unreachable!("too many players!"),
            };

            // 플레이어 빌드 결과를 전송합니다.
            task_result.push(TaskResult::Players {
                uid,
                entity,
                archetype,
                batch_commands,
            });
        }

        // 그래픽스 처리 결과를 전송합니다.
        task_result.push(TaskResult::Graphics {
            command_buffer: encoder.finish(),
            staging_buffers,
        });
    });
}

// 스테이지를 구성하는 엔터티를 생성합니다.
fn create_stage_entities<'a>(
    scope: &Scope<'a>,
    world: &'a World,
    device: Arc<wgpu::Device>,
    model_pool: ModelPool,
    texture_data_pool: TextureDataPool,
    task_result: Arc<Queue<TaskResult>>,
    stage_attributes: &'a StageAttributes,
) {
    scope.spawn(move |_| {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut staging_buffers = Vec::default();

        let (bvh, batch_commands) = build_stage(
            Some("Stage"),
            world,
            &device,
            &mut encoder,
            &mut staging_buffers,
            &model_pool,
            &texture_data_pool,
            stage_attributes,
        );

        // 스테이지 빌드 결과를 전송합니다.
        task_result.push(TaskResult::Stage {
            stage: bvh,
            batch_commands,
        });

        // 그래픽스 처리 결과를 전송합니다.
        task_result.push(TaskResult::Graphics {
            command_buffer: encoder.finish(),
            staging_buffers,
        });
    });
}

/// 스카이박스를 생성합니다.
fn create_skybox<'a>(
    scope: &Scope<'a>,
    stage_kind: StageKind,
    device: Arc<wgpu::Device>,
    texture_data_pool: TextureDataPool,
    task_result: Arc<Queue<TaskResult>>,
) {
    scope.spawn(move |_| {
        let (texture, sampler) = texture_data_pool
            .get(BG_SKY_URI)
            .expect("the BG_Sky texture must be preloaded!");

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut staging_buffers = Vec::default();

        // 스카이박스 생성 결과를 전송합니다
        task_result.push(TaskResult::Skybox(Skybox::new(
            Some(&format!("Skybox({:?})", &stage_kind)),
            &device,
            &texture,
            &sampler,
            &mut encoder,
            &mut staging_buffers,
        )));

        // 그래픽스 처리 결과를 전송합니다.
        task_result.push(TaskResult::Graphics {
            command_buffer: encoder.finish(),
            staging_buffers,
        });
    });
}

fn create_stage_lights<'a>(
    scope: &Scope<'a>,
    texture_data_pool: TextureDataPool,
    task_result: Arc<Queue<TaskResult>>,
    stage_attributes: &'a StageAttributes,
) {
    scope.spawn(move |_| {
        // 전역 조명 데이터를 가져옵니다.
        let data = match &stage_attributes.global_light {
            Some(light) => light,
            None => return, // 전역 조명 데이터가 없는 경우 조명을 처리하지 않습니다.
        };

        // 그림자 맵 텍스처를 가져옵니다.
        let (shadow_map_view, shadow_sampler) = texture_data_pool
            .get(&data.static_shadow_map)
            .expect("the shadow map texture must be preloaded!");

        // 스테이지 정적 조명 생성 결과를 전송합니다.
        task_result.push(TaskResult::DirectionLight(DirectionLight {
            shadow_map_view,
            shadow_sampler,
            light_proj_view: data.static_light_proj_view.into(),
            direction_w: data.direction_w.into(),
            color: data.color.into(),
        }));
    });
}
