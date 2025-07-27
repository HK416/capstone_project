use std::time::Instant;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, UserId, WorldId},
    protocol::{
        JoinFailedReason, JoinRoomFailedPacket, JoinRoomRequestPacket, Packet, PacketType,
        RawPacket, RoomDataUpdatePacket,
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
        SoundDataPool, TexturePool, TextureViewPool, NOTOSANS_BOLD, NOTOSANS_REGULAR,
        UI_BUTTON_BACK, UI_BUTTON_TOUCH, UI_NOTICE,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        lobby::{
            ERR_BANNED_TEXTS, ERR_FULL_CAPACITY_TEXTS, ERR_IN_PROGRASS_TEXTS, ERR_LIMITS_TEXTS,
            ERR_NOT_FOUND_TEXTS, MSG_MODAL_TEXTS,
        },
        CustomGameRoomScene, FatalErrorSceneLayer, MessageSceneLayer, BASE_WIDTH,
        ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, NORM_COLOR,
        NORM_EXP_COLOR, NORM_FOCUS_COLOR, POSI_COLOR, POSI_FOCUS_COLOR,
    },
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 참여"];
/// 애플리케이션 표시 언어에 따른 `확인 버튼` 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];
/// 애플리케이션 표시 언어에 따른 `취소 버튼` 텍스트입니다.
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["취소"];
/// 애플리케이션 표시 언어에 따른 `방 번호 입력` 텍스트 입니다.
const INFORMATION_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 방 번호를 입력해 주세요"];

/// 게임의 메인 로비 화면입니다.
/// 커스텀 게임에 참여하기 위한 모달 대화상자를 화면에 표시합니다.
pub struct MainLobbyJoinModalScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 클라이언트의 사용자 식별자입니다.
    uid: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 입력된 번호 데이터입니다.
    input_number: String,
    /// 확인 버튼 상태
    okay_btn_state: ButtonState,
    /// 취소 버튼 상태
    cancel_btn_state: ButtonState,

    /// 응답 요청 기다리는 여부
    wait_for_response: bool,
    /// 지연 시간
    delay_time_sec: f32,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl MainLobbyJoinModalScene {
    /// 새로운 `MainLobbyJoinModalScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            background_volume,
            effect_volume,
            voice_volume,
            input_number: String::with_capacity(9),
            okay_btn_state: ButtonState::Idle,
            cancel_btn_state: ButtonState::Idle,
            wait_for_response: false,
            delay_time_sec: 0.3,
            texture_pool,
            texture_view_pool,
            sound_data_pool,
        }
    }
}

impl GameScene for MainLobbyJoinModalScene {
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
            PacketType::RoomDataUpdate => {
                let packet = RoomDataUpdatePacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let next_scene = CustomGameRoomScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    packet.id,
                    self.texture_pool.clone(),
                    self.texture_view_pool.clone(),
                    self.sound_data_pool.clone(),
                    packet.stage_kind(),
                    packet.allow_duplicates(),
                    packet.allow_unbalanced(),
                    packet.allow_using_ai(),
                    packet.players,
                );
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();

                // 현재 재생 중인 배경음을 중단합니다.
                while let Some(sink) = app.sink_list().pop() {
                    sink.stop();
                }

