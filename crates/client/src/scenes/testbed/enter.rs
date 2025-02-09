use std::{error::Error, fmt, io::Cursor, sync::Arc};

use ahash::HashMap;
use ddsfile::Dds;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{BulletKind, CharacterKind, ClientId, Epoch, ObjectId},
    protocol::{EnterStagePacket, InitStagePacket, Packet, PacketType, RawPacket},
};
use mod_render::{
    GraphicsPipelinePool, SamplerPool, ScreenDescriptor, SkyboxResource, TexturePool,
    TextureViewPool, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT,
};
use rayon::ThreadPool;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    asset::{ModelAssetError, ModelHierarchyPool},
    channel::TaskResultChannel,
    component::{load_bullet_model, load_character_model, spawn_player_character, spawn_terrain},
    config::UserConfig,
    render::{
        create_bullet_render_pipeline, create_character_halo_render_pipeline,
        create_character_render_pipeline, create_fx_damage_render_pipeline,
        create_skybox_render_pipeline, create_terrain_render_pipeline, skybox,
        BULLET_PIPELINE_NAME, CHARACTER_HALO_PIPELINE_NAME, CHARACTER_PIPELINE_NAME,
        FX_DAMAGE_PIPELINE_NAME, SKYBOX_PIPELINE_NAME, TERRAIN_PIPELINE_NAME,
    },
    SERVER_TCP_ADDR,
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
        let socket = net_manager.get(&SERVER_TCP_ADDR).expect("no such socket");
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

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

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
        load_all_character_models(
            app.io_threads(),
            app.asset_manager().clone(),
            app.render_device().clone(),
            app.render_queue().clone(),
            self.task_result_channel.clone(),
            &mut self.num_tasks,
        );

        // 게임 월드에 존재하는 캐릭터 모델의 총알 모델 데이터를 로드합니다.
        load_all_bullet_models(
            app.io_threads(),
            app.asset_manager().clone(),
            app.render_device().clone(),
            app.render_queue().clone(),
            self.task_result_channel.clone(),
            &mut self.num_tasks,
        );

        // Skybox 텍스처 데이터를 로드합니다.
        load_skybox_texture(
            app.io_threads(),
            app.asset_manager().clone(),
            self.task_result_channel.clone(),
            &mut self.num_tasks,
        );

        // 게임 월드 지형 모델 데이터를 로드합니다.
        // FIXME: 추후 수정 필요
        load_terrain_models(
            app.io_threads(),
            app.asset_manager().clone(),
            app.render_device().clone(),
            app.render_queue().clone(),
            self.task_result_channel.clone(),
            &mut self.num_tasks,
        );

        // 데미지 폰트 텍스처를 로드합니다.
        load_damage_font(
            app.io_threads(),
            app.asset_manager().clone(),
            self.task_result_channel.clone(),
            &mut self.num_tasks,
        );

        // 캐릭터 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(CHARACTER_PIPELINE_NAME, move || {
            create_character_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
        });

        // 캐릭터 헤일로 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(CHARACTER_HALO_PIPELINE_NAME, move || {
            create_character_halo_render_pipeline(
                app.render_device(),
                DEPTH_FORMAT,
                SWAPCHAIN_FORMAT,
            )
        });

        // 총알 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(BULLET_PIPELINE_NAME, move || {
            create_bullet_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
        });

        // 지형 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(TERRAIN_PIPELINE_NAME, move || {
            create_terrain_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
        });

        // Skybox 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(SKYBOX_PIPELINE_NAME, move || {
            create_skybox_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
        });

        // 데미지 파티클 렌더링 파이프라인을 생성합니다.
        GraphicsPipelinePool::get_or_init(FX_DAMAGE_PIPELINE_NAME, move || {
            create_fx_damage_render_pipeline(app.render_device(), DEPTH_FORMAT, SWAPCHAIN_FORMAT)
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

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl fmt::Debug for LoadStageResourceScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LoadStageScene))
    }
}

/// 모든 캐릭터 모델을 로드합니다.
fn load_all_character_models(
    pool: &ThreadPool,
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    channel: TaskResultChannel<()>,
    num_tasks: &mut usize,
) {
    // ArisOriginal 모델을 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            channel.send(load_character_model(
                &asset_manager,
                CharacterKind::ArisOriginal,
                &device,
                &queue,
            ));
        });
        *num_tasks += 1;
    }

    // MomoiOriginal 모델을 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            channel.send(load_character_model(
                &asset_manager,
                CharacterKind::MomoiOriginal,
                &device,
                &queue,
            ));
        });
        *num_tasks += 1;
    }
}

