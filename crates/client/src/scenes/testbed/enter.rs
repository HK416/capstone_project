use std::{error::Error, fmt, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{CharacterKind, ClientId, ObjectId},
    EnterStagePacket, InitStagePacket, PacketType, RawPacket,
};
use mod_parallelism::collections::Queue;
use mod_render::{
    GraphicsPipelinePool, ScreenDescriptor, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT,
};
use parking_lot::Mutex;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::{ModelHierarchyPool, MotionPool},
    channel::TaskResultChannel,
    component::{aris_original, spawn_player_character},
    config::UserConfig,
    render::{
        create_character_render_pipeline, create_student_halo_render_pipeline,
        CHARACTER_HALO_PIPELINE_NAME, CHARACTER_PIPELINE_NAME,
    },
    SERVER_ADDR,
};

use super::TestbedInGameScene;

/// 게임 월드에 접속을 요청하는 게임 장면입니다.
pub struct EnterStageScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,
    /// 선택한 캐릭터 종류
    character_kind: CharacterKind,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl EnterStageScene {
    /// 새로운 `EnterStageScene`을 생성합니다.
    ///
    /// # Panics
    /// 주어진 클라이언트 식별자가 유효하지 않는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        user_config: Box<UserConfig>,
        client_id: ClientId,
        character_kind: CharacterKind,
    ) -> Self {
        assert_ne!(client_id, ClientId::NULL, "invalid client id");
        Self {
            user_config: Some(user_config),
            client_id,
            character_kind,
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 인터페이스 콜백 함수
    #[allow(unused_variables)]
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        let connect_server_text = egui::RichText::new("서버에 연결 중...")
            .color(egui::Color32::WHITE)
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.label(connect_server_text);
                });
            });

        Ok(())
    }
}

impl GameScene for EnterStageScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 월드 접속 패킷 전송
        let net_manager = app.net_manager();
        let socket = net_manager.get(&SERVER_ADDR).expect("no such socket");
        let packet = EnterStagePacket::new(self.client_id, self.character_kind);
        let packet = packet.as_raw();
        socket.push_packet(packet);

        Ok(())
    }

    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        match packet.packet_type() {
            PacketType::InitStage => {
                let init_stage_packet = InitStagePacket::from_raw(packet);
                let proxy = app.event_loop_proxy();
                let user_config = self.user_config.take().expect("duplicate packet received");
                let next_scene =
                    LoadStageResourceScene::new(user_config, self.client_id, init_stage_packet);
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                proxy.send_event(event).unwrap();
            }
            PacketType::PullStage => {
                log::info!("received pull stage packet");
            }
            _ => panic!("invalid packet"),
        }

        Ok(())
    }

    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();

        // 윈도우 창 설명자를 생성합니다.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 검은색 화면에 오른쪽 하단에 상태를 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(EnterStageScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }
}

impl fmt::Debug for EnterStageScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(StageEnterScene))
    }
}

/// 게임 월드 리소스를 로드하는 게임 장면입니다.
pub struct LoadStageResourceScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,
    /// 게임 월드 초기화 패킷 데이터
    init_stage_packet: Option<InitStagePacket>,

    /// 작업의 개수
    num_tasks: usize,
    /// 작업 결과 전송 채널
    task_result_channel: TaskResultChannel<()>,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl LoadStageResourceScene {
    /// 새로운 `LoadStageResourceScene`을 생성합니다.
    ///
    /// # Panics
    /// 주어진 클라이언트 식별자가 유효하지 않는 경우 [`panic!`]을 호출합니다.
    ///
    fn new(
        user_config: Box<UserConfig>,
        client_id: ClientId,
        init_stage_packet: InitStagePacket,
    ) -> Self {
        assert_ne!(client_id, ClientId::NULL, "invalid client id");
        Self {
            user_config: Some(user_config),
            client_id,
            init_stage_packet: Some(init_stage_packet),
            num_tasks: 0,
            task_result_channel: TaskResultChannel::new(),
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 인터페이스 콜백 함수
    #[allow(unused_variables)]
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        let connect_server_text = egui::RichText::new("게임 리소스 로드 중...")
            .color(egui::Color32::WHITE)
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.label(connect_server_text);
                });
            });

        Ok(())
    }
}

