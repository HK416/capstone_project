mod enter;
mod join_modal;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{CharacterKind, GameTier, LoginToken, UserId, UserName, WorldId},
    protocol::{
        Packet, PacketType, RawPacket,
    },
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{TexturePool, TextureViewPool, BG_MAIN_LOBBY_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::FatalErrorSceneLayer,
    SERVER_TCP_ADDR,
};

pub use self::{enter::*, join_modal::*};

use super::{BASE_WIDTH};

/// 애플리케이션 표시 언어에 따른 `커스텀 게임 생성` 버튼 텍스트입니다.
const CREATE_GAME_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["게임 생성"];
/// 애플리케이션 표시 언어에 따른 `커스텀 게임 참가` 버튼 텍스트입니다.
const JOIN_GAME_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["게임 참가"];

/// 게임의 메인 로비 화면입니다.
pub struct MainLobbyScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 사용자 이름 (게임 장면이 유지되는 동안 존재합니다)
    name: Option<UserName>,
    /// 사용자 게임 티어
    tier: GameTier,
    /// 프로필 캐릭터
    profile_character: Option<CharacterKind>,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 버튼의 활성화 여부입니다.
    button_enabled: bool,

    /// 배경화면 텍스처의 식별자입니다.
    bg_texture_id: egui::load::SizedTexture,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl MainLobbyScene {
    /// 새로운 `MainLobbyScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_character: Option<CharacterKind>,
        token: LoginToken,
        texture_pool: TexturePool,
    ) -> Self {
        Self {
            locale,
            uid,
            name: Some(name),
            tier,
            profile_character,
            token,
            button_enabled: true,
            bg_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            texture_pool,
            texture_view_pool: TextureViewPool::new(),
        }
    }
}

impl GameScene for MainLobbyScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(BG_MAIN_LOBBY_URI)
            .expect("BG_Main_Lobby texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id = ui_renderer.register_native_texture(
            app.render_device(),
            &texture,
            wgpu::FilterMode::Linear,
        );

        // 등록된 텍스처 정보를 저장합니다.
        self.bg_texture_id = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
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
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            // PacketType::JoinSuccess => {
            //     // 패킷을 생성합니다
            //     let packet = CustomGameJoinSuccessPacket::from_raw(packet);

            //     // 게임 장면을 변경합니다.
            //     let next_scene = CustomGameRoomScene::new(
            //         self.locale,
            //         self.user_info.uid,
            //         self.token,
            //         self.texture_pool.clone(),
            //         self.texture_view_pool.clone(),
            //         packet.world_id,
            //         packet.players,
            //     );
            //     let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
            //     let event = AppEvent::AddGameSceneFlow(scene_flow);
            //     let event_loop_proxy = app.event_loop_proxy();
            //     event_loop_proxy.send_event(event).unwrap();
            // }
            PacketType::LobbyPull => {}
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
        let locale = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 플레이어 정보 텍스트
        let name = self.name.as_ref().unwrap();
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let text = egui::RichText::new(name.clone())
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);
        let label_widget = egui::Label::new(text)
            .halign(egui::Align::Min)
            .wrap_mode(egui::TextWrapMode::Extend);

        // 플레이어 정보 텍스트 출력
        egui::Area::new(egui::Id::new("Player_Info"))
            .fixed_pos(clip_rect.min + egui::vec2(8.0, 8.0) * scale)
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(1280.0 * scale);
                ui.set_max_width(1280.0 * scale);
                ui.add(label_widget);
            });

        // 게임 생성 버튼
        let text = CREATE_GAME_BTN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(48.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);
        let create_button = egui::Button::new(text)
            .fill(egui::Color32::WHITE)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        let create_rect = egui::Rect::from_min_max(
            clip_rect.min + egui::vec2(1004.0 * scale, 536.0 * scale),
            clip_rect.min + egui::vec2(1264.0 * scale, 616.0 * scale),
        );

        // 게임 참가 버튼
        let text = JOIN_GAME_BTN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(48.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);
        let join_button = egui::Button::new(text)
            .fill(egui::Color32::WHITE)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        let join_rect = egui::Rect::from_min_max(
            clip_rect.min + egui::vec2(1004.0 * scale, 624.0 * scale),
            clip_rect.min + egui::vec2(1264.0 * scale, 704.0 * scale),
        );

        egui::Area::new(egui::Id::new("Game")).show(app.egui_ctx(), |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_enabled_ui(self.button_enabled, |ui| {
                    ui.shrink_clip_rect(clip_rect);

                    if ui.put(create_rect, create_button).clicked() {
                        // // 커스텀 게임 생성 패킷을 생성합니다.
                        // let packet = CustomGameJoinRequestPacket::new(
                        //     WorldId::NULL,
                        //     self.user_info.uid,
                        //     self.token,
                        // );

                        // // 패킷을 전송합니다.
                        // let net_manager = app.net_manager();
                        // let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                        // socket.push_packet(packet.as_raw());
                        // return;
                    }

                    if ui.put(join_rect, join_button).clicked() {
                        // // 다음 게임 장면으로 전환합니다.
                        // let next_scene = MainLobbyJoinModalScene::new(
                        //     self.locale,
                        //     self.user_info.uid,
                        //     self.token,
                        //     self.texture_pool.clone(),
                        //     self.texture_view_pool.clone(),
                        // );
                        // let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
                        // let event = AppEvent::AddGameSceneFlow(scene_flow);
                        // let event_loop_proxy = app.event_loop_proxy();
                        // event_loop_proxy.send_event(event).unwrap();
                    }
                });
            });
        });

        // 배경화면
        let source = self.bg_texture_id;
        let ratio = source.size.x / source.size.y;
        let center_x = 1280.0 * 0.5 * scale;
        let center_y = 720.0 * 0.5 * scale;
        let img_width = 1280.0 * scale;
        let img_height = img_width / ratio;
        let rect = egui::Rect {
            min: egui::pos2(
                clip_rect.min.x + center_x - 0.5 * img_width,
                clip_rect.min.y + center_y - 0.5 * img_height,
            ),
            max: egui::pos2(
                clip_rect.min.x + center_x + 0.5 * img_width,
                clip_rect.min.y + center_y + 0.5 * img_height,
            ),
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                egui::Image::new(source).paint_at(ui, rect);
            });
    }
}
