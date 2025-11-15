use std::time::Instant;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, UserId},
    protocol::{
        FormationDataInitPacket, MatchCancelPacket, MatchRequestPacket, MatchRequestRejectedPacket,
        MatchRequestRejectedReason, Packet, PacketType, RawPacket,
    },
};
use mod_render::UiRenderer;
use rodio::Sink;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    SERVER_TCP_ADDR,
    asset::{
        NOTOSANS_BOLD, NOTOSANS_REGULAR, SoundDataPool, TexturePool, TextureViewPool,
        UI_BUTTON_BACK, UI_NOTICE,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        BASE_WIDTH, CharacterFormationScene, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, FatalErrorSceneLayer, FormationPlayerData,
        MessageSceneLayer, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR,
    },
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["게임 대기"];
/// 애플리케이션 표시 언어에 따른 안내 텍스트입니다.
const INFO_TEXTS: [&'static str; NUM_LOCALE] = ["다른 상대를 찾고 있습니다..."];
/// 애플리케이션 표시 언어에 따른 취소 버튼 텍스트입니다.
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["취소"];

/// 애플리케이션 표시 언어에 따른 게임 매칭 실패 모달의 타이틀 텍스트입니다.
const REJECT_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["매칭 실패"];
/// 애플리케이션 표시 언어에 따른 관지라 차단 메시지입니다.
const REJECT_BANNED_TEXTS: [&'static str; NUM_LOCALE] = ["게임 관리자에 의해 차단되었습니다."];
/// 애플리케이션 표시 언어에 따른 서버 용량 제한 메시지입니다.
const REJECT_LIMITED_TEXTS: [&'static str; NUM_LOCALE] = ["서버가 혼잡합니다."];

/// 게임의 메인 로비 화면입니다.
/// 매칭을 위한 모달 대화상자를 화면에 표시합니다.
pub struct MainLobbyWaitForMatching {
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

    /// 지연 시간
    delay_time_sec: f32,

    /// 취소 버튼 상태
    cancel_btn_state: ButtonState,
    /// 취소 응답을 기다리는 여부
    wait_for_response: bool,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl MainLobbyWaitForMatching {
    /// 새로운 `MainLobbyWaitForMatching` 장면을 생성합니다.
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
            delay_time_sec: 0.3,
            cancel_btn_state: ButtonState::Idle,
            wait_for_response: false,
            texture_pool,
            texture_view_pool,
            sound_data_pool,
        }
    }
}

impl GameScene for MainLobbyWaitForMatching {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // 패킷을 생성 후 전송합니다.
        let packet = MatchRequestPacket::new(self.uid, self.token);
        let net_manager = app.net_manager();
        let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
        socket.push_packet(packet.as_raw());
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
        if let Some(mixer) = app.audio_mixer() {
            let decoded = self
                .sound_data_pool
                .get(UI_NOTICE)
                .expect("UI_Notice sound must be preloaded!");
            let source = decoded.as_source();
            let sink = Sink::connect_new(mixer);
            sink.set_volume(self.effect_volume as f32 / 255.0);
            sink.append(source);
            sink.play();
            sink.detach();
        }
    }

    fn on_received_packet(
        &mut self,
        _time_stamp: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::LobbyDataUpdate => return Some(packet),
            PacketType::MatchRequestRejected => {
                let packet = MatchRequestRejectedPacket::from_raw(packet);
                let reason = packet.reason;
                match reason {
                    MatchRequestRejectedReason::AlreadyInQueue => { /* empty */ }
                    MatchRequestRejectedReason::Banned => {
                        // 게임 장면을 변경합니다.
                        let i = self.locale as usize;
                        let title = REJECT_TITLE_TEXTS[i];
                        let message = REJECT_BANNED_TEXTS[i];
                        let scene = MessageSceneLayer::new(
                            self.locale,
                            self.background_volume,
                            self.effect_volume,
                            self.voice_volume,
                            title,
                            message,
                            None,
                            self.sound_data_pool.clone(),
                        );
                        let flow = GameSceneFlow::Change(Box::new(scene));
                        let event = AppEvent::AddGameSceneFlow(flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();

                        // 효과음을 재생합니다.
                        if let Some(mixer) = app.audio_mixer() {
                            let decoded = self
                                .sound_data_pool
                                .get(UI_NOTICE)
                                .expect("UI_Notice sound must be preloaded!");
                            let source = decoded.as_source();
                            let sink = Sink::connect_new(mixer);
                            sink.set_volume(self.effect_volume as f32 / 255.0);
                            sink.append(source);
                            sink.play();
                            sink.detach();
                        }
                    }
                    MatchRequestRejectedReason::CreationLimited => {
                        // 게임 장면을 변경합니다.
                        let i = self.locale as usize;
                        let title = REJECT_TITLE_TEXTS[i];
                        let message = REJECT_LIMITED_TEXTS[i];
                        let scene = MessageSceneLayer::new(
                            self.locale,
                            self.background_volume,
                            self.effect_volume,
                            self.voice_volume,
                            title,
                            message,
                            None,
                            self.sound_data_pool.clone(),
                        );
                        let flow = GameSceneFlow::Change(Box::new(scene));
                        let event = AppEvent::AddGameSceneFlow(flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();

                        // 효과음을 재생합니다.
                        if let Some(mixer) = app.audio_mixer() {
                            let decoded = self
                                .sound_data_pool
                                .get(UI_NOTICE)
                                .expect("UI_Notice sound must be preloaded!");
                            let source = decoded.as_source();
                            let sink = Sink::connect_new(mixer);
                            sink.set_volume(self.effect_volume as f32 / 255.0);
                            sink.append(source);
                            sink.play();
                            sink.detach();
                        }
                    }
                    MatchRequestRejectedReason::Canceled => {
                        // 이전 게임 장면으로 전환합니다.
                        let flow = GameSceneFlow::Pop;
                        let event = AppEvent::AddGameSceneFlow(flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                }
            }
            PacketType::FormationDataInit => {
                let mut packet = FormationDataInitPacket::from_raw(packet);
                // 플레이어 데이터를 생성합니다.
                let players = packet
                    .players
                    .drain(..)
                    .map(|data| {
                        (
                            data.uid,
                            FormationPlayerData::new(
                                data.uid,
                                data.name,
                                data.profile_icon,
                                data.tier(),
                                data.team(),
                                data.team_index(),
                            ),
                        )
                    })
                    .collect();

                let scene = CharacterFormationScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    packet.remaining_time_ms,
                    players,
                    self.texture_pool.clone(),
                    self.texture_view_pool.clone(),
                    self.sound_data_pool.clone(),
                );
                let flow = GameSceneFlow::Change(Box::new(scene));
                let event = AppEvent::AddGameSceneFlow(flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            _ => {
                log::warn!(
                    "packet ignored: invalid packet received! (TYPE:{:?})",
                    packet_type,
                );
            }
        }

        None
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
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
        if !repeat && self.delay_time_sec <= 0.0 && !self.wait_for_response {
            if code == KeyCode::Escape {
                // 패킷을 전송합니다.
                let packet = MatchCancelPacket::new(self.uid, self.token);
                let net_manager = app.net_manager();
                let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                socket.push_packet(packet.as_raw());

                // 효과음을 재생합니다.
                if let Some(mixer) = app.audio_mixer() {
                    let decoded = self
                        .sound_data_pool
                        .get(UI_BUTTON_BACK)
                        .expect("UI_Button_Back sound must be preloaded!");
                    let source = decoded.as_source();
                    let sink = Sink::connect_new(mixer);
                    sink.set_volume(self.effect_volume as f32 / 255.0);
                    sink.append(source);
                    sink.play();
                    sink.detach();
                }

                self.wait_for_response = true;
            }
        }

        true
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
        let text = INFO_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let info_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let info_label = egui::Label::new(info_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 취소 버튼 텍스트
        let text = CANCEL_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

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
        let mut modal = egui::Modal::new(egui::Id::new("Matching_Modal"))
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

                let enable = !self.wait_for_response && self.delay_time_sec <= 0.0;
                let response = ui.add_enabled(enable, cancel_button);
                if response.clicked() {
                    // 패킷을 전송합니다.
                    let packet = MatchCancelPacket::new(self.uid, self.token);
                    let net_manager = app.net_manager();
                    let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                    socket.push_packet(packet.as_raw());

                    // 효과음을 재생합니다.
                    if let Some(mixer) = app.audio_mixer() {
                        let decoded = self
                            .sound_data_pool
                            .get(UI_BUTTON_BACK)
                            .expect("UI_Button_Back sound must be preloaded!");
                        let source = decoded.as_source();
                        let sink = Sink::connect_new(mixer);
                        sink.set_volume(self.effect_volume as f32 / 255.0);
                        sink.append(source);
                        sink.play();
                        sink.detach();
                    }

                    self.cancel_btn_state = ButtonState::Clicked;
                    self.wait_for_response = true;
                } else if response.is_pointer_button_down_on() {
                    self.cancel_btn_state = ButtonState::Pressed;
                } else if response.hovered() | response.has_focus() {
                    self.cancel_btn_state = ButtonState::Hovered;
                } else {
                    self.cancel_btn_state = ButtonState::Idle;
                }

                ui.add_space(18.0 * scale);
            });
        });
    }
}