impl GameScene for LoadStageResourceScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 월드 초기화 패킷 데이터를 가져옵니다.
        let init_stage_packet = self
            .init_stage_packet
            .as_ref()
            .expect("packet data must exist");

        // 게임 월드에 존재하는 캐릭터 모델 데이터를 로드합니다.
        let pool = app.io_threads();
        let num_player = init_stage_packet.num_players as usize;
        let players = &init_stage_packet.players;
        for index in 0..num_player {
            let character_kind = players[index].character_kind;
            let device = app.render_device().clone();
            let queue = app.render_queue().clone();
            let channel = self.task_result_channel.clone();
            let asset_manager = app.asset_manager().clone();
            load_character_model(pool, device, queue, channel, asset_manager, character_kind);
            self.num_tasks += 3;
        }

        // 캐릭터 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(CHARACTER_PIPELINE_NAME, move || {
            create_character_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
        });

        // 캐릭터 헤일로 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(CHARACTER_HALO_PIPELINE_NAME, move || {
            create_student_halo_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
        });

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 기다립니다.
        if let Some(result) = self.task_result_channel.recv() {
            self.num_tasks -= 1;
            result?;
        }

        // 모든 작업이 끝난 경우 다음 게임 장면으로 전환합니다.
        if self.num_tasks == 0 {
            let mut pair = self.user_config.take().zip(self.init_stage_packet.take());
            if let Some((user_config, init_stage_packet)) = pair.take() {
                let next_scene =
                    InitStageScene::new(user_config, self.client_id, init_stage_packet);
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let proxy = app.event_loop_proxy();
                proxy.send_event(event).unwrap();
            }
        }

        Ok(())
    }

    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();

        // 윈도우 창 설명자를 생성합니다.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 검은색 화면에 오른쪽 하단에 상태를 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(LoadStageResourceScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }
}

impl fmt::Debug for LoadStageResourceScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LoadStageScene))
    }
}

/// 주어진 스레드 풀에서 캐릭터 모델 리소스를 로드합니다.
/// 작업 결과를 주어진 작업 결과 채널로 전송합니다.
fn load_character_model(
    pool: &ThreadPool,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    channel: TaskResultChannel<()>,
    asset_manager: AssetManager,
    character_kind: CharacterKind,
) {
    // 캐릭터 종류에 따른 모델 정보를 가져옵니다.
    let (workspace, model_name, model_halo_name) = match character_kind {
        CharacterKind::ArisOriginal => (
            aris_original::WORKSPACE,
            aris_original::MODEL_NAME,
            aris_original::MODEL_HALO_NAME,
        ),
        CharacterKind::MomoiOriginal => todo!(),
    };

    // 캐릭터 애니메이션 데이터를 로드합니다.
    let channel_cloned = channel.clone();
    let asset_manager_cloned = asset_manager.clone();
    pool.spawn(move || {
        let result = MotionPool::get_or_init(model_name, workspace, &asset_manager_cloned);
        channel_cloned.send(result.map(|_| ()));
    });

    // 캐릭터 모델 데이터를 로드합니다.
    let device_cloned = device.clone();
    let queue_cloned = queue.clone();
    let channel_cloned = channel.clone();
    let asset_manager_cloned = asset_manager.clone();
    pool.spawn(move || {
        let result = ModelHierarchyPool::get_or_init(
            model_name,
            workspace,
            &asset_manager_cloned,
            &device_cloned,
            &queue_cloned,
        );
        channel_cloned.send(result.map(|_| ()));
    });

    // 캐릭터 헤일로 모델 데이터를 로드합니다.
    let device_cloned = device.clone();
    let queue_cloned = queue.clone();
    let channel_cloned = channel.clone();
    let asset_manager_cloned = asset_manager.clone();
    pool.spawn(move || {
        let result = ModelHierarchyPool::get_or_init(
            model_halo_name,
            workspace,
            &asset_manager_cloned,
            &device_cloned,
            &queue_cloned,
        );
        channel_cloned.send(result.map(|_| ()));
    });
}

