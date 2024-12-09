use std::{error::Error, fmt, net::ToSocketAddrs, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    net::IpAddress,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{InitPacket, PacketType, Player, RawPacket};
use mod_parallelism::collections::Queue;
use mod_render::{
    GraphicsPipelinePool, ScreenDescriptor, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT,
};
use winit::window::Window;

use crate::{
    asset::{ModelHierarchyPool, MotionPool},
    component::{
        create_student_render_pipeline, spawn_student, AnimationTimer, StudentBehaviorState,
        StudentKind,
    },
    scenes::TestbedInGameScene,
};

/// ## Testbed Enter Scene State
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum State {
    First,
    Second,
}

type LoadResult = Result<(), Box<dyn Error + Send>>;
type SpawnResult = Result<(u32, HashMap<u32, Entity>, World), Box<dyn Error + Send>>;
type LocalResult = Result<(u32, Entity, Vec<(Entity, EntityBuilder)>), Box<dyn Error + Send>>;

/// ## Testbed Enter Scene
pub struct TestbedEnterScene {
    /// 사용자의 학생 종류
    /// <차후 사용 예정>
    _kind: StudentKind,

    /// 월드 생성 결과 대기열
    channel: Arc<Queue<SpawnResult>>,

    /// 작업 결과 대기열
    results: Arc<Queue<LoadResult>>,

    /// 남은 작업의 개수
    num_tasks: usize,

    /// 작업 상태
    state: State,

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedEnterScene {
    pub fn new(kind: StudentKind) -> Self {
        Self {
            _kind: kind,
            channel: Arc::new(Queue::new()),
            results: Arc::new(Queue::new()),
            num_tasks: 0,
            state: State::First,
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 인터페이스 콜백 함수입니다.
    #[allow(unused_variables)]
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        let loading_text = egui::RichText::new("Loading...")
            .color(egui::Color32::WHITE)
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    let text_label = match self.state {
                        State::First => egui::Label::new("게임 세상에 접속 중..."),
                        State::Second => egui::Label::new("게임 세상을 만드는 중..."),
                    };
                    ui.add(text_label);
                    ui.label(loading_text);
                });
            });

        Ok(())
    }
}

impl GameScene for TestbedEnterScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let pool = app.io_threads();

        // 게임 서버 연결
        let results = self.results.clone();
        let net_manager = app.net_manager().clone();
        pool.spawn(move || {
            let address = "localhost:7878".to_socket_addrs().unwrap().next().unwrap();
            let result = net_manager
                .connect(&IpAddress::Tcp(address))
                .map(|_| {})
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            results.push(result);
        });
        self.num_tasks += 1;

        // `Aris_Original` 모델 로드
        let results = self.results.clone();
        let device = app.render_device().clone();
        let queue = app.render_queue().clone();
        let asset_manager = app.asset_manager().clone();
        pool.spawn(move || {
            let result = ModelHierarchyPool::get_or_init(
                "aris_original",
                "characters/aris_original",
                &asset_manager,
                &device,
                &queue,
            )
            .map(|_| {})
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            results.push(result);
        });
        self.num_tasks += 1;

        // `Aris_Original_Halo` 모델 로드
        let results = self.results.clone();
        let device = app.render_device().clone();
        let queue = app.render_queue().clone();
        let asset_manager = app.asset_manager().clone();
        pool.spawn(move || {
            let result = ModelHierarchyPool::get_or_init(
                "aris_original_halo",
                "characters/aris_original",
                &asset_manager,
                &device,
                &queue,
            )
            .map(|_| {})
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            results.push(result);
        });
        self.num_tasks += 1;

        // `Aris_Original` 애니메이션 로드
        let results = self.results.clone();
        let asset_manager = app.asset_manager().clone();
        pool.spawn(move || {
            let result = MotionPool::get_or_init(
                "aris_original",
                "characters/aris_original",
                &asset_manager,
            )
            .map(|_| {})
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            results.push(result);
        });
        self.num_tasks += 1;

        // `Student` 렌더링 파이프라인 생성
        let results = self.results.clone();
        let device = app.render_device().clone();
        pool.spawn(move || {
            GraphicsPipelinePool::get_or_init("student", move || {
                create_student_render_pipeline(&device, DEPTH_FORMAT, SWAPCHAIN_FORMAT)
            });
            results.push(Ok(()));
        });
        self.num_tasks += 1;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        match self.state {
            State::First => {
                if let Some(result) = self.results.pop() {
                    self.num_tasks -= 1;
                    result?;
                }

                if self.num_tasks == 0 {
                    log::debug!("TestbedEnterScene :: 장면에 필요한 리소스를 로드함.");
                    self.state = State::Second;
                }
            }
            State::Second => {
                if let Some(result) = self.channel.pop() {
                    log::debug!("TestbedEnterScene :: 장면을 생성함.");
                    let (client_id, entities, world) = result?;
                    log::debug!("TestbedEnterScene :: 다음 장면으로 이동.");
                    let proxy = app.event_loop_proxy();
                    proxy
                        .send_event(AppEvent::SetGameSceneFlow(GameSceneFlow::Change(Box::new(
                            TestbedInGameScene::new(client_id, entities, world),
                        ))))
                        .unwrap();
                }
            }
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        if packet.packet_type() == PacketType::INIT {
            let packet = InitPacket::from_raw(packet);
            spawn_world_objects(
                packet,
                self.channel.clone(),
                app.asset_manager().clone(),
                app.render_device().clone(),
                app.render_queue().clone(),
            );
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

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TestbadEnterScene)"),
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
                    depth_ops: None,
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

impl fmt::Debug for TestbedEnterScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestbedEnterScene))
    }
}

