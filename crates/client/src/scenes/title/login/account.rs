use std::time::Instant;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::UserId,
    protocol::{
        LoginFailedPacket, LoginRequestPacket, LoginSuccessPacket, Packet, PacketType, RawPacket,
    },
};
use rodio::Sink;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        SoundDataPool, TexturePool, NOTOSANS_BOLD, NOTOSANS_REGULAR, UI_BUTTON_BACK,
        UI_BUTTON_TOUCH, UI_NOTICE,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, GameLoginModalScene, LoginFailedModalScene, MainLobbyEnterScene,
        BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS, FONT_COLOR,
        NEG_COLOR, NEG_FOCUS_COLOR, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR,
    },
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["계정 입력"];
/// 애플리케이션 표시 언어에 따른 로그인 버튼 텍스트
const LOGIN_TEXTS: [&'static str; NUM_LOCALE] = ["로그인"];
/// 애플리케이션 표시 언어에 따른 취소 버튼 텍스트
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["취소"];

/// 로그인 타이틀 장면입니다.
/// 계정 정보를 입력하는 모달 대화상자를 출력합니다.
pub struct GameAccountModalScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 로그인 요청 여부
    requested: bool,
    /// 입력된 번호 데이터입니다.
    input_number: String,

    /// 입력 지연 시간입니다.
    delay_time_sec: f32,

    /// 로그인 버튼 상태
    login_button_state: ButtonState,
    /// 취소 버튼 상태
    cancel_button_state: ButtonState,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl GameAccountModalScene {
    pub fn new(
        locale: Locale,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        texture_pool: TexturePool,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            background_volume,
            effect_volume,
            voice_volume,
            requested: false,
            delay_time_sec: 0.3,
            input_number: String::with_capacity(16),
            login_button_state: ButtonState::Idle,
            cancel_button_state: ButtonState::Idle,
            texture_pool,
            sound_data_pool,
        }
    }
}

