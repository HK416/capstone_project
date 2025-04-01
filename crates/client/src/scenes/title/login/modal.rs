use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{Email, Passwd},
    protocol::{LoginRequestPacket, LoginSuccessPacket, Packet, PacketType, RawPacket},
};
use winit::window::Window;

use crate::{
    asset::NOTOSANS_REGULAR,
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, MainLobbyEnterScene, BASE_WIDTH},
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 로그인 텍스트
const LOGIN_TEXTS: [&'static str; NUM_LOCALE] = ["로그인"];

pub struct GameLoginModalScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 계정 이메일
    email: Email,
    /// 계정 비밀번호
    passwd: Passwd,

    /// 로그인 요청 여부
    requested: bool,
}

impl GameLoginModalScene {
    /// 새로운 `GameLoginModalScene`을 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            email: Email::default(),
            passwd: Passwd::default(),
            requested: false,
        }
    }
}

impl GameScene for GameLoginModalScene {
    fn transparents(&self) -> bool {
        true
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
        let title = ERR_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결이 끊겼습니다!"];
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::LoginFailed => {
                self.requested = false;
                // TODO 로그인 실패 처리
            }
            PacketType::LoginSuccess => {
                // 사용자 정보와 로그인 토큰을 저장합니다.
                let packet = LoginSuccessPacket::from_raw(packet);
                let mut config = UserConfig::get();
                config.info = packet.account;
                config.token = packet.token;
                drop(config);

                // 다음 게임 장면으로 전환합니다.
                let next_scene = Box::new(MainLobbyEnterScene::new(
                    self.locale,
                    packet.account,
                    packet.token,
                ));
                let scene_flow = GameSceneFlow::Reset(next_scene);
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            _ => {
                log::warn!(
                    "packet ignored >> invalid packet received! (SCENE:{:?}, TYPE:{:?})",
                    &self,
                    packet_type
                );
            }
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
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
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.add_enabled_ui(!self.requested, |ui| {
                                ui.set_width(128.0 * scale);
                                ui.set_height(96.0 * scale);
                                if ui.add(login_button).clicked() {
                                    self.requested = true;

                                    // 로그인 요청 패킷을 생성합니다.
                                    let packet = LoginRequestPacket::new(self.email, self.passwd);

                                    // 패킷을 게임 서버에 전송합니다.
                                    let net_manager = app.net_manager();
                                    let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                                    socket.push_packet(packet.as_raw());
                                }
                            });
                        },
                    );
                });
            });
    }
}
