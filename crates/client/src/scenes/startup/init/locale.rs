use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use rodio::Sink;
use winit::window::Window;

use crate::{
    asset::{SoundDataPool, TexturePool, NOTOSANS_REGULAR, UI_BUTTON_TOUCH},
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
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    // 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InitLocaleScene {
    /// 새로운 `InitLocaleScene`을 생성합니다.
    pub fn new(texture_pool: TexturePool, sound_data_pool: SoundDataPool) -> Self {
        Self {
            locale: Locale::default(),
            background_volume: 51,
            effect_volume: 255,
            voice_volume: 204,
            selected: false,
            texture_pool,
            sound_data_pool,
        }
    }
}

impl GameScene for InitLocaleScene {
    fn on_enter(&mut self, window: &Window, _app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
        window.set_cursor_visible(true);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let mut config = UserConfig::get();
        config.locale = self.locale;
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        if self.selected {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = InitWindowScene::new(
                self.locale,
                self.background_volume,
                self.effect_volume,
                self.voice_volume,
                self.texture_pool.clone(),
                self.sound_data_pool.clone(),
            );
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 폰트 속성
        let font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, font_family);
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
        let btn_width = 320.0 * scale;
        let btn_height = btn_width * 0.25;
        let btn_size = egui::Vec2::new(btn_width, btn_height);

        // 버튼
        // let eng_btn = egui::Button::new(eng_btn_text)
        //     .min_size(btn_size.clone());
        // let jpn_btn = egui::Button::new(jpn_btn_text)
        //     .min_size(btn_size.clone());
        let kor_btn = egui::Button::new(kor_btn_text).min_size(btn_size.clone());

        egui::Area::new(egui::Id::new("Locale_Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(!self.selected, |ui| {
                        // if ui.add(eng_btn).clicked() && !self.selected {
                        //     self.locale = Locale::ENG;
                        //     self.button_pressed = true;
                        // }

                        // if ui.add(jpn_btn).clicked() && !self.selected {
                        //     self.locale = Locale::JPN;
                        //     self.button_pressed = true;
                        // }

                        if ui.add(kor_btn).clicked() {
                            self.locale = Locale::KOR;
                            self.selected = true;

                            // 효과음을 재생합니다.
                            let decoded = self
                                .sound_data_pool
                                .get(UI_BUTTON_TOUCH)
                                .expect("UI_Button_Touch sound must be preloaded!");
                            let source = decoded.as_source();
                            let sink = Sink::connect_new(app.audio_mixer());
                            sink.set_volume(self.effect_volume as f32 / 255.0);
                            sink.append(source);
                            sink.play();
                            sink.detach();
                        }
                    });
                });
            });
    }
}