/// 게임 월드를 생성하는 게임 장면입니다.
pub struct InitStageScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,
    /// 게임 월드 초기화 패킷 데이터
    init_stage_packet: Option<InitStagePacket>,

    /// 작업 결과 전송 채널
    task_result_channel: TaskResultChannel<(World, HashMap<ObjectId, Entity>)>,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl InitStageScene {
    /// 새로운 `InitStageScene`을 생성합니다.
    ///
    /// # Panics
    /// 주어진 클라이언트 식별자가 유효하지 않는 경우 [`panic!`]을 호출합니다.
    ///
    fn new(
        user_config: Box<UserConfig>,
        client_id: ClientId,
        init_stage_packet: InitStagePacket,
    ) -> Self {
        assert_ne!(client_id, ClientId::NULL, "invalid client id");
        Self {
            user_config: Some(user_config),
            client_id,
            init_stage_packet: Some(init_stage_packet),
            task_result_channel: TaskResultChannel::default(),
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 인터페이스 콜백 함수
    #[allow(unused_variables)]
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        let connect_server_text = egui::RichText::new("게임 월드 생성 중...")
            .color(egui::Color32::WHITE)
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.label(connect_server_text);
                });
            });

        Ok(())
    }
}

impl GameScene for InitStageScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        spawn_players(
            app.asset_manager().clone(),
            app.render_device().clone(),
            app.render_queue().clone(),
            self.init_stage_packet.take().expect("packet must exist"),
            self.task_result_channel.clone(),
        );
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 사용자 구성 데이터가 없는 경우 함수 실행을 생략합니다.
        if self.user_config.is_none() {
            return Ok(());
        }

        // 작업 처리 결과를 대기합니다.
        if let Some(result) = self.task_result_channel.recv() {
            let (world, entities) = result?;
            let user_config = self
                .user_config
                .take()
                .expect("user configuration must exist");
            let next_scene = TestbedInGameScene::new(user_config, self.client_id, world, entities);
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let proxy = app.event_loop_proxy();
            proxy.send_event(event).unwrap();
        }

        Ok(())
    }

    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();

        // 윈도우 창 설명자를 생성합니다.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 검은색 화면에 오른쪽 하단에 상태를 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InitStageScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }
}

impl fmt::Debug for InitStageScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(InitStageScene))
    }
}

/// 게임 월드에 존재하는 플레이어를 생성하는 함수입니다.
fn spawn_players(
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    init_stage_packet: InitStagePacket,
    task_result_channel: TaskResultChannel<(World, HashMap<ObjectId, Entity>)>,
) {
    type LocalResult = (ObjectId, Entity, Vec<(Entity, EntityBuilder)>);
    rayon::spawn(move || {
        let mut world = World::new();
        let mut entities = HashMap::default();
        let local_result_channel: TaskResultChannel<LocalResult> = TaskResultChannel::default();

        let init_stage_packet = &init_stage_packet;
        let num_players = init_stage_packet.num_players as usize;
        let mut num_tasks = num_players;

        {
            let world = &world;
            let device = &device;
            let queue = &queue;
            let asset_manager = &asset_manager;
            let local_result_channel = &local_result_channel;
            rayon::scope(move |scope| {
                for i in 0..num_players {
                    let player_data = &init_stage_packet.players[i];
                    scope.spawn(move |_| {
                        let result = spawn_player_character(
                            player_data,
                            asset_manager,
                            device,
                            queue,
                            world,
                        )
                        .map(|(entity, batch_commands)| (player_data.id, entity, batch_commands));
                        local_result_channel.send(result);
                    });
                }
            });
        }

        while num_tasks > 0 {
            if let Some(result) = local_result_channel.recv() {
                match result {
                    Ok((id, entity, batch_commands)) => {
                        for (entity, mut builder) in batch_commands {
                            world
                                .insert(entity, builder.build())
                                .expect("no such entity");
                        }
                        entities.insert(id, entity);
                    }
                    Err(e) => {
                        task_result_channel.send_err(e);
                        return;
                    }
                }
                num_tasks -= 1;
            }
        }

        task_result_channel.send_ok((world, entities));
    });
}