                // 효과음을 재생합니다.
                let decoded = self
                    .sound_data_pool
                    .get(UI_BUTTON_TOUCH)
                    .expect("UI_Button_Touch sound must be preloaded!");
                let source = decoded.as_source();
                let sink = Sink::connect_new(app.audio_mixer());
                sink.set_volume(self.effect_volume as f32 / 255.0);
                sink.append(source);
                sink.play();
                sink.detach();
            }
            PacketType::JoinRoomFailed => {
                // 패킷을 생성합니다
                let packet = JoinRoomFailedPacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let i = self.locale as usize;
                let next_scene = Box::new(MessageSceneLayer::new(
                    self.locale,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    MSG_MODAL_TEXTS[i],
                    match packet.reason {
                        JoinFailedReason::NotFound => ERR_NOT_FOUND_TEXTS[i],
                        JoinFailedReason::FullCapacity => ERR_FULL_CAPACITY_TEXTS[i],
                        JoinFailedReason::InProgress => ERR_IN_PROGRASS_TEXTS[i],
                        JoinFailedReason::CreationLimited => ERR_LIMITS_TEXTS[i],
                        JoinFailedReason::Banned => ERR_BANNED_TEXTS[i],
                    },
                    None,
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
            PacketType::LobbyDataUpdate => return Some(packet),
            _ => {
                log::warn!(
                    "packet ignored: invalid packet received! (TYPE:{:?})",
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
                KeyCode::Enter => {
                    if let Ok(val) = self.input_number.parse::<u32>() {
                        // 패킷을 전송합니다.
                        let val = if val == 0 { u32::MAX } else { val };
                        let world_id = WorldId::new(val);
                        let packet = JoinRoomRequestPacket::new(world_id, self.uid, self.token);

                        // 패킷을 전송합니다.
                        let net_manager = app.net_manager();
                        let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                        socket.push_packet(packet.as_raw());

                        self.wait_for_response = true;
                    }
                }
                KeyCode::Escape => {
                    // 게임 장면을 전환합니다.
                    let scene_flow = GameSceneFlow::Pop;
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
        };

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
        let font_id = egui::FontId::new(32.0 * scale, family);
        let title_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 안내 텍스트
        let text = INFORMATION_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let info_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let info_label = egui::Label::new(info_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // `확인 버튼` 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // `취소 버튼` 텍스트
        let text = CANCEL_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

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

        // 확인 버튼
        let (bg_color, line_color) = match self.okay_btn_state {
            ButtonState::Idle => (POSI_COLOR, egui::Color32::TRANSPARENT),
            ButtonState::Hovered => (POSI_COLOR, POSI_FOCUS_COLOR),
            ButtonState::Clicked | ButtonState::Pressed => (POSI_FOCUS_COLOR, POSI_FOCUS_COLOR),
        };
        let okay_button = egui::Button::new(okay_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((128.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        // 취소 버튼
        let (bg_color, line_color) = match self.cancel_btn_state {
            ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
            ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
            ButtonState::Clicked | ButtonState::Pressed => (NORM_EXP_COLOR, egui::Color32::BLACK),
        };
        let cancel_button = egui::Button::new(cancel_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .corner_radius(5.0 * scale)
            .min_size((128.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        let frame = egui::Frame::new()
            .corner_radius(20.0 * scale)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        let mut modal = egui::Modal::new(egui::Id::new("Join_Custom_Modal"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(64));
        modal.area = modal.area.order(egui::Order::Foreground);
        modal.show(app.egui_ctx(), |ui| {
            ui.shrink_clip_rect(clip_rect);
            ui.set_min_width(640.0 * scale);
            ui.set_max_width(640.0 * scale);

            ui.vertical_centered(|ui| {
                ui.add_space(8.0 * scale);
                ui.add(title_label);
                ui.separator();

                ui.add_space(8.0 * scale);
                ui.add(info_label);
                ui.add_space(8.0 * scale);

                ui.add_enabled_ui(!self.wait_for_response, |ui| {
                    const EDITOR_SIZE: egui::Vec2 = egui::vec2(272.0, 52.0);
                    let response = ui.add_sized(EDITOR_SIZE * scale, editor);
                    if response.changed() {
                        input_changed = true;
                    }
                });

                ui.add_space(16.0 * scale);
                egui::Grid::new(egui::Id::new("Button_Grid"))
                    .min_col_width(640.0 * 0.5 * scale)
                    .max_col_width(640.0 * 0.5 * scale)
                    .show(ui, |ui| {
                        ui.set_max_height(45.0 * scale);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // 예 버튼
                            let enable = !self.wait_for_response && input_number.is_ok();
                            let response = ui.add_enabled(enable, okay_button);
                            if response.clicked() && self.delay_time_sec <= 0.0 {
                                self.okay_btn_state = ButtonState::Clicked;

                                if let Ok(val) = input_number {
                                    // 패킷을 전송합니다.
                                    let val = if val == 0 { u32::MAX } else { val };
                                    let world_id = WorldId::new(val);
                                    let packet =
                                        JoinRoomRequestPacket::new(world_id, self.uid, self.token);

                                    // 패킷을 전송합니다.
                                    let net_manager = app.net_manager();
                                    let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                                    socket.push_packet(packet.as_raw());

                                    self.wait_for_response = true;
                                }
                            } else if response.is_pointer_button_down_on() {
                                self.okay_btn_state = ButtonState::Pressed;
                            } else if response.hovered() | response.has_focus() {
                                self.okay_btn_state = ButtonState::Hovered;
                            } else {
                                self.okay_btn_state = ButtonState::Idle;
                            }
                        });

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            // 취소 버튼
                            let enable = !self.wait_for_response;
                            let response = ui.add_enabled(enable, cancel_button);
                            if response.clicked() && self.delay_time_sec <= 0.0 {
                                // 이전 게임 장면으로 복귀합니다.
                                let scene_flow = GameSceneFlow::Pop;
                                let event = AppEvent::AddGameSceneFlow(scene_flow);
                                let event_loop_proxy = app.event_loop_proxy();
                                event_loop_proxy.send_event(event).unwrap();

                                self.cancel_btn_state = ButtonState::Clicked;
                                self.wait_for_response = true;

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
                            } else if response.is_pointer_button_down_on() {
                                self.cancel_btn_state = ButtonState::Pressed;
                            } else if response.hovered() | response.has_focus() {
                                self.cancel_btn_state = ButtonState::Hovered;
                            } else {
                                self.cancel_btn_state = ButtonState::Idle;
                            }
                        });
                    });
            });
            ui.add_space(18.0 * scale);
        });

        if input_changed {
            self.input_number.retain(|c| c.is_ascii_digit());
        }
    }
}