/// 모든 총알 모델을 로드합니다.
fn load_all_bullet_models(
    pool: &ThreadPool,
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    channel: TaskResultChannel<()>,
    num_tasks: &mut usize,
) {
    // Common 총알 모델을 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            channel.send(load_bullet_model(
                &asset_manager,
                BulletKind::Common,
                &device,
                &queue,
            ));
        });
        *num_tasks += 1;
    }
    // ArisOriginal 총알 모델을 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            channel.send(load_bullet_model(
                &asset_manager,
                BulletKind::ArisOriginal,
                &device,
                &queue,
            ));
        });
        *num_tasks += 1;
    }
}

/// Skybox 텍스처를 로드합니다.
fn load_skybox_texture(
    pool: &ThreadPool,
    asset_manager: AssetManager,
    channel: TaskResultChannel<()>,
    num_tasks: &mut usize,
) {
    pool.spawn(move || {
        let path = format!("{}/{}.dds", skybox::WORKSPACE, skybox::TEXTURE_NAME);
        channel.send(asset_manager.load(&path).map(|_| ()));
    });
    *num_tasks += 1;
}

/// 지형 모델 데이터를 로드합니다.
fn load_terrain_models(
    pool: &ThreadPool,
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    channel: TaskResultChannel<()>,
    num_tasks: &mut usize,
) {
    // 교차로 모델을 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            let result = ModelHierarchyPool::get_or_init(
                "CrossroadPlane",
                "stage/terrain",
                &asset_manager,
                &device,
                &queue,
            );
            channel.send(result.map(|_| ()));
        });
        *num_tasks += 1;
    }

    // 도로 모델을 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            let result = ModelHierarchyPool::get_or_init(
                "RoadPlane",
                "stage/terrain",
                &asset_manager,
                &device,
                &queue,
            );
            channel.send(result.map(|_| ()));
        });
        *num_tasks += 1;
    }

    // 평면 모델을 로드합니다.
    // {
    //     let asset_manager = asset_manager.clone();
    //     let device = device.clone();
    //     let queue = queue.clone();
    //     let channel = channel.clone();
    //     pool.spawn(move || {
    //         let result = ModelHierarchyPool::get_or_init(
    //             "Plane",
    //             "stage/terrain",
    //             &asset_manager,
    //             &device,
    //             &queue,
    //         );
    //         channel.send(result.map(|_| ()));
    //     });
    //     *num_tasks += 1;
    // }
}

fn load_damage_font(
    pool: &ThreadPool,
    asset_manager: AssetManager,
    channel: TaskResultChannel<()>,
    num_tasks: &mut usize,
) {
    // 데미지 폰트 텍스처를 로드합니다.
    {
        let asset_manager = asset_manager.clone();
        let channel = channel.clone();
        pool.spawn(move || {
            let path = "font/D_Font_Normal.dds";
            channel.send(asset_manager.load(&path).map(|_| ()));
        });
        *num_tasks += 1;
    }
}

/// 게임 월드를 생성하는 게임 장면입니다.
pub struct InitStageScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,
    /// 플레이어 캐릭터 오브젝트 식별자
    object_id: ObjectId,
    /// 서버의 Epoch
    epoch: Epoch,
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
            object_id: init_stage_packet.object_id,
            epoch: init_stage_packet.epoch,
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
        create_game_world(
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

            // Skybox 쉐이더 리소스를 생성합니다.
            let skybox_resource = create_skybox_resource(
                app.asset_manager(),
                app.render_device(),
                app.render_queue(),
            )
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

            let user_config = self
                .user_config
                .take()
                .expect("user configuration must exist");
            let next_scene = TestbedInGameScene::new(
                user_config,
                self.client_id,
                self.object_id,
                self.epoch,
                world,
                entities,
                skybox_resource,
            );
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

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl fmt::Debug for InitStageScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(InitStageScene))
    }
}

/// 스레드 풀간 데이터 전송을 위한 채널 데이터입니다.
type LocalResult = (ObjectId, Entity, Vec<(Entity, EntityBuilder)>);

