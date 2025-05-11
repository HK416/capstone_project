use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::RawPacket;
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

/// 애플리케이션 표시 언어에 따른 `확인 버튼` 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

/// 메시지를 출력하는 게임 장면입니다.
pub struct MessageSceneLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// 모달 대화상자의 타이틀 문자열입니다.
    title: String,
    /// 모달 대화상자의 내용 문자열입니다.
    message: String,

    /// 버튼 눌림 여부입니다.
    is_button_pressed: bool,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,
}

impl MessageSceneLayer {
    /// 새로운 `MessageSceneLayer`을 생성합니다.
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
            delay_time_sec: 1.0,
        }
    }
}

impl GameScene for MessageSceneLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
        app.enable_cursor();
    }

    fn on_received_packet(&mut self, packet: RawPacket, _app: &dyn AppHandle) -> Option<RawPacket> {
        Some(packet)
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
        let title = ERR_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결이 끊어졌습니다!"];
                ERR_MSG_TEXTS[i]
            }
            NetworkError::IO(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] =
                    ["패킷을 읽는 도중 오류가 발생했습니다!"];
                ERR_MSG_TEXTS[i]
            }
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if !repeat && self.delay_time_sec <= 0.0 {
            if code == KeyCode::Enter {
                self.is_button_pressed = true;
                return true;
            }
        }
        return false;
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _: &Window, app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
        if self.is_button_pressed {
            let scene_flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 타이틀 텍스트
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title = egui::RichText::new(&self.title)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 메시지 텍스트
        let fmaily = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, fmaily);
        let main_text = egui::RichText::new(&self.message)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `확인 버튼` 텍스트
        let text = OKAY_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 확인 버튼
        let btn_width = 180.0 * scale;
        let btn_height = btn_width * 0.25;
        let okay_button = egui::Button::new(okay_text)
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .min_size((btn_width, btn_height).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Message"))
            .frame(frame)
            .show(app.egui_ctx(), |ui| {
                ui.set_max_width(640.0 * scale);
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.label(title);
                    ui.separator();

                    ui.add_space(8.0 * scale);
                    ui.label(main_text);
                    ui.add_space(16.0 * scale);

                    if ui.add(okay_button).clicked() {
                        // 이전 게임 장면으로 돌아갑니다.
                        let scene_flow = GameSceneFlow::Pop;
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    ui.add_space(16.0 * scale);
                });
            });
    }
}
