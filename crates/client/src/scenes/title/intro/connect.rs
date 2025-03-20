use std::{error::Error, sync::Arc};

use mod_app::{app::AppHandle, etc::AppEvent, net::NetManager, scene::{GameScene, GameSceneFlow}};
use mod_parallelism::collections::Queue;
use mod_render::{ScreenDescriptor, UiRenderer};
use rayon::ThreadPool;
use winit::window::Window;

use crate::{asset::NOTOSANS_REGULAR, config::{Locale, NUM_LOCALE}, scenes::BASE_WIDTH, SERVER_TCP_ADDR};

use super::GameIntroVerifyScene;

/// 애플리케이션 표시 언어에 따른 게임 서버 연결 텍스트
const CONNECT_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결 중"];

pub struct GameIntroConnectScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 작업 결과를 저장
    task_result: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,

    // ----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>, 
    egui_free_texture_ids: Vec<egui::TextureId>, 

    /// 게임 로고 텍스처 식별자
    game_logo_texture_id: egui::load::SizedTexture,
}

impl GameIntroConnectScene {
    /// 새로운 `GameIntroConnectScene`을 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self { 
            locale, 
            task_result: Arc::new(Queue::new()), 
            egui_clip_primitives: Vec::default(), 
            egui_free_texture_ids: Vec::default(), 
            game_logo_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            } 
        }
    }

    /// UI 콜백 함수
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let main_font_id = egui::FontId::new(18.0 * scale, main_font_family);
        
        // 텍스트
        let i = self.locale as usize;
        let text = CONNECT_TEXTS[i];
        let connect_text = egui::RichText::new(text)
            .font(main_font_id)
            .color(egui::Color32::BLACK);

        // 게임 로고 속성
        let ratio = self.game_logo_texture_id.size.x / self.game_logo_texture_id.size.y;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let img_width = width * 0.3;
        let img_height = img_width / ratio;
        let rect = egui::Rect {
            min: egui::pos2(
                (center_x - 0.5 * img_width) / scale_factor,
                (center_y - 0.5 * img_height) / scale_factor,
            ),
            max: egui::pos2(
                (center_x + 0.5 * img_width) / scale_factor,
                (center_y + 0.5 * img_height) / scale_factor,
            ),
        };

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -18.0 * scale])
            .show(app.egui_ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(connect_text);
                })
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                egui::Image::new(self.game_logo_texture_id).paint_at(ui, rect);
            });

        Ok(())
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

impl GameScene for GameIntroConnectScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.try_connect_game_server(app.io_threads(), app.net_manager());
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
            // 오류를 확인합니다.
            result?;

            // 다음 게임 장면으로 전환합니다.
            let next_scene = Box::new(GameIntroVerifyScene::new(self.locale));
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
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!(
                        "RenderPass(UI({}))",
                        stringify!(GameIntroConnectScene)
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
