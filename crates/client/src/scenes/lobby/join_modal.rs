use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
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
    asset::{TexturePool, TextureViewPool, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::{CustomGameRoomScene, FatalErrorSceneLayer, BASE_WIDTH},
    SERVER_TCP_ADDR,
};

use super::MainLobbyMessageModalScene;

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 참여"];
/// 애플리케이션 표시 언어에 따른 `확인 버튼` 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];
/// 애플리케이션 표시 언어에 따른 `취소 버튼` 텍스트입니다.
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["취소"];
/// 애플리케이션 표시 언어에 따른 `방 번호 입력` 텍스트 입니다.
const INFORMATION_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 방 번호를 입력해 주세요"];
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

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl MainLobbyJoinModalScene {
    /// 새로운 `MainLobbyJoinModalScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            input_enabled: true,
            input_number: String::with_capacity(16),
            texture_pool,
            texture_view_pool,
        }
    }
}

impl GameScene for MainLobbyJoinModalScene {
    fn transparents(&self) -> bool {
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
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
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
                let event = AppEvent::AddGameSceneFlow(scene_flow);
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
                    self.texture_pool.clone(),
                    self.texture_view_pool.clone(),
                    packet.world_id,
                    packet.players,
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
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

        None
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 타이틀 텍스트
        let text = TITLE_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let title = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 안내 텍스트
        let text = INFORMATION_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let info_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `확인 버튼` 텍스트
        let text = OKAY_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `취소 버튼` 텍스트
        let text = CANCEL_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 텍스트 입력기
        let mut input_changed = false;
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let editor = egui::TextEdit::singleline(&mut self.input_number)
            .font(font_id)
            .char_limit(8)
            .min_size(egui::vec2(272.0 * scale, 52.0 * scale))
            .text_color(egui::Color32::BLACK)
            .background_color(egui::Color32::LIGHT_GRAY);

        // 확인 버튼
        let mut requested = false;
        let okay_button = egui::Button::new(okay_text)
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .min_size((128.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        // 취소 버튼
        let cancel_button = egui::Button::new(cancel_text)
            .fill(egui::Color32::WHITE)
            .corner_radius(3.0)
            .min_size((128.0 * scale, 72.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Join_Custom_Modal"))
            .frame(frame)
            .show(app.egui_ctx(), |ui| {
                ui.set_max_width(640.0 * scale);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(title).sense(egui::Sense::empty()));
                    ui.separator();

                    ui.add_space(8.0 * scale);
                    ui.add(egui::Label::new(info_text).sense(egui::Sense::empty()));
                    ui.add_space(8.0 * scale);
                    ui.add_enabled_ui(self.input_enabled, |ui| {
                        if ui
                            .add_sized((272.0 * scale, 52.0 * scale), editor)
                            .changed()
                        {
                            input_changed = true;
                        }
                    });
                    ui.add_space(8.0 * scale);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.set_max_height(72.0 * scale);
                        ui.add_space(16.0 * scale);
                        if ui.add(okay_button).clicked() {
                            requested = true;
                        }

                        if ui.add(cancel_button).clicked() {
                            // 이전 게임 장면으로 복귀합니다.
                            let scene_flow = GameSceneFlow::Pop;
                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                            let event_loop_proxy = app.event_loop_proxy();
                            event_loop_proxy.send_event(event).unwrap();
                        };
                    });
                    ui.add_space(8.0 * scale);
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
    }
}
