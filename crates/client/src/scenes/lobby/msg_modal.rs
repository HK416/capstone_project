use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::BASE_WIDTH,
};

/// 애플리케이션 표시 언어에 따른 `확인 버튼` 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

pub struct MainLobbyMessageModalScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// 모달 대화상자의 타이틀 문자열입니다.
    title: String,
    /// 모달 대화상자의 내용 문자열입니다.
    message: String,
}

impl MainLobbyMessageModalScene {
    /// 새로운 `MainLobbyMessageModalScene`을 생성합니다.
    pub fn new<T, M>(locale: Locale, title: T, message: M) -> Self
    where
        T: Into<String>,
        M: Into<String>,
    {
        Self {
            locale,
            title: title.into(),
            message: message.into(),
        }
    }
}

impl GameScene for MainLobbyMessageModalScene {
    fn transparents(&self) -> bool {
        true
    }

    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // `Head` 텍스트
        let font_id = egui::FontId::new(32.0 * scale, head_font_family.clone());
        let head_text = egui::RichText::new(&self.title)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // `Main` 텍스트
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let main_text = egui::RichText::new(&self.message)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // `확인 버튼` 텍스트
        let text = OKAY_TEXTS[i];
        let font_id = egui::FontId::new(24.0 * scale, main_font_family.clone());
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 확인 버튼
        let okay_button = egui::Button::new(okay_text)
            .fill(egui::Color32::LIGHT_GRAY)
            .corner_radius(3.0)
            .min_size((128.0 * scale, 72.0 * scale).into());

        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Message"))
            .frame(frame)
            .show(app.egui_ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.label(head_text);
                    ui.separator();

                    ui.add_space(16.0 * scale);
                    ui.label(main_text);
                    ui.add_space(16.0 * scale);

                    if ui.add(okay_button).clicked() {
                        // 이전 게임 장면으로 돌아갑니다.
                        let scene_flow = GameSceneFlow::Pop;
                        let event = AppEvent::SetGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    ui.add_space(8.0 * scale);
                });
            });

        Ok(())
    }
}
