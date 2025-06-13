use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::{
    LoginFailedPacket, LoginRequestPacket, LoginSuccessPacket, Packet, PacketType, RawPacket,
};
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{TexturePool, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, GameExitModalScene, LoginFailedModalScene, MainLobbyEnterScene,
        BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
    },
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["로그인 방법 선택"];
/// 애플리케이션 표시 언어에 따른 `로그인` 버튼 텍스트
const LOGIN_TEXTS: [&'static str; NUM_LOCALE] = ["테스트 로그인"];
/// 애플리케이션 표시 언어에 따른 `종료` 버튼 텍스트
const EXIT_TEXTS: [&'static str; NUM_LOCALE] = ["게임 종료"];

pub struct GameLoginModalScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 로그인 요청 여부
    requested: bool,

    /// 로그인 버튼 상태
    login_button_state: ButtonState,
    /// 종료 버튼 상태
    exit_button_state: ButtonState,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl GameLoginModalScene {
    /// 새로운 `GameLoginModalScene`을 생성합니다.
    pub fn new(locale: Locale, texture_pool: TexturePool) -> Self {
        Self {
            locale,
            requested: false,
            login_button_state: ButtonState::Idle,
            exit_button_state: ButtonState::Idle,
            delay_time_sec: 0.3,
            texture_pool,
        }
    }
}

impl GameScene for GameLoginModalScene {
    fn transparents(&self) -> bool {
        true
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::LoginFailed => {
                let packet = LoginFailedPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let next_scene = Box::new(LoginFailedModalScene::new(
                    self.locale,
                    packet.reason,
                    self.texture_pool.clone(),
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::LoginSuccess => {
                // 사용자 정보와 로그인 토큰을 저장합니다.
                let packet = LoginSuccessPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let next_scene = Box::new(MainLobbyEnterScene::new(
                    self.locale,
                    packet.uid,
                    packet.name,
                    packet.tier,
                    packet.profile_icon,
                    self.texture_pool.clone(),
                    packet.token,
                ));
                let scene_flow = GameSceneFlow::Reset(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
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

        None
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
        if !repeat && self.delay_time_sec <= 0.0 {
            match code {
                KeyCode::Escape => {
                    // 게임 장면을 전환합니다.
                    let next_scene = Box::new(GameExitModalScene::new(
                        self.locale,
                        self.texture_pool.clone(),
                    ));
                    let scene_flow = GameSceneFlow::Change(next_scene);
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
                _ => {}
            }
        }

        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
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

        // 로그인 타이틀 텍스트
        let text = TITLE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 로그인 버튼 텍스트
        let text = LOGIN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let login_btn_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 로그인 버튼
        let fill = match self.login_button_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let login_button = egui::Button::new(login_btn_text)
            .sense(egui::Sense::all())
            .fill(fill)
            .corner_radius(3.0)
            .min_size((512.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        // 종료 버튼 텍스트
        let text = EXIT_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let login_btn_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 종료 버튼
        let fill = match self.exit_button_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let exit_button = egui::Button::new(login_btn_text)
            .sense(egui::Sense::all())
            .fill(fill)
            .corner_radius(3.0)
            .min_size((512.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(5.0)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Login_Modal"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96))
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(640.0 * scale);
                ui.set_max_width(640.0 * scale);

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.label(title_text);
                    ui.separator();

                    ui.add_space(8.0 * scale);
                    ui.add_enabled_ui(!self.requested, |ui| {
                        // 로그인 버튼
                        let response = ui.add(login_button);
                        if response.clicked() && self.delay_time_sec <= 0.0 {
                            self.login_button_state = ButtonState::Clicked;
                            self.requested = true;

                            // 로그인 요청 패킷을 생성합니다.
                            let packet = LoginRequestPacket::new();

                            // 패킷을 게임 서버에 전송합니다.
                            let net_manager = app.net_manager();
                            let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                            socket.push_packet(packet.as_raw());
                        } else if response.is_pointer_button_down_on() {
                            self.login_button_state = ButtonState::Pressed;
                        } else if response.hovered() | response.has_focus() {
                            self.login_button_state = ButtonState::Hovered;
                        } else {
                            self.login_button_state = ButtonState::Idle;
                        }

                        ui.add_space(4.0 * scale);

                        // 종료 버튼
                        let response = ui.add(exit_button);
                        if response.clicked() && self.delay_time_sec <= 0.0 {
                            self.exit_button_state = ButtonState::Clicked;

                            // 게임 장면을 전환합니다.
                            let next_scene = Box::new(GameExitModalScene::new(
                                self.locale,
                                self.texture_pool.clone(),
                            ));
                            let scene_flow = GameSceneFlow::Change(next_scene);
                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                            let event_loop_proxy = app.event_loop_proxy();
                            event_loop_proxy.send_event(event).unwrap();
                        } else if response.is_pointer_button_down_on() {
                            self.exit_button_state = ButtonState::Pressed;
                        } else if response.hovered() | response.has_focus() {
                            self.exit_button_state = ButtonState::Hovered;
                        } else {
                            self.exit_button_state = ButtonState::Idle;
                        }
                    });
                    ui.add_space(18.0 * scale);
                });
            });
    }
}