/// 게임 세상을 생성합니다.
fn spawn_world_objects(
    packet: InitPacket,
    channel: Arc<Queue<SpawnResult>>,
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
) {
    let world = World::new();
    let client_id = packet.client_id;
    let num_entities = packet.world.len();
    let local_channel: Arc<Queue<LocalResult>> = Arc::new(Queue::new());
    rayon::scope(|scope| {
        for player in packet.world.iter() {
            let local_channel = local_channel.clone();
            let asset_manager = asset_manager.clone();
            let device = device.clone();
            let queue = queue.clone();
            let world = &world;
            let data = player;
            scope.spawn(move |_| {
                local_channel.push(spawn_entities(data, asset_manager, device, queue, world));
            });
        }
    });

    channel.push(poll_results(local_channel, num_entities, client_id, world));
}

/// 게임 세상에 존재하는 `Entity`를 생성합니다.
fn spawn_entities(
    data: &Player,
    asset_manager: AssetManager,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    world: &World,
) -> LocalResult {
    let (entity, batch_commands) = spawn_student(
        world,
        &asset_manager,
        &device,
        &queue,
        StudentKind::ArisOriginal,  // 차후 패킷에 포함되어야 함.
        StudentBehaviorState::Idle, // 차후 패킷 수정이 필요
        AnimationTimer(data.anim_timer),
        glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 1.0)),
        // glam::Mat4::from_rotation_translation(
        //     glam::quat(
        //         data.rotation.x,
        //         data.rotation.y,
        //         data.rotation.z,
        //         data.rotation.w,
        //     ),
        //     glam::vec3(data.translation.x, data.translation.y, data.translation.z),
        // )
    )
    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
    Ok((data.id, entity, batch_commands))
}

fn poll_results(
    local_channel: Arc<Queue<LocalResult>>,
    mut num_entities: usize,
    client_id: u32,
    mut world: World,
) -> SpawnResult {
    let mut entities = HashMap::default();
    while num_entities > 0 {
        core::hint::spin_loop();
        if let Some(result) = local_channel.pop() {
            let (id, entity, batch_commands) = result?;
            for (entity, mut builder) in batch_commands {
                world.insert(entity, builder.build()).unwrap();
            }
            entities.insert(id, entity);
            num_entities -= 1;
        }
    }

    Ok((client_id, entities, world))
}