impl GameScene for GameAccountModalScene {
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
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            title,
            message,
            self.sound_data_pool.clone(),
        );
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 효과음을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(UI_NOTICE)
            .expect("UI_Notice sound must be preloaded!");
        let source = decoded.as_source();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.effect_volume as f32 / 255.0);
        sink.append(source);
        sink.play();
        sink.detach();
    }

    fn on_received_packet(
        &mut self,
        _: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::LoginFailed => {
                let packet = LoginFailedPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let next_scene = Box::new(LoginFailedModalScene::new(
                    self.locale,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    packet.reason,
                    self.texture_pool.clone(),
                    self.sound_data_pool.clone(),
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();

                // 효과음을 재생합니다.
                let decoded = self
                    .sound_data_pool
                    .get(UI_NOTICE)
                    .expect("UI_Notice sound must be preloaded!");
                let source = decoded.as_source();
                let sink = Sink::connect_new(app.audio_mixer());
                sink.set_volume(self.effect_volume as f32 / 255.0);
                sink.append(source);
                sink.play();
                sink.detach();
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
                    packet.token,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    &self.texture_pool,
                    &self.sound_data_pool,
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
                    let next_scene = Box::new(GameLoginModalScene::new(
                        self.locale,
                        self.background_volume,
                        self.effect_volume,
                        self.voice_volume,
                        self.texture_pool.clone(),
                        self.sound_data_pool.clone(),
                    ));
                    let scene_flow = GameSceneFlow::Change(next_scene);
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();

                    // 효과음을 재생합니다.
                    let decoded = self
                        .sound_data_pool
                        .get(UI_BUTTON_BACK)
                        .expect("UI_Button_Back sound must be preloaded!");
                    let source = decoded.as_source();
                    let sink = Sink::connect_new(app.audio_mixer());
                    sink.set_volume(self.effect_volume as f32 / 255.0);
                    sink.append(source);
                    sink.play();
                    sink.detach();
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
        let title_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 텍스트 입력기
        let mut input_changed = false;
        let input_number = self.input_number.parse::<u32>();
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let editor = egui::TextEdit::singleline(&mut self.input_number)
            .font(font_id)
            .char_limit(8)
            .min_size(egui::vec2(272.0 * scale, 52.0 * scale))
            .text_color(FONT_COLOR)
            .background_color(NORM_FOCUS_COLOR);

        // 로그인 버튼 텍스트
        let text = LOGIN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let login_btn_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // 로그인 버튼
        let (bg_color, line_color) = match self.login_button_state {
            ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
            ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
            ButtonState::Pressed | ButtonState::Clicked => (NORM_EXP_COLOR, egui::Color32::BLACK),
        };
        let login_button = egui::Button::new(login_btn_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((160.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        // 취소 버튼 텍스트
        let text = CANCEL_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_btn_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // 취소 버튼
        let (bg_color, line_color) = match self.cancel_button_state {
            ButtonState::Idle => (NEG_COLOR, egui::Color32::TRANSPARENT),
            ButtonState::Hovered => (NEG_COLOR, NEG_FOCUS_COLOR),
            ButtonState::Pressed | ButtonState::Clicked => (NEG_FOCUS_COLOR, NEG_FOCUS_COLOR),
        };
        let cancel_button = egui::Button::new(cancel_btn_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((160.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(20.0 * scale)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Login_Account_Modal"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96))
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(640.0 * scale);
                ui.set_max_width(640.0 * scale);

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.add(title_label);
                    ui.separator();

                    ui.add_enabled_ui(!self.requested, |ui| {
                        const EDITOR_SIZE: egui::Vec2 = egui::vec2(320.0, 52.0);
                        let response = ui.add_sized(EDITOR_SIZE * scale, editor);
                        if response.changed() {
                            input_changed = true;
                        }
                    });

                    ui.add_space(8.0 * scale);
                    ui.add_enabled_ui(!self.requested, |ui| {
                        egui::Grid::new(egui::Id::new("Login_Account_Buttons"))
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.set_min_width(320.0 * scale);
                                        ui.set_max_width(320.0 * scale);

                                        // 로그인 버튼
                                        let enabled = self.delay_time_sec <= 0.0
                                            && !self.requested
                                            && input_number.as_ref().is_ok_and(|val| *val != 0);
                                        let response = ui.add_enabled(enabled, login_button);
                                        if response.clicked() {
                                            // 로그인 요청 패킷을 생성합니다.
                                            if let Ok(num) = input_number {
                                                let packet =
                                                    LoginRequestPacket::new(UserId::new(num));

                                                // 패킷을 게임 서버에 전송합니다.
                                                let net_manager = app.net_manager();
                                                let socket =
                                                    net_manager.get(&SERVER_TCP_ADDR).unwrap();

                                                socket.push_packet(packet.as_raw());
                                                // 효과음을 재생합니다.
                                                let decoded = self
                                                    .sound_data_pool
                                                    .get(UI_BUTTON_TOUCH)
                                                    .expect(
                                                        "UI_Button_Touch sound must be preloaded!",
                                                    );
                                                let source = decoded.as_source();
                                                let sink = Sink::connect_new(app.audio_mixer());
                                                sink.set_volume(self.effect_volume as f32 / 255.0);
                                                sink.append(source);
                                                sink.play();
                                                sink.detach();

                                                self.requested = true;
                                                self.delay_time_sec = 0.3;
                                                self.login_button_state = ButtonState::Clicked;
                                            }
                                        } else if response.is_pointer_button_down_on() {
                                            self.login_button_state = ButtonState::Pressed;
                                        } else if response.hovered() | response.has_focus() {
                                            self.login_button_state = ButtonState::Hovered;
                                        } else {
                                            self.login_button_state = ButtonState::Idle;
                                        }
                                    },
                                );
                                ui.add_space(8.0 * scale);

                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_min_width(320.0 * scale);
                                        ui.set_max_width(320.0 * scale);

                                        // 취소 버튼
                                        let enabled = self.delay_time_sec <= 0.0 && !self.requested;
                                        let response = ui.add_enabled(enabled, cancel_button);
                                        if response.clicked() {
                                            // 게임 장면을 전환합니다.
                                            let next_scene = Box::new(GameLoginModalScene::new(
                                                self.locale,
                                                self.background_volume,
                                                self.effect_volume,
                                                self.voice_volume,
                                                self.texture_pool.clone(),
                                                self.sound_data_pool.clone(),
                                            ));
                                            let scene_flow = GameSceneFlow::Change(next_scene);
                                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                                            let event_loop_proxy = app.event_loop_proxy();
                                            event_loop_proxy.send_event(event).unwrap();

                                            // 효과음을 재생합니다.
                                            let decoded = self
                                                .sound_data_pool
                                                .get(UI_BUTTON_BACK)
                                                .expect("UI_Button_Back sound must be preloaded!");
                                            let source = decoded.as_source();
                                            let sink = Sink::connect_new(app.audio_mixer());
                                            sink.set_volume(self.effect_volume as f32 / 255.0);
                                            sink.append(source);
                                            sink.play();
                                            sink.detach();

                                            self.requested = true;
                                            self.delay_time_sec = 0.3;
                                            self.cancel_button_state = ButtonState::Clicked;
                                        } else if response.is_pointer_button_down_on() {
                                            self.cancel_button_state = ButtonState::Pressed;
                                        } else if response.hovered() | response.has_focus() {
                                            self.cancel_button_state = ButtonState::Hovered;
                                        } else {
                                            self.cancel_button_state = ButtonState::Idle;
                                        }
                                    },
                                );
                            });
                    });
                    ui.add_space(18.0 * scale);
                });
            });

        if input_changed {
            self.input_number.retain(|c| c.is_ascii_digit());
        }
    }
}
