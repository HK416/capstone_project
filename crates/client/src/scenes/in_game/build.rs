use std::sync::{Arc, OnceLock};

use ahash::{HashMap, RandomState};
use hecs::{Entity, EntityBuilder, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        InGamePlayerInitData, LoginToken, StageKind, StageLayoutAttributes, UserId,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{InGameDataInitPacket, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rayon::Scope;
use serde::de;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, StageBoundingVolumn,
        StageBoundingVolumnHierarchy, TextureDataPool, TexturePool, TextureViewPool, BG_SKY_URI,
        NOTOSANS_BOLD,
    },
    component::{
        build_stage, spawn_player, DataArchetype, GlobalLight, Other, Player0, Player1, Player2,
        Player3, Player4, Player5, Player6, Player7, Player8, Player9, Skybox,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
};

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];

/// 작업 결과 데이터입니다.
enum TaskResult {
    Players {
        uid: UserId,
        entity: Entity,
        archetype: DataArchetype,
        batch_commands: Vec<(Entity, EntityBuilder)>,
    },
    Stage {
        bvh: StageBoundingVolumnHierarchy,
        archetype: DataArchetype,
        batch_commands: Vec<(Entity, EntityBuilder)>,
    },
    Skybox {
        skybox: Skybox,
    },
    GlobalLight {
        light: GlobalLight,
    },
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

    /// 초기화 패킷
    packet: Option<InGameDataInitPacket>,
    /// 스테이지 레이아웃 데이터
    stage_layout_data: Arc<OnceLock<StageLayoutAttributes>>,

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
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        packet: InGameDataInitPacket,
        stage_layout_data: Arc<OnceLock<StageLayoutAttributes>>,
        mesh_pool: MeshPool,
        model_pool: ModelPool,
        motion_pool: MotionPool,
        texture_data_pool: TextureDataPool,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            packet: Some(packet),
            stage_layout_data,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
        }
    }
}

impl GameScene for InGameBuildScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let packet = self.packet.take().expect("the packet must be exists!");
        let device = app.render_device().clone();
        let queue = app.render_queue().clone();
        let model_pool = self.model_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let stage_attributes = self.stage_layout_data.clone();
        let event_loop_proxy = app.event_loop_proxy().clone();

        build_next_scene(
            self.locale,
            self.uid,
            self.token,
            packet,
            device,
            queue,
            model_pool,
            texture_data_pool,
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
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(
        &mut self,
        _packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Option<RawPacket> {
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
    packet: InGameDataInitPacket,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    model_pool: ModelPool,
    texture_data_pool: TextureDataPool,
    stage_attributes: Arc<OnceLock<StageLayoutAttributes>>,
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
        });

        // 결과를 확인합니다.
        while let Some(result) = task_result.pop() {
            match result {
                TaskResult::Players {
                    uid,
                    entity,
                    archetype,
                    batch_commands,
                } => {}
                TaskResult::Stage {
                    bvh,
                    archetype,
                    batch_commands,
                } => {}
                TaskResult::Skybox { skybox } => {}
                TaskResult::GlobalLight { light } => {}
                TaskResult::Graphics {
                    command_buffer,
                    staging_buffers,
                } => {}
            }
        }

        println!("!");
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
                    DataArchetype::Player0,
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
                    DataArchetype::Player1,
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
                    DataArchetype::Player2,
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
                    DataArchetype::Player3,
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
                    DataArchetype::Player4,
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
                    DataArchetype::Player5,
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
                    DataArchetype::Player6,
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
                    DataArchetype::Player7,
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
                    DataArchetype::Player8,
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
                    DataArchetype::Player9,
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
    stage_attributes: &'a StageLayoutAttributes,
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
            bvh,
            archetype: DataArchetype::Stage,
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
    _stage_kind: StageKind,
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

        let skybox = Skybox::new(
            Some("Skybox"),
            &device,
            &texture,
            &sampler,
            &mut encoder,
            &mut staging_buffers,
        );

        // 스카이박스 생성 결과를 전송합니다
        task_result.push(TaskResult::Skybox { skybox });

        // 그래픽스 처리 결과를 전송합니다.
        task_result.push(TaskResult::Graphics {
            command_buffer: encoder.finish(),
            staging_buffers,
        });
    });
}

// fn spawn_global_light<'a>(
//     scope: &Scope<'a>,
//     texture_data_pool: TextureDataPool,
//     stage_attributes: &'a StageLayoutAttributes,
// ) {
// }
