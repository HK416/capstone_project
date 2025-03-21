use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use winit::window::Window;

use crate::{asset::NOTOSANS_REGULAR, config::{Locale, NUM_LOCALE}, scenes::BASE_WIDTH};

/// 애플리케이션 표시 언어에 따른 로그인 텍스트
const LOGIN_TEXTS: [&'static str; NUM_LOCALE] = ["로그인"];

pub struct GameLoginModalScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 로그인 요청 여부
    requested: bool,
}

impl GameLoginModalScene {
    /// 새로운 `GameLoginModalScene`을 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale, 
            requested: false,
        }
    }
}

impl GameScene for GameLoginModalScene {
    fn transparents(&self) -> bool {
        true
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

        // 텍스트 속성
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 텍스트
        let i = self.locale as usize;
        let text = LOGIN_TEXTS[i];
        let login_btn_font = egui::FontId::new(24.0 * scale, main_font_family);
        let login_btn_text = egui::RichText::new(text)
            .font(login_btn_font)
            .color(egui::Color32::BLACK);

        // 버튼
        let login_button = egui::Button::new(login_btn_text)
            .fill(egui::Color32::LIGHT_GRAY)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK));


        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Login_Modal"))
            .frame(frame)
            .show(app.egui_ctx(), |ui| {
                ui.set_width(640.0 * scale);
                ui.set_height(480.0 * scale);

                ui.vertical_centered(|ui| {
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                        ui.add_enabled_ui(!self.requested, |ui| {
                            ui.set_width(128.0 * scale);
                            ui.set_height(96.0 * scale);
                            if ui.add(login_button).clicked() {
                                self.requested = true;
                            }
                        });
                    });
                });
            });

        Ok(())
    }
}


