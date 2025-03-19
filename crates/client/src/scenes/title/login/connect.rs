use std::{error::Error, sync::Arc};

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetManager,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::{ConnectPacket, Packet, PacketType, RawPacket};
use mod_parallelism::collections::Queue;
use mod_render::{CameraResource, ScreenDescriptor, UiRenderer};
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::NOTOSANS_REGULAR,
    config::{Locale, UserConfig, NUM_LOCALE},
    render::BackgroundResource,
    scenes::BASE_WIDTH,
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 Main 텍스트
const MAIN_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결 중..."];

/// 게임 로그인 타이틀 화면을 표시하는 장면입니다.  
/// 게임 서버와 연결을 시도 후 로그인을 시도합니다.
pub struct GameLoginConnectScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 메인 카메라 리소스입니다.
    main_camera: Arc<CameraResource>,
    /// 배경 리소스입니다.
    background: Arc<BackgroundResource>,
    /// 작업 결과를 저장하는 대기열
    task_result: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,
    /// 사용자 정보와 로그인 토큰이 담긴 패킷입니다.
    connect_packet: Option<ConnectPacket>,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl GameLoginConnectScene {
    /// 새로운 `GameLoginConnectScene`을 생성합니다.
    pub fn new(main_camera: Arc<CameraResource>, background: Arc<BackgroundResource>) -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale,
            main_camera,
            background,
            task_result: Arc::new(Queue::new()),
            connect_packet: None,
            egui_clip_primitives: Vec::default(),
            egui_free_texture_ids: Vec::default(),
        }
    }

    /// UI 콜백 함수
    fn ui_callback(&mut self, window: &Window, egui_ctx: &egui::Context) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let main_font_id = egui::FontId::new(16.0 * scale, main_font_family);
        let main_font_color = egui::Color32::BLACK;

        // 텍스트
        let i = self.locale as usize;
        let text = MAIN_TEXTS[i];
        let head_text = egui::RichText::new(text)
            .font(main_font_id)
            .color(main_font_color);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -24.0 * scale])
            .show(egui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(head_text);
                })
            });
    }

    /// 게임 서버와 연결을 시도합니다.
    fn try_connect_game_server(&mut self, thread_pool: &ThreadPool, net_manager: &NetManager) {
        let task_result = self.task_result.clone();
        let net_manager = net_manager.clone();
        thread_pool.spawn(move || {
            let result = net_manager
                .connect(&SERVER_TCP_ADDR)
                .map(|_| ())
                .map_err(|e| {
                    log::error!("failed to connect to game server! (REASON:{e})");
                    Box::new(e) as Box<dyn Error + Send>
                });
            task_result.push(result);
        });
    }
}

impl GameScene for GameLoginConnectScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.try_connect_game_server(app.io_threads(), app.net_manager());
        Ok(())
    }

    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::Connect => {
                let packet = ConnectPacket::from_raw(packet);
                self.connect_packet = Some(packet);
            }
            _ => {
                panic!("invalid packet received! (TYPE:{:?})", &packet_type);
            }
        }

        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_result.pop() {
            result?;
        }

        // `ConnectPakcet` 데이터를 확인합니다.
        if let Some(connect_packet) = self.connect_packet.take() {
            // 사용자 구성 정보에 저장합니다.
            let mut config = UserConfig::get();
            config.info = connect_packet.user;
            config.token = connect_packet.token;
            drop(config);
            println!("!");

            // 다음 게임 장면으로 전환합니다.
            // TODO: MainLobbyEnterScene으로 전환
            use crate::scenes::testbed::TestbedTitleScene;
            let next_scene = Box::new(TestbedTitleScene::new());
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
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
        self.ui_callback(window, egui_ctx);
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
        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        commands.push(encoder.finish());
        queue.submit(commands);

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
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({})", stringify!(GameLoginTitleScene))),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.background.draw(&self.main_camera, &mut rpass);
        }

        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!(
                        "RenderPass(UI({}))",
                        stringify!(GameLoginTitleScene)
                    )),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        view: render_target_view,
                        resolve_target: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_buffer_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();

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

    fn on_finish_draw(
        &mut self,
        _window: &Window,
        egui_renderer: &mut UiRenderer,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}
