use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    asset::NEXON_LV2_GOTHIC_BOLD, config::{Locale, UserConfig, NUM_LOCALE}, render::{BackgroundDataLayout, BackgroundResource}, scenes::BASE_WIDTH
};

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["아무 키나 눌러 게임을 시작"];

/// 게임 장면 경과 시간의 최대 지속 시간입니다.
const MAX_SCENE_DURATION: f32 = 24.0;
/// 폰트 알파 값의 주기입니다.
const FONT_APPEAR_CYCLE: f32 = 4.0;
/// 게임 로그인 타이틀 화면을 표시하는 장면입니다.
pub struct GameLoginTitleScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 배경 리소스입니다.
    background: BackgroundResource,
    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl GameLoginTitleScene {
    /// 새로운 `GameLoginTitleScene`을 생성합니다.
    pub fn new(background: BackgroundResource) -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale, 
            background,
            elapsed_time_sec: 0.0,
            egui_clip_primitives: Vec::default(),
            egui_free_texture_ids: Vec::default(),
        }
    }

    /// UI 콜백 함수
    fn ui_callback(&mut self, window: &Window, egui_ctx: &egui::Context) {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NEXON_LV2_GOTHIC_BOLD.into());
        let head_font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let head_font_color = self.get_font_color();

        // 텍스트
        let i = self.locale as usize;
        let text = HEAD_TEXTS[i];
        let head_text = egui::RichText::new(text)
            .font(head_font_id)
            .color(head_font_color);

        egui::Area::new(egui::Id::new("Layout_Enter"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -128.0 * scale])
            .show(egui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(head_text);
                })
            });
    }

    fn get_font_color(&self) -> egui::Color32 {
        use core::f32::consts::PI;
        let s = (self.elapsed_time_sec % FONT_APPEAR_CYCLE) / FONT_APPEAR_CYCLE;
        let c = (s * PI).sin();
        egui::Color32::from_black_alpha((255.0 * c) as u8)
    }
}

impl GameScene for GameLoginTitleScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, height): (f32, f32) = window.inner_size().into();
        self.background.uniform_buffer.update(
            app.render_device(),
            app.render_queue(),
            BackgroundDataLayout {
                aspect_ratio: width / height,
                ..Default::default()
            },
        );
        Ok(())
    }

    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.elapsed_time_sec 
            = (self.elapsed_time_sec + elapsed_time_sec) % MAX_SCENE_DURATION;
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

            self.background.draw(&mut rpass);
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
