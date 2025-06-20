use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, UserId, UserName},
    protocol::{Packet, PacketType, RawPacket, RoomPlayerBanRequestPacket},
};
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, NEG_COLOR, NEG_FOCUS_COLOR, NORM_COLOR,
        NORM_EXP_COLOR, NORM_FOCUS_COLOR,
    },
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["플레이어 강제 퇴장"];
/// 애플리케이션 표시 언어에 따른 `확인` 버튼 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];
/// 애플리케이션 표시 언어에 따른 `취소` 버튼 텍스트입니다.
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["취소"];

/// 커스텀 게임 대기실 화면입니다.
/// 플레이어를 강제 퇴장하기 전에 한 번 더 묻습니다.
pub struct RoomPlayerBanOnemoreLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 대상 사용자 식별자
    target: UserId,
    /// 대상 사용자 이름
    target_name: UserName,

    /// 확인 버튼 상태
    okay_btn_state: ButtonState,
    /// 취소 버튼 상태
    cancel_btn_state: ButtonState,
    /// 입력 지연 시간
    delay_time_sec: f32,
}

impl RoomPlayerBanOnemoreLayer {
    /// 새로운 `RoomPlayerBanOnemoreLayer`를 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        target: UserId,
        target_name: UserName,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            target,
            target_name,
            okay_btn_state: ButtonState::Idle,
            cancel_btn_state: ButtonState::Idle,
            delay_time_sec: 0.3,
        }
    }
}

impl GameScene for RoomPlayerBanOnemoreLayer {
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

    fn on_received_packet(&mut self, packet: RawPacket, _app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::RoomDataUpdate => Some(packet),
            _ => None,
        }
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
                    // 게임 장면에서 빠져나옵니다.
                    let scene_flow = GameSceneFlow::Pop;
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
                KeyCode::Enter => {
                    // 패킷을 전송합니다.
                    let packet = RoomPlayerBanRequestPacket::new(self.uid, self.token, self.target);
                    let net = app.net_manager();
                    let socket = net.get(&SERVER_TCP_ADDR).unwrap();
                    socket.push_packet(packet.as_raw());

                    // 게임 장면에서 빠져나옵니다.
                    let scene_flow = GameSceneFlow::Pop;
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

        // 타이틀 텍스트
        let text = TITLE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 메시지 텍스트
        let text = match self.locale {
            Locale::KOR => format!("\"{}\"님을 게임에서 퇴장시키겠습니까?", &self.target_name),
        };
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let message_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let message_label = egui::Label::new(message_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // `확인` 버튼 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // `취소` 버튼 텍스트
        let text = CANCEL_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // `확인` 버튼
        let (bg_color, line_color) = match self.okay_btn_state {
            ButtonState::Idle => (NEG_COLOR, egui::Color32::TRANSPARENT),
            ButtonState::Hovered => (NEG_COLOR, NEG_FOCUS_COLOR),
            ButtonState::Pressed | ButtonState::Clicked => (NEG_FOCUS_COLOR, NEG_FOCUS_COLOR),
        };
        let okay_button = egui::Button::new(okay_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((180.0 * scale, 45.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        // `취소` 버튼
        let (bg_color, line_color) = match self.cancel_btn_state {
            ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
            ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
            ButtonState::Pressed | ButtonState::Clicked => (NORM_EXP_COLOR, egui::Color32::BLACK),
        };
        let cancel_button = egui::Button::new(cancel_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((180.0 * scale, 45.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(20.0 * scale)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        let mut modal = egui::Modal::new(egui::Id::new("Exit_Onemore"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96));
        modal.area = modal.area.order(egui::Order::Tooltip);
        modal.show(app.egui_ctx(), |ui| {
            ui.shrink_clip_rect(clip_rect);
            ui.set_min_width(640.0 * scale);
            ui.set_max_width(640.0 * scale);

            ui.vertical_centered(|ui| {
                ui.add_space(8.0 * scale);
                ui.add(title_label);
                ui.separator();

                ui.add_space(8.0 * scale);
                ui.add(message_label);
                ui.add_space(16.0 * scale);

                let enable = self.okay_btn_state != ButtonState::Clicked
                    && self.cancel_btn_state != ButtonState::Clicked;
                ui.add_enabled_ui(enable, |ui| {
                    egui::Grid::new(egui::Id::new("Button_Grid"))
                        .min_col_width(640.0 * 0.5 * scale)
                        .max_col_width(640.0 * 0.5 * scale)
                        .show(ui, |ui| {
                            ui.set_max_height(45.0 * scale);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // 예 버튼
                                    let response = ui.add(okay_button);
                                    if response.clicked() && self.delay_time_sec <= 0.0 {
                                        self.okay_btn_state = ButtonState::Clicked;

                                        // 패킷을 전송합니다.
                                        let packet = RoomPlayerBanRequestPacket::new(
                                            self.uid,
                                            self.token,
                                            self.target,
                                        );
                                        let net = app.net_manager();
                                        let socket = net.get(&SERVER_TCP_ADDR).unwrap();
                                        socket.push_packet(packet.as_raw());

                                        // 이전 게임 장면으로 돌아갑니다.
                                        let scene_flow = GameSceneFlow::Pop;
                                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                                        let event_loop_proxy = app.event_loop_proxy();
                                        event_loop_proxy.send_event(event).unwrap();
                                    } else if response.is_pointer_button_down_on() {
                                        self.okay_btn_state = ButtonState::Pressed;
                                    } else if response.hovered() | response.has_focus() {
                                        self.okay_btn_state = ButtonState::Hovered;
                                    } else {
                                        self.okay_btn_state = ButtonState::Idle;
                                    }
                                },
                            );

                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    // 취소 버튼
                                    let response = ui.add(cancel_button);
                                    if response.clicked() && self.delay_time_sec <= 0.0 {
                                        self.cancel_btn_state = ButtonState::Clicked;

                                        // 이전 게임 장면으로 돌아갑니다.
                                        let scene_flow = GameSceneFlow::Pop;
                                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                                        let event_loop_proxy = app.event_loop_proxy();
                                        event_loop_proxy.send_event(event).unwrap();
                                    } else if response.is_pointer_button_down_on() {
                                        self.cancel_btn_state = ButtonState::Pressed;
                                    } else if response.hovered() | response.has_focus() {
                                        self.cancel_btn_state = ButtonState::Hovered;
                                    } else {
                                        self.cancel_btn_state = ButtonState::Idle;
                                    }
                                },
                            );
                        });
                });
            });
            ui.add_space(18.0 * scale);
        });
    }
}
