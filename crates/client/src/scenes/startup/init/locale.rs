use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
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
}

impl InitLocaleScene {
    /// 새로운 `InitLocaleScene`을 생성합니다.
    pub fn new() -> Self {
        Self {
            locale: Locale::default(),
            selected: false,
        }
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

    fn on_draw(
        &self,
        _window: &Window,
        _encoder: &mut wgpu::CommandEncoder, 
        _render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
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
            .show(app.egui_ctx(), |ui| {
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

        Ok(())
    }
}
