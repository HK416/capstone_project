use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{JoinFailedReason, LoginToken, UserId, WorldId},
    protocol::{
        CustomGameJoinFailedPacket, CustomGameJoinRequestPacket, CustomGameJoinSuccessPacket,
        Packet, PacketType, RawPacket,
    },
};
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::{CustomGameRoomScene, BASE_WIDTH},
    SERVER_TCP_ADDR,
};

use super::MainLobbyMessageModalScene;

/// 애플리케이션 표시 언어에 따른 `Head` 텍스트입니다.
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 참여"];
/// 애플리케이션 표시 언어에 따른 `확인 버튼` 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];
/// 애플리케이션 표시 언어에 따른 `취소 버튼` 텍스트입니다.
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["취소"];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 타이틀 텍스트입니다.
const MSG_MODAL_TEXTS: [&'static str; NUM_LOCALE] = ["알림"];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_NOT_FOUND_TEXTS: [&'static str; NUM_LOCALE] =
    ["해당 커스텀 게임 대기실이 존재하지 않습니다!"];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_FULL_CAPACITY_TEXTS: [&'static str; NUM_LOCALE] =
    ["해당 커스텀 게임 대기실 인원이 가득찼습니다."];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_IN_PROGRASS_TEXTS: [&'static str; NUM_LOCALE] = ["이미 게임이 진행 중 입니다."];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_BANNED_TEXTS: [&'static str; NUM_LOCALE] = ["관리자로부터 차단당했습니다."];

/// 게임의 메인 로비 화면입니다.
/// 커스텀 게임에 참여하기 위한 모달 대화상자를 화면에 표시합니다.
pub struct MainLobbyJoinModalScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 클라이언트의 사용자 식별자입니다.
    user_id: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 버튼의 활성화 여부입니다.
    input_enabled: bool,

    /// 입력된 번호 데이터입니다.
    input_number: String,
}

impl MainLobbyJoinModalScene {
    /// 새로운 `MainLobbyJoinModalScene`을 생성합니다.
    pub fn new(locale: Locale, user_id: UserId, token: LoginToken) -> Self {
        Self {
            locale,
            user_id,
            token,
            input_enabled: true,
            input_number: String::with_capacity(16),
        }
    }
}

impl GameScene for MainLobbyJoinModalScene {
    fn transparents(&self) -> bool {
        true
    }

    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::CustomGameJoinFailed => {
                // 패킷을 생성합니다
                let packet = CustomGameJoinFailedPacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let i = self.locale as usize;
                let next_scene = Box::new(MainLobbyMessageModalScene::new(
                    self.locale,
                    MSG_MODAL_TEXTS[i],
                    match packet.reason {
                        JoinFailedReason::NotFound => ERR_NOT_FOUND_TEXTS[i],
                        JoinFailedReason::FullCapacity => ERR_FULL_CAPACITY_TEXTS[i],
                        JoinFailedReason::InProgress => ERR_IN_PROGRASS_TEXTS[i],
                        JoinFailedReason::Banned => ERR_BANNED_TEXTS[i],
                    },
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::CustomGameJoinSuccess => {
                // 패킷을 생성합니다
                let packet = CustomGameJoinSuccessPacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let next_scene = Box::new(CustomGameRoomScene::new(
                    self.locale,
                    self.user_id,
                    self.token,
                    packet.world_id,
                    packet.players,
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::LobbyPull => { /* IGNORED */ }
            _ => {
                log::warn!(
                    "packet ignored: invalid packet received! (TYPE:{:?})",
                    packet_type
                );
            }
        }
        Ok(())
    }

    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // `Head` 텍스트
        let text = HEAD_TEXTS[i];
        let font_id = egui::FontId::new(32.0 * scale, head_font_family.clone());
        let head_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // `확인 버튼` 텍스트
        let text = OKAY_TEXTS[i];
        let font_id = egui::FontId::new(24.0 * scale, main_font_family.clone());
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // `취소 버튼` 텍스트
        let text = CANCEL_TEXTS[i];
        let font_id = egui::FontId::new(24.0 * scale, main_font_family.clone());
        let cancel_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 텍스트 입력기
        let mut input_changed = false;
        let font_id = egui::FontId::new(24.0 * scale, main_font_family.clone());
        let editor = egui::TextEdit::singleline(&mut self.input_number)
            .font(font_id)
            .char_limit(8)
            .text_color(egui::Color32::DARK_GRAY)
            .background_color(egui::Color32::LIGHT_GRAY);

        // 확인 버튼
        let mut requested = false;
        let okay_button = egui::Button::new(okay_text)
            .fill(egui::Color32::LIGHT_GRAY)
            .corner_radius(3.0)
            .min_size((128.0 * scale, 72.0 * scale).into());

        // 취소 버튼
        let cancel_button = egui::Button::new(cancel_text)
            .fill(egui::Color32::LIGHT_GRAY)
            .corner_radius(3.0)
            .min_size((128.0 * scale, 72.0 * scale).into());

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::DARK_GRAY));
        egui::Modal::new(egui::Id::new("Join_Custom"))
            .frame(frame)
            .show(app.egui_ctx(), |ui| {
                ui.set_max_width(640.0 * scale);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(head_text);
                    ui.separator();

                    ui.add_enabled_ui(self.input_enabled, |ui| {
                        ui.add_space(16.0 * scale);

                        if ui.add(editor).changed() {
                            input_changed = true;
                        }

                        ui.add_space(16.0 * scale);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(okay_button).clicked() {
                                requested = true;
                            }

                            ui.add_space(16.0 * scale);

                            if ui.add(cancel_button).clicked() {
                                // 이전 게임 장면으로 복귀합니다.
                                let scene_flow = GameSceneFlow::Pop;
                                let event = AppEvent::SetGameSceneFlow(scene_flow);
                                let event_loop_proxy = app.event_loop_proxy();
                                event_loop_proxy.send_event(event).unwrap();
                            };
                        });
                    });
                });
            });

        if input_changed {
            self.input_number.retain(|c| c.is_ascii_digit());
        }

        if requested {
            if let Ok(val) = self.input_number.parse::<u32>() {
                self.input_enabled = false;

                // 패킷을 생성합니다.
                let world_id = WorldId::new(val);
                let packet = CustomGameJoinRequestPacket::new(world_id, self.user_id, self.token);

                // 패킷을 전송합니다.
                let net_manager = app.net_manager();
                let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                socket.push_packet(packet.as_raw());
            }
        }

        Ok(())
    }
}
