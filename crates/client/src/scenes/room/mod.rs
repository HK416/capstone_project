//! 커스텀 게임 장면과 관련된 코드를 작성합니다.
//!
use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        CustomGamePlayer, CustomGameStatus, LoginToken, Permission, Team, UserId, WorldId,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{
        CustomGameLeavePacket, CustomGamePullPacket, CustomGamePushStatusPacket, Packet,
        PacketType, RawPacket,
    },
};
use mod_render::{TexturePool, TextureViewPool};
use winit::window::Window;

use crate::{
    asset::{BG_MAIN_LOBBY_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, NUM_LOCALE},
    SERVER_TCP_ADDR,
};

use super::BASE_WIDTH;

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

    /// 커스텀 게임 대기실의 월드 식별자입니다.
    world_id: WorldId,
    /// 현재 커스텀 게임에 참가한 플레이어 목록입니다.  
    /// 사용자 식별자의 오름차순으로 정렬됩니다.
    players: Vec<CustomGamePlayer>,

    /// 배경화면 텍스처의 식별자입니다.
    bg_texture_id: egui::load::SizedTexture,
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
        world_id: WorldId,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = CustomGamePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        assert_ne!(user_id, UserId::NULL, "invalid user identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            locale,
            user_id,
            token,
            world_id,
            players: iter.into_iter().collect(),
            bg_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
        }
    }
}

impl GameScene for CustomGameRoomScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let texture =
            TexturePool::get(BG_MAIN_LOBBY_URI).expect("BG_Main_Lobby texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture =
            TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

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

        Ok(())
    }

    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::CustomGamePull => {
                let packet = CustomGamePullPacket::from_raw(packet);
                self.players = packet.players;
                self.players
                    .sort_by(|lhs, rhs| rhs.info.uid.cmp(&lhs.info.uid));
            }
            _ => {
                log::warn!("invalid packet received! (TYPE:{:?})", packet_type);
            }
        };

        Ok(())
    }

    fn on_draw(
        &self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({})", stringify!(CustomGameRoomScene))),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        Ok(())
    }

    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // Head 텍스트
        let text = format!("{} - {}", HEAD_TEXTS[i], self.world_id);
        let font_id = egui::FontId::new(32.0 * scale, head_font_family.clone());
        let head_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 준비/시작 버튼 텍스트
        let mut other_players_ready = self.players.len() >= 2;
        let mut current_permission = Permission::User;
        let mut current_status = CustomGameStatus::Wait;
        for player in self.players.iter() {
            if self.user_id == player.info.uid {
                current_permission = player.permission;
                current_status = player.status;
            } else {
                other_players_ready &= player.status == CustomGameStatus::Ready;
            }
        }
        let button_color = match current_status {
            CustomGameStatus::Ready => egui::Color32::YELLOW,
            _ => egui::Color32::WHITE,
        };
        let enable_enter_button = current_permission == Permission::User
            || (current_permission == Permission::Admin && other_players_ready);
        let text = match current_permission {
            Permission::Admin => START_TEXTS[i],
            Permission::User => READY_TEXTS[i],
        };
        let font_id = egui::FontId::new(48.0 * scale, head_font_family.clone());
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);
        let enter_button = egui::Button::new(text)
            .fill(button_color)
            .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK))
            .corner_radius(1.5);

        // 나가기 버튼
        // TODO: 나중에 이미지 버튼으로 수정해야 함.
        let exit_button = egui::Button::new("X")
            .corner_radius(1.5)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::DARK_GRAY));

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
                        let event = AppEvent::SetGameSceneFlow(scene_flow);
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
                    let font_id = egui::FontId::new(24.0 * scale, main_font_family);
                    let mut iter = self.players.iter();
                    for i in 0..MAX_IN_GAME_PLAYERS {
                        let ui = &mut cols[i % 2];
                        if let Some(player) = iter.next() {
                            let text = if player.info.uid == self.user_id {
                                &format!("*{}", &player.info.name.to_string())
                            } else {
                                &player.info.name.to_string()
                            };
                            let text = egui::RichText::new(text)
                                .font(font_id.clone())
                                .color(egui::Color32::DARK_GRAY);
                            let button = egui::Button::new(text)
                                .corner_radius(1.0)
                                .min_size((470.0 * scale, 80.0 * scale).into())
                                .stroke(egui::Stroke::new(
                                    3.0,
                                    match player.team {
                                        Team::Blue => match player.status {
                                            CustomGameStatus::Ready => egui::Color32::BLUE,
                                            _ => egui::Color32::DARK_BLUE,
                                        },
                                        Team::Red => match player.status {
                                            CustomGameStatus::Ready => egui::Color32::RED,
                                            _ => egui::Color32::DARK_RED,
                                        },
                                    },
                                ))
                                .fill(match player.permission {
                                    Permission::Admin => egui::Color32::YELLOW,
                                    Permission::User => egui::Color32::WHITE,
                                });
                            ui.add(button);
                        } else {
                            let button = egui::Button::new("")
                                .corner_radius(1.0)
                                .min_size((470.0 * scale, 80.0 * scale).into())
                                .stroke(egui::Stroke::new(3.0, egui::Color32::DARK_GRAY))
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
                            let packet = CustomGamePushStatusPacket::new(
                                self.user_id,
                                self.token,
                                #[allow(unreachable_patterns)]
                                match current_status {
                                    CustomGameStatus::Ready => CustomGameStatus::Wait,
                                    CustomGameStatus::Wait => CustomGameStatus::Ready,
                                    _ => current_status,
                                },
                            );
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

        Ok(())
    }
}
