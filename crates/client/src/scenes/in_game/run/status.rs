use ahash::HashMap;
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{CapturePoint, GameInput},
    protocol::{Packet, PacketType, PullStagePacket, RawPacket},
};
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

/// 인게임 장면에서 현재 게임 진행 상태를 출력하는 게임 장면입니다.
pub struct InGameStatusLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 게임 진행 상황입니다.
    capture_point: CapturePoint,
    /// 남은 게임 시간입니다.
    remaining_time_sec: f32,

    /// 게임 인터페이스 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,
}

impl InGameStatusLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new(
        locale: Locale,
        capture_point: CapturePoint,
        remaining_time_sec: f32,
        ui_textures: HashMap<String, egui::load::SizedTexture>,
    ) -> Self {
        Self {
            locale,
            capture_point,
            remaining_time_sec,
            ui_textures,
        }
    }
}

impl GameScene for InGameStatusLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn should_update_subscene(&self) -> bool {
        true
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
        let event = AppEvent::SetGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        match packet.packet_type() {
            PacketType::PullStage => {
                let packet = PullStagePacket::from_raw(packet.clone());
                self.capture_point = packet.capture_point;
                self.remaining_time_sec = packet.remaining_time_sec;
            }
            _ => {}
        };
        Some(packet)
    }

    fn on_keyboard_pressed(
        &mut self,
        _code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        _repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !repeat {
            let config = UserConfig::get();
            if let Some(input) = config.get_keyboard_input(&(code, location)) {
                if input == GameInput::Status {
                    let scene_flow = GameSceneFlow::Pop;
                    let event = AppEvent::SetGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                    return true;
                }
            }
        }

        false
    }

    fn on_cursor_moved(
        &mut self,
        _x: f32,
        _y: f32,
        _dx: f32,
        _dy: f32,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn on_mouse_btn_pressed(
        &mut self,
        _x: f32,
        _y: f32,
        _button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn on_mouse_btn_released(
        &mut self,
        _x: f32,
        _y: f32,
        _button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 대화 상자 속성
        let wnd_width = 960.0 * scale;
        let wnd_height = 480.0 * scale;
        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::from_black_alpha(194))
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        egui::Modal::new(egui::Id::new("Modal_Window_Layout"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(64))
            .show(app.egui_ctx(), |ui| {
                ui.set_width(wnd_width);
                ui.set_height(wnd_height);
            });
    }
}
