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
        LoginToken, Permission, RecruitPhasePlayer, Team, UserId, WorldId, MAX_IN_GAME_PLAYERS,
    },
    protocol::{
        CustomGameLeavePacket, CustomGamePullPacket, CustomGameReadyPacket,
        CustomGameStartFailedPacket, FormationPullPacket, Packet, PacketType, RawPacket,
    },
};
use winit::window::Window;

use crate::{
    asset::{TexturePool, TextureViewPool, BG_MAIN_LOBBY_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    scenes::FatalErrorSceneLayer,
    SERVER_TCP_ADDR,
};

use super::{CharacterFormationScene, BASE_WIDTH, TEAM_COLOR};

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 대기실"];
/// 애플리케이션 표시 언어에 따른 `준비 버튼` 텍스트
const READY_TEXTS: [&'static str; NUM_LOCALE] = ["준비"];
/// 애플리케이션 표시 언어에 따른 `시작 버튼` 텍스트
const START_TEXTS: [&'static str; NUM_LOCALE] = ["시작"];

/// 커스텀 게임 대기실 장면입니다.
pub struct CustomGameRoomScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 클라이언트의 사용자 식별자입니다.
    user_id: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 게임 장면의 활성화 여부입니다.
    is_active: bool,

    /// 커스텀 게임 대기실의 월드 식별자입니다.
    world_id: WorldId,
    /// 현재 커스텀 게임에 참가한 플레이어 목록입니다.
    players: Vec<RecruitPhasePlayer>,

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
    pub fn new<I>(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        world_id: WorldId,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = RecruitPhasePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        assert_ne!(user_id, UserId::NULL, "invalid user identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            locale,
            user_id,
            token,
            is_active: true,
            world_id,
            players: iter.into_iter().collect(),
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
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
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
        let mut egui_renderer = app.egui_renderer_mut();
        let texture_id = egui_renderer.register_native_texture(
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

    fn on_resume(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.is_active = true;
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
        if !self.is_active {
            return Some(packet);
        }

        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::CustomGamePull => {
                let packet = CustomGamePullPacket::from_raw(packet);
                self.players = packet.players;
            }
            PacketType::FormationPull => {
                let packet = FormationPullPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                self.is_active = false;
                let next_scene = Box::new(CharacterFormationScene::new(
                    self.locale,
                    self.user_id,
                    self.token,
                    self.texture_pool.clone(),
                    self.texture_view_pool.clone(),
                    packet.remaining_time,
                    packet.players,
                ));
                let scene_flow = GameSceneFlow::Push(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::CustomGameStartFailed => {
                let packet = CustomGameStartFailedPacket::from_raw(packet);
                println!("{:?}", packet.reason);
                // TODO : 오류 메시지 네비게이션 모달 띄우기
            }
            _ => {
                log::warn!(
                    "ignored >> invalid packet received! (TYPE:{:?})",
                    packet_type
                );
            }
        };

        None
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성

        // Head 텍스트
        let text = format!("{} - {}", HEAD_TEXTS[i], self.world_id);
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
            if self.user_id == player.account.uid {
                permission = player.permission();
                ready = player.is_ready();
            } else {
                other_players_ready &= player.is_ready();
            }
        }
        let button_color = match ready {
            true => egui::Color32::YELLOW,
            false => egui::Color32::WHITE,
        };
        let enable_enter_button = permission == Permission::User
            || (permission == Permission::Admin && other_players_ready);
        let text = match permission {
            Permission::Admin => START_TEXTS[i],
            Permission::User => READY_TEXTS[i],
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
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let img_width = width;
        let img_height = img_width / ratio;
        let rect = egui::Rect {
            min: egui::pos2(
                (center_x - 0.5 * img_width) / scale_factor,
                (center_y - 0.5 * img_height) / scale_factor,
            ),
            max: egui::pos2(
                (center_x + 0.5 * img_width) / scale_factor,
                (center_y + 0.5 * img_height) / scale_factor,
            ),
        };

        egui::Area::new(egui::Id::new("Head_Layout"))
            .anchor(egui::Align2::LEFT_TOP, (16.0 * scale, 16.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.add(exit_button).clicked() {
                        // 패킷을 생성하고 전송합니다.
                        let packet = CustomGameLeavePacket::new(self.user_id, self.token);
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
                ui.set_width(960.0 * scale);
                ui.set_height(500.0 * scale);

                ui.columns(2, |cols| {
                    let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                    let font_id = egui::FontId::new(24.0 * scale, family);
                    let mut iter = self.players.iter();
                    for i in 0..MAX_IN_GAME_PLAYERS {
                        let ui = &mut cols[i % 2];
                        if let Some(player) = iter.next() {
                            let text = if player.account.uid == self.user_id {
                                &format!("*me* {}", &player.account.name.to_string())
                            } else {
                                &player.account.name.to_string()
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
                                        Team::Blue => match player.is_ready() {
                                            true => TEAM_COLOR[Team::Blue as usize],
                                            false => egui::Color32::DARK_BLUE,
                                        },
                                        Team::Red => match player.is_ready() {
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

        egui::Area::new(egui::Id::new("Control_Pannel"))
            .anchor(egui::Align2::RIGHT_BOTTOM, (-16.0 * scale, -48.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.add_enabled_ui(enable_enter_button, |ui| {
                    ui.set_width(200.0 * scale);
                    ui.set_height(140.0 * scale);
                    ui.centered_and_justified(|ui| {
                        if ui.add(enter_button).clicked() {
                            // 패킷을 생성하고 전송합니다.
                            let packet =
                                CustomGameReadyPacket::new(self.user_id, self.token, !ready);
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
                egui::Image::new(source).paint_at(ui, rect);
            });
    }
}
