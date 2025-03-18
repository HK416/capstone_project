use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    asset::NOTOSANS_REGULAR,
    config::{Locale, UserConfig},
    scenes::BASE_WIDTH,
};

use super::InitWindowScene;

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성을 설정하는 장면입니다.  
/// 애플리케이션 표시 언어를 선택합니다.
pub struct InitLocaleScene {
    /// 선택된 언어
    locale: Locale,
    /// 언어가 선택된 여부
    selected: bool,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl InitLocaleScene {
    /// 새로운 `InitLocaleScene`을 생성합니다.
    pub fn new() -> Self {
        Self {
            locale: Locale::default(),
            selected: false,
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
        let font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(48.0 * scale, font_family);
        let font_color = egui::Color32::WHITE;

        // 버튼 텍스트 데이터
        // let eng_btn_text = egui::RichText::new("English")
        //     .font(font_id.clone())
        //     .color(font_color.clone());
        // let jpn_btn_text = egui::RichText::new("日本語")
        //     .font(font_id.clone())
        //     .color(font_color.clone());
        let kor_btn_text = egui::RichText::new("한국어")
            .font(font_id.clone())
            .color(font_color.clone());

        // 버튼 속성
        let btn_width = width / (6.0 * scale_factor);
        let btn_height = btn_width / 6.0;
        let btn_size = egui::Vec2::new(btn_width, btn_height);

        // 버튼
        // let eng_btn = egui::Button::new(eng_btn_text)
        //     .min_size(btn_size.clone());
        // let jpn_btn = egui::Button::new(jpn_btn_text)
        //     .min_size(btn_size.clone());
        let kor_btn = egui::Button::new(kor_btn_text).min_size(btn_size.clone());

        egui::Area::new(egui::Id::new("InitLocaleScene"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(egui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // if ui.add(eng_btn).clicked() && !self.selected {
                    //     self.locale = Locale::ENG;
                    //     self.button_pressed = true;
                    // }

                    // if ui.add(jpn_btn).clicked() && !self.selected {
                    //     self.locale = Locale::JPN;
                    //     self.button_pressed = true;
                    // }

                    if ui.add(kor_btn).clicked() && !self.selected {
                        self.locale = Locale::KOR;
                        self.selected = true;
                    }
                });
            });
    }
}

impl GameScene for InitLocaleScene {
    fn on_enter(
        &mut self,
        window: &Window,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
        window.set_cursor_visible(true);
        Ok(())
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let mut config = UserConfig::get();
        config.locale = self.locale;
        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        if self.selected {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = Box::new(InitWindowScene::new());
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
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("RenderPass(UI({}))", stringify!(InitLocaleScene))),
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
