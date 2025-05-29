use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::RawPacket;
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::BASE_WIDTH,
};

/// 애플리케이션 표시 언어에 따른 `확인 버튼` 텍스트
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

/// 오류 메시지를 출력하는 게임 장면입니다.
pub struct FatalErrorSceneLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// 모달 대화상자의 타이틀 텍스트입니다.
    title: String,
    /// 모달 대화상자의 메시지 텍스트입니다.
    message: String,

    /// 버튼 눌림 여부입니다.
    is_button_pressed: bool,
}

impl FatalErrorSceneLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new<T, M>(locale: Locale, title: T, message: M) -> Self
    where
        T: Into<String>,
        M: Into<String>,
    {
        Self {
            locale,
            title: title.into(),
            message: message.into(),
            is_button_pressed: false,
        }
    }
}

impl GameScene for FatalErrorSceneLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_enter_foreground(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(
        &mut self,
        _packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        None
    }

    fn on_update(&mut self, _: f32, _: &Window, app: &dyn AppHandle) {
        if self.is_button_pressed {
            let scene_flow = GameSceneFlow::Clear;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 타이틀 텍스트
        let font_id = egui::FontId::new(36.0 * scale, head_font_family);
        let title_text = egui::RichText::new(&self.title)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 메시지 텍스트
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let message_text = egui::RichText::new(&self.message)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 확인 텍스트
        let text = OKAY_TEXTS[i];
        let font_id = egui::FontId::new(24.0 * scale, main_font_family.clone());
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 확인 버튼
        let btn_width = 180.0 * scale;
        let btn_height = btn_width * 0.25;
        let okay_button = egui::Button::new(okay_text)
            .fill(egui::Color32::WHITE)
            .min_size((btn_width, btn_height).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let wnd_width = 640.0 * scale;
        let wnd_height = 480.0 * scale;
        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Window::new(title_text)
            .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
            .frame(frame)
            .movable(false)
            .collapsible(false)
            .order(egui::Order::Foreground)
            .max_size((wnd_width, wnd_height))
            .resizable([false, false])
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(16.0 * scale);
                    ui.label(message_text);
                    ui.add_space(16.0 * scale);
                    ui.add_enabled_ui(!self.is_button_pressed, |ui| {
                        if ui.add(okay_button).clicked() {
                            self.is_button_pressed = true;
                        }
                    });
                    ui.add_space(16.0 * scale);
                });
            });
    }
}
