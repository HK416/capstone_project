//! 커스텀 게임 장면과 관련된 코드를 작성합니다.
//!
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        CustomRoomPlayerData, LoginToken, Permission, StageKind, Team, UserId, WorldId,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{
        Packet, PacketType, RawPacket, RoomDataUpdatePacket, RoomLeaveNotifyPacket,
        RoomReadyRequestPacket, StartFailedReason, StartGameFailedPacket,
    },
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{TexturePool, TextureViewPool, BG_MAIN_LOBBY_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
    },
    SERVER_TCP_ADDR,
};

use super::{MessageSceneLayer, BASE_WIDTH, TEAM_COLOR};

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 대기실"];
/// 애플리케이션 표시 언어에 따른 `준비 버튼` 텍스트
const READY_TEXTS: [&'static str; NUM_LOCALE] = ["준비"];
/// 애플리케이션 표시 언어에 따른 `시작 버튼` 텍스트
const START_TEXTS: [&'static str; NUM_LOCALE] = ["시작"];

/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["알림"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const NOT_ENOUGH_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["게임 참여 인원이 적습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const UNBALANCED_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["두 팀의 인원이 다릅니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const PLAYER_NOT_READY_ERR_TEXTS: [&'static str; NUM_LOCALE] =
    ["모든 플레이어가 준비되지 않았습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const EMPTY_BLUE_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["블루 팀 인원이 비어있습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const EMPTY_RED_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["레드 팀 인원이 비어있습니다"];

/// 커스텀 게임 대기실 장면입니다.
pub struct CustomGameRoomScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 클라이언트의 사용자 식별자입니다.
    uid: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 커스텀 게임 대기실의 월드 식별자입니다.
    world_id: WorldId,
    /// 지형 종류입니다.
    stage_kind: StageKind,
    /// 캐릭터 중복 허용 여부
    allow_duplicates: bool,
    /// 팀 밸런스 불균형 허용 여부
    allow_unbalanced: bool,
    /// 현재 커스텀 게임에 참가한 플레이어 목록입니다.
    players: Vec<CustomRoomPlayerData>,

    /// 배경화면 텍스처의 식별자입니다.
    bg_texture_id: egui::load::SizedTexture,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl CustomGameRoomScene {
    /// 새로운 `CustomGameRoomScene`을 생성합니다.
    ///
    /// # Panics
    /// `UserId` 또는 `LoginToken`이 NULL인 경우 `panic!`을 호출합니다.
    ///
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        world_id: WorldId,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        stage_kind: StageKind,
        allow_duplicates: bool,
        allow_unbalanced: bool,
        players: Vec<CustomRoomPlayerData>,
    ) -> Self {
        assert_ne!(uid, UserId::NULL, "invalid user identifier");
        assert_ne!(world_id, WorldId::NULL, "invalid world identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            locale,
            uid,
            token,
            world_id,
            players,
            stage_kind,
            allow_duplicates,
            allow_unbalanced,
            bg_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            texture_pool,
            texture_view_pool,
        }
    }
}

impl GameScene for CustomGameRoomScene {
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

    fn on_resume(&mut self, _window: &Window, _app: &dyn AppHandle) {}

    fn on_pause(&mut self, _window: &Window, _app: &dyn AppHandle) {}

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
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
            PacketType::RoomDataUpdate => {
                let packet = RoomDataUpdatePacket::from_raw(packet);
                self.stage_kind = packet.stage_kind();
                self.allow_duplicates = packet.allow_duplicates();
                self.allow_unbalanced = packet.allow_unbalanced();
                self.players = packet.players;
            }
            PacketType::StartGameFailed => {
                let packet = StartGameFailedPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let i = self.locale as usize;
                let next_scene = Box::new(MessageSceneLayer::new(
                    self.locale,
                    ERR_TITLE_TEXTS[i],
                    match packet.reason {
                        StartFailedReason::NotEnoughPlayers => NOT_ENOUGH_ERR_TEXTS[i],
                        StartFailedReason::UnbalancedTeams => UNBALANCED_ERR_TEXTS[i],
                        StartFailedReason::PlayersNotReady => PLAYER_NOT_READY_ERR_TEXTS[i],
                        StartFailedReason::EmptyBlueTeam => EMPTY_BLUE_ERR_TEXTS[i],
                        StartFailedReason::EmptyRedTeam => EMPTY_RED_ERR_TEXTS[i],
                    },
                    None,
                ));
                let scene_flow = GameSceneFlow::Push(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            _ => {
                log::warn!(
                    "packet ignored: invalid packet received! (TYPE:{:?})",
                    packet_type
                );
            }
        };

        None
    }

    fn on_update(&mut self, _: f32, _: &Window, app: &dyn AppHandle) {
        // if let Some(packet) = self.formation_packet.as_ref() {
        // let next_scene = Box::new(CharacterFormationScene::new(
        //     self.locale,
        //     self.user_id,
        //     self.token,
        //     self.texture_pool.clone(),
        //     self.texture_view_pool.clone(),
        //     packet.remaining_time,
        //     packet.players.clone(),
        // ));
        // let scene_flow = GameSceneFlow::Push(next_scene);
        // let event = AppEvent::AddGameSceneFlow(scene_flow);
        // let event_loop_proxy = app.event_loop_proxy();
        // event_loop_proxy.send_event(event).unwrap();
        // }
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

        // Head 텍스트
        let text = format!("{} - {}", HEAD_TEXTS[locale], self.world_id);
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let head_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 준비/시작 버튼 텍스트
        let mut other_players_ready = self.players.len() >= 2;
        let mut permission = Permission::User;
        let mut ready = false;
        for player in self.players.iter() {
            if self.uid == player.uid {
                permission = player.permission();
                ready = player.is_ready_to_play();
            } else {
                other_players_ready &= player.is_ready_to_play();
            }
        }
        let button_color = match ready {
            true => egui::Color32::YELLOW,
            false => egui::Color32::WHITE,
        };
        let enable_enter_button = permission == Permission::User
            || (permission == Permission::Admin && other_players_ready);
        let text = match permission {
            Permission::Admin => START_TEXTS[locale],
            Permission::User => READY_TEXTS[locale],
        };
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(48.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);
        let enter_button = egui::Button::new(text)
            .fill(button_color)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK))
            .corner_radius(1.5);

        // 나가기 버튼
        // TODO: 나중에 이미지 버튼으로 수정해야 함.
        let exit_button = egui::Button::new("X")
            .corner_radius(1.5)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

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

        let offset = clip_rect.min + egui::vec2(16.0, 16.0) * scale;
        egui::Area::new(egui::Id::new("Head_Layout"))
            .anchor(egui::Align2::LEFT_TOP, offset.to_vec2())
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_size(egui::vec2(1212.0, 64.0) * scale);
                ui.set_max_size(egui::vec2(1212.0, 64.0) * scale);

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.add(exit_button).clicked() {
                        // 패킷을 생성하고 전송합니다.
                        let packet = RoomLeaveNotifyPacket::new(self.uid, self.token);
                        let net_manager = app.net_manager();
                        let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                        socket.push_packet(packet.as_raw());

                        // 장면을 전환합니다.
                        let scene_flow = GameSceneFlow::Pop;
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    ui.label(head_text);
                });
            });

        egui::Area::new(egui::Id::new("List_Layout"))
            .anchor(egui::Align2::CENTER_CENTER, (-96.0 * scale, 48.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_size(egui::vec2(960.0, 500.0) * scale);
                ui.set_max_size(egui::vec2(960.0, 500.0) * scale);

                ui.columns(2, |cols| {
                    let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                    let font_id = egui::FontId::new(24.0 * scale, family);
                    let mut iter = self.players.iter();
                    for i in 0..MAX_IN_GAME_PLAYERS {
                        let ui = &mut cols[i % 2];
                        if let Some(player) = iter.next() {
                            let text = if player.uid == self.uid {
                                &format!("*me* {}", &player.name.to_string())
                            } else {
                                &player.name.to_string()
                            };
                            let text = egui::RichText::new(text)
                                .font(font_id.clone())
                                .color(egui::Color32::BLACK);
                            let button = egui::Button::new(text)
                                .corner_radius(1.0)
                                .min_size((470.0 * scale, 80.0 * scale).into())
                                .stroke(egui::Stroke::new(
                                    3.0 * scale,
                                    match player.team() {
                                        Team::Blue => match player.is_ready_to_play() {
                                            true => TEAM_COLOR[Team::Blue as usize],
                                            false => egui::Color32::DARK_BLUE,
                                        },
                                        Team::Red => match player.is_ready_to_play() {
                                            true => TEAM_COLOR[Team::Red as usize],
                                            false => egui::Color32::DARK_RED,
                                        },
                                    },
                                ))
                                .fill(match player.permission() {
                                    Permission::Admin => egui::Color32::YELLOW,
                                    Permission::User => egui::Color32::WHITE,
                                });
                            ui.add(button);
                        } else {
                            let button = egui::Button::new("")
                                .corner_radius(1.0)
                                .min_size((470.0 * scale, 80.0 * scale).into())
                                .stroke(egui::Stroke::new(3.0 * scale, egui::Color32::DARK_GRAY))
                                .fill(egui::Color32::LIGHT_GRAY);
                            ui.add(button);
                        }
                        ui.add_space(20.0 * scale);
                    }
                });
            });

        let offset = clip_rect.max - egui::vec2(16.0, 48.0) * scale;
        egui::Area::new(egui::Id::new("Control_Pannel"))
            .anchor(egui::Align2::RIGHT_BOTTOM, offset.to_vec2())
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.add_enabled_ui(enable_enter_button, |ui| {
                    ui.set_min_size(egui::vec2(200.0, 140.0) * scale);
                    ui.set_max_size(egui::vec2(200.0, 140.0) * scale);
                    ui.centered_and_justified(|ui| {
                        if ui.add(enter_button).clicked() {
                            // 패킷을 생성하고 전송합니다.
                            let packet = RoomReadyRequestPacket::new(self.uid, self.token, !ready);
                            let net_manager = app.net_manager();
                            let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                            socket.push_packet(packet.as_raw());
                        }
                    });
                })
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                egui::Image::new(source).paint_at(ui, rect);
            });
    }
}
