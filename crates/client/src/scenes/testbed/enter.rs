use std::{error::Error, fmt, net::ToSocketAddrs, sync::Arc};

use hecs::World;
use mod_app::{app::AppHandle, net::IpAddress, scene::GameScene};
use mod_network::{InitPacket, PacketType, RawPacket};
use mod_parallelism::collections::Queue;
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::asset::{ModelHierarchyPool, MotionPool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum State {
    First,
    Second,
}

/// ## Testbed Enter Scene
pub struct TestbedEnterScene {
    /// 사용자의 클라이언트 식별자
    child_id: Option<u32>, 

    /// 작업 결과 대기열
    results: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,

    /// 남은 작업의 개수
    num_tasks: usize,

    /// 작업 상태
    state: State, 

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedEnterScene {
    pub fn new() -> Self {
        Self {
            child_id: None, 
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

        let results = self.results.clone();
        let device = app.render_device().clone();
        let queue = app.render_queue().clone();
        let asset_manager = app.asset_manager().clone();
        pool.spawn(move || {
            let result = ModelHierarchyPool::load(
                "aris_original",
                "characters/aris_original",
                &asset_manager,
                &device,
                &queue,
            )
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            results.push(result);
        });
        self.num_tasks += 1;

        let results = self.results.clone();
        let asset_manager = app.asset_manager().clone();
        pool.spawn(move || {
            let result = MotionPool::get_or_init(
                "aris_original",
                "characters/aris_original",
                &asset_manager,
                |_| {},
            )
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            results.push(result);
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
                    self.state = State::Second;
                }
            }, 
            State::Second => {
                if let Some(client_id) = self.child_id.take() {
                    println!("enter game world");
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
            self.child_id = Some(packet.client_id);
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
