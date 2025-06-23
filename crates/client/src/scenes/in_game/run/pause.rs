use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::{PacketType, RawPacket};
use mod_render::UiRenderer;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

/// 애플리케이션 표시 언어에 따른 대화 상자 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["일시 정지"];

/// 애플리케이션 표시 언어에 따른 `계속 게임하기 버튼` 텍스트입니다.
const RESUME_TEXTS: [&'static str; NUM_LOCALE] = ["계속 하기"];

/// 인게임에서 일시정지 대화상자를 출력하는 게임 장면입니다.
pub struct InGamePauseLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// UI의 활성화 여부입니다.
    ui_enabled: bool,
}

impl InGamePauseLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            ui_enabled: true,
        }
    }
}

impl GameScene for InGamePauseLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn should_update_subscene(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let event = AppEvent::CursorDisable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i]
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        if packet.packet_type() == PacketType::FinishStage {
            let scene_flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        Some(packet)
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !repeat && code == KeyCode::Escape {
            // 이전 게임 장면으로 되돌아갑니다.
            self.ui_enabled = false;
            let scene_flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        true
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
        let font_id = egui::FontId::new(36.0 * scale, head_font_family.clone());
        let title_text = egui::RichText::new(TITLE_TEXTS[i])
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 버튼 속성
        let btn_width = 300.0 * scale;
        let btn_height = 40.0 * scale;

        // 계속하기 버튼
        let font_id = egui::FontId::new(18.0 * scale, main_font_family.clone());
        let resume_btn_text = egui::RichText::new(RESUME_TEXTS[i])
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);
        let resume_btn = egui::Button::new(resume_btn_text)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK))
            .fill(egui::Color32::LIGHT_GRAY);

        // 대화 상자 속성
        let wnd_width = 320.0 * scale;
        let wnd_height = 480.0 * scale;
        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        egui::Modal::new(egui::Id::new("Modal_Window_Layout"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(64))
            .show(app.egui_ctx(), |ui| {
                ui.set_width(wnd_width);
                ui.set_height(wnd_height);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(8.0 * scale);
                    ui.label(title_text);
                    ui.add_space(24.0 * scale);

                    ui.add_enabled_ui(self.ui_enabled, |ui| {
                        if ui.add_sized((btn_width, btn_height), resume_btn).clicked() {
                            // 이전 게임 장면으로 되돌아갑니다.
                            self.ui_enabled = false;
                            let scene_flow = GameSceneFlow::Pop;
                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                            let event_loop_proxy = app.event_loop_proxy();
                            event_loop_proxy.send_event(event).unwrap();
                        }

                        ui.add_space(8.0 * scale);
                    });
                });
            });
    }
}