/// 게임 월드를 생성합니다.
fn create_game_world(
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    init_stage_packet: InitStagePacket,
    task_result_channel: TaskResultChannel<(World, HashMap<ObjectId, Entity>)>,
) {
    rayon::spawn(move || {
        let mut world = World::default();
        let mut entities = HashMap::default();

        let mut num_tasks = init_stage_packet.num_players as usize;
        let channel: TaskResultChannel<LocalResult> = TaskResultChannel::default();
        {
            let asset_manager = asset_manager.clone();
            let device = device.clone();
            let queue = queue.clone();
            let world = &world;
            let channel = channel.clone();
            rayon::scope(move |_| {
                spawn_players(
                    world,
                    asset_manager,
                    device,
                    queue,
                    init_stage_packet,
                    channel,
                );
            });
        }

        match spawn_terrains(&world, &asset_manager, &device, &queue) {
            Ok(batch_commands) => {
                for (entity, mut builder) in batch_commands {
                    world
                        .insert(entity, builder.build())
                        .expect("no such entity");
                }
            }
            Err(e) => task_result_channel.send_err(Box::new(e)),
        }

        while num_tasks > 0 {
            if let Some(result) = channel.recv() {
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

/// 게임 월드에 존재하는 플레이어를 생성하는 함수입니다.
fn spawn_players(
    world: &World,
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    packet: InitStagePacket,
    channel: TaskResultChannel<LocalResult>,
) {
    for player in packet.players.into_iter() {
        let world = world;
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        let channel = channel.clone();
        let result = spawn_player_character(&player, &asset_manager, &device, &queue, world)
            .map(|(entity, batch_commands)| (player.object_id, entity, batch_commands));
        channel.send(result);
    }
}

/// 게임 월드에 존재하는 지형을 생성하는 함수입니다.
fn spawn_terrains<'a>(
    world: &'a World,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Vec<(Entity, EntityBuilder)>, ModelAssetError> {
    // FIXME: 에셋 피벗이 잘못 설정되어있음

    let mut total_batch_commands = Vec::new();
    let (_, mut batch_commands) = spawn_terrain(
        "CrossroadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::IDENTITY,
        glam::vec3(30.0, 0.0, 0.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "CrossroadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::IDENTITY,
        glam::vec3(-30.0, 0.0, 0.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::IDENTITY,
        glam::vec3(-30.0, 0.0, 15.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::IDENTITY,
        glam::vec3(-30.0, 0.0, -15.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::IDENTITY,
        glam::vec3(30.0, 0.0, 15.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::IDENTITY,
        glam::vec3(30.0, 0.0, -15.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::from_rotation_y(90f32.to_radians()),
        glam::vec3(0.0, 0.0, 0.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::from_rotation_y(90f32.to_radians()),
        glam::vec3(15.0, 0.0, 0.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    let (_, mut batch_commands) = spawn_terrain(
        "RoadPlane",
        "stage/terrain",
        glam::Vec3::ONE,
        glam::Quat::from_rotation_y(90f32.to_radians()),
        glam::vec3(-15.0, 0.0, 0.0),
        &asset_manager,
        &device,
        &queue,
        world,
    )?;
    total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Road",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::from_rotation_y(90f32.to_radians()),
    //     glam::vec3(60.0, 0.0, -15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    // total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Plane",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::IDENTITY,
    //     glam::vec3(0.0, 0.0, 15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    // total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Plane",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::IDENTITY,
    //     glam::vec3(0.0, 0.0, -15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    // total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Plane",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::IDENTITY,
    //     glam::vec3(15.0, 0.0, 15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    // total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Plane",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::IDENTITY,
    //     glam::vec3(15.0, 0.0, -15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    // total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Plane",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::IDENTITY,
    //     glam::vec3(-15.0, 0.0, 15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    // total_batch_commands.append(&mut batch_commands);

    // let (_, mut batch_commands) = spawn_terrain(
    //     "Plane",
    //     "stage/terrain",
    //     glam::Vec3::ONE,
    //     glam::Quat::IDENTITY,
    //     glam::vec3(-15.0, 0.0, -15.0),
    //     &asset_manager,
    //     &device,
    //     &queue,
    //     world,
    // )?;
    total_batch_commands.append(&mut batch_commands);

    Ok(total_batch_commands)
}

/// Skybox 쉐이더 리소스를 로드합니다.
fn create_skybox_resource(
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Arc<SkyboxResource>, ModelAssetError> {
    let texture = TexturePool::get_or_init(
        skybox::TEXTURE_NAME,
        move || -> Result<Arc<wgpu::Texture>, ModelAssetError> {
            let path = format!("{}/{}.dds", skybox::WORKSPACE, skybox::TEXTURE_NAME);
            let cached_asset = asset_manager
                .get_or_init(&path)
                .map_err(|e| ModelAssetError::from(e))?;

            let dds = Dds::read(Cursor::new(cached_asset.as_bytes()))
                .map_err(|e| ModelAssetError::from(e))?;

            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&"Texture(Skybox)"),
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
                wgpu::util::TextureDataOrder::LayerMajor,
                &dds.data,
            );

            asset_manager.remove(path);
            Ok(Arc::new(texture))
        },
    )?;

    let t_skybox = TextureViewPool::get_or_init(
        &texture,
        &wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        },
    );
    let s_skybox = SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default());

    Ok(Arc::new(SkyboxResource::uninit(
        Some("Skybox"),
        device,
        &t_skybox,
        &s_skybox,
    )))
}
