use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{TexturePool, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::BASE_WIDTH,
};

use super::GameIntroLogoScene;

/// 장면 지속 시간(초)
const SCENE_DURATION: f32 = 5.6;
/// 장면 전환 지속 시간(초)
const FADE_IN_DURATION: f32 = 0.8;
/// 안내사항 텍스트가 사라지는 시간(초)
const FADE_OUT_DURATION: f32 = 0.8;

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["안내 사항"];
/// 애플리케이션 표시 언어에 따른 Main 텍스트
const MAIN_TEXTS: [&'static str; NUM_LOCALE] = ["이 게임은 Blue Archive의 2차 창작 게임이며"];
/// 애플리케이션 표시 언어에 따른 Sub 텍스트
const SUB_TEXTS: [&'static str; NUM_LOCALE] =
    ["2025년 한국공학대학교 게임공학과 졸업 작품 목적으로 제작되었습니다."];

/// 게임 인트로 화면을 보여주는 장면입니다.  
/// 검은색 화면에서 하얀색 화면으로 전환되며(Fade in) 화면에 안내사항이 표시됩니다.
pub struct GameIntroNotifyScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl GameIntroNotifyScene {
    /// 새로운 `GameIntroNotifyScene`을 생성합니다.
    pub fn new(locale: Locale, texture_pool: TexturePool) -> Self {
        Self {
            locale,
            elapsed_time_sec: 0.0,
            texture_pool,
        }
    }

    /// 배경 색상을 가져옵니다.
    fn get_background_color(&self) -> wgpu::Color {
        let s = self.elapsed_time_sec.min(FADE_IN_DURATION) / FADE_IN_DURATION;
        let c = (s * s * (3.0 - 2.0 * s)) as f64; // Smooth Step
        wgpu::Color {
            r: c,
            g: c,
            b: c,
            a: 1.0,
        }
    }

    /// 폰트 색상을 가져옵니다.
    fn get_font_color(&self) -> egui::Color32 {
        let s = (self.elapsed_time_sec - (SCENE_DURATION - FADE_OUT_DURATION)).max(0.0)
            / FADE_OUT_DURATION;
        let c = 1.0 - (s * s * (3.0 - 2.0 * s)) as f64; // Smooth Step
        egui::Color32::from_black_alpha((255.0 * c) as u8)
    }
}

impl GameScene for GameIntroNotifyScene {
    fn on_enter(&mut self, window: &Window, _app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 게임 장면 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 다음 게임 장면으로 전환합니다.
        if self.elapsed_time_sec >= SCENE_DURATION {
            let next_scene = GameIntroLogoScene::new(self.locale, self.texture_pool.clone());
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
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
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.get_background_color()),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let locale = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 텍스트
        let font_color = self.get_font_color();
        let text = TITLE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(64.0 * scale, family);
        let title_text = egui::RichText::new(text).font(font_id).color(font_color);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        let text = MAIN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(48.0 * scale, family);
        let notify_main_text = egui::RichText::new(text).font(font_id).color(font_color);
        let notify_main_label = egui::Label::new(notify_main_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        let text = SUB_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let notify_sub_text = egui::RichText::new(text).font(font_id).color(font_color);
        let notify_sub_label = egui::Label::new(notify_sub_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);

                ui.vertical_centered(|ui| {
                    ui.set_min_width(BASE_WIDTH * scale);
                    ui.set_max_width(BASE_WIDTH * scale);
                    ui.add(title_label);
                });
                ui.add_space(48.0 * scale);
                ui.vertical_centered(|ui| {
                    ui.set_min_width(BASE_WIDTH * scale);
                    ui.set_max_width(BASE_WIDTH * scale);
                    ui.add(notify_main_label);
                });
                ui.add_space(12.0 * scale);
                ui.vertical_centered(|ui| {
                    ui.set_min_width(BASE_WIDTH * scale);
                    ui.set_max_width(BASE_WIDTH * scale);
                    ui.add(notify_sub_label);
                });
            });
    }
}
