use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        CharacterKind, FormationPhasePlayer, GamePlayStopReason, LoginToken, SelectResult, UserId,
        NUM_CHARACTERS,
    },
    protocol::{
        FormationPullPacket, FormationSelectPacket, FormationSelectResponsePacket,
        GamePlayStopPacket, InitStagePacket, Packet, PacketType, RawPacket,
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

use super::{InGameLoadScene, MessageSceneLayer, BASE_WIDTH};

/// 애플리케이션 표시 언어에 따른 Title 텍스트
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["캐릭터 편성"];
/// 애플리케이션 표시 언어에 따른 `남은 시간` 텍스트
const TIMER_TEXTS: [&'static str; NUM_LOCALE] = ["남은 시간"];
/// 애플리케이션 표시 언어에 따른 `캐릭터 선택` 텍스트
const SELECT_TEXTS: [&'static str; NUM_LOCALE] = ["캐릭터 선택"];

/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["알림"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const NOT_ENOUGH_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["게임 참여 인원이 적습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const EMPTY_TEAM_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["한쪽 팀 인원이 비어있습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const DUPLICATE_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["이미 사용중인 캐릭터입니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const BANNED_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["사용이 금지된 캐릭터입니다"];

/// 인 게임 장면에 진입하기 전 캐릭터를 편성하는 장면입니다.  
pub struct CharacterFormationScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 캐릭터 편성까지 남은 시간
    remaining_time_sec: f32,

    /// 플레이어 집합
    players: Vec<FormationPhasePlayer>,

    /// 배경화면 텍스처의 식별자입니다.
    bg_texture_id: egui::load::SizedTexture,
    /// 현재 선택한 캐릭터 종류
    select_character: Option<CharacterKind>,
    /// 캐릭터 선택 여부
    is_selected: bool,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl CharacterFormationScene {
    /// 새로운 게임 장면을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        remaining_time_sec: f32,
        players: Vec<FormationPhasePlayer>,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            remaining_time_sec,
            players,
            bg_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            select_character: None,
            is_selected: false,
            texture_pool,
            texture_view_pool,
        }
    }
}

impl GameScene for CharacterFormationScene {
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
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i]
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
            PacketType::FormationSelectResponse => {
                let packet = FormationSelectResponsePacket::from_raw(packet);
                match packet.result {
                    SelectResult::Success => self.is_selected = true,
                    SelectResult::Duplicates => {
                        // 다음 게임 장면으로 전환합니다.
                        let i = self.locale as usize;
                        let next_scene = Box::new(MessageSceneLayer::new(
                            self.locale,
                            ERR_TITLE_TEXTS[i],
                            DUPLICATE_ERR_TEXTS[i],
                        ));
                        let scene_flow = GameSceneFlow::Push(next_scene);
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    SelectResult::Banned => {
                        // 다음 게임 장면으로 전환합니다.
                        let i = self.locale as usize;
                        let next_scene = Box::new(MessageSceneLayer::new(
                            self.locale,
                            ERR_TITLE_TEXTS[i],
                            BANNED_ERR_TEXTS[i],
                        ));
                        let scene_flow = GameSceneFlow::Push(next_scene);
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                }
            }
            PacketType::FormationPull => {
                let packet = FormationPullPacket::from_raw(packet);
                self.remaining_time_sec = packet.remaining_time;
                self.players = packet.players;
            }
            PacketType::GamePlayStop => {
                // 게임 장면을 변경합니다.
                let scene_flow = GameSceneFlow::Pop;
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();

                let packet = GamePlayStopPacket::from_raw(packet);
                match packet.reason {
                    GamePlayStopReason::NotEnughPlayers => {
                        // 다음 게임 장면으로 전환합니다.
                        let i = self.locale as usize;
                        let next_scene = Box::new(MessageSceneLayer::new(
                            self.locale,
                            ERR_TITLE_TEXTS[i],
                            NOT_ENOUGH_ERR_TEXTS[i],
                        ));
                        let scene_flow = GameSceneFlow::Push(next_scene);
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    GamePlayStopReason::OneTeamEmpty => {
                        // 다음 게임 장면으로 전환합니다.
                        let i = self.locale as usize;
                        let next_scene = Box::new(MessageSceneLayer::new(
                            self.locale,
                            ERR_TITLE_TEXTS[i],
                            EMPTY_TEAM_ERR_TEXTS[i],
                        ));
                        let scene_flow = GameSceneFlow::Push(next_scene);
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                };
            }
            PacketType::InitStage => {
                let packet = InitStagePacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let next_scene =
                    InGameLoadScene::new(self.locale, self.user_id, self.token, packet);
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            _ => {
                log::warn!(
                    "ignored >> invalid packet received! (TYPE:{:?})",
                    packet_type
                );
            }
        }

        None
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn mod_app::app::AppHandle,
    ) {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!(
                    "RenderPass({})",
                    stringify!(CharacterFormationScene)
                )),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn mod_app::app::AppHandle) {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 타이틀 텍스트
        let text = TITLE_TEXTS[i];
        let font_id = egui::FontId::new(32.0 * scale, head_font_family.clone());
        let title_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 남은 시간 텍스트
        let timer = self.remaining_time_sec.floor() as u16;
        let text = format!("{}:{}", TIMER_TEXTS[i], timer);
        let font_id = egui::FontId::new(28.0 * scale, head_font_family.clone());
        let timer_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 캐릭터 선택 버튼 텍스트
        let text = SELECT_TEXTS[i];
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let select_btn_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 캐릭터 선택 버튼
        let enable_button = self.select_character.is_some();
        let button_color = match self.is_selected {
            true => egui::Color32::YELLOW,
            false => egui::Color32::LIGHT_GRAY,
        };
        let select_button = egui::Button::new(select_btn_text)
            .fill(button_color)
            .corner_radius(1.5)
            .min_size((420.0 * scale, 84.0 * scale).into());

        // 캐릭터 버튼
        const CHARACTERS: [CharacterKind; NUM_CHARACTERS] = [
            CharacterKind::ArisOriginal,
            CharacterKind::MidoriOriginal,
            CharacterKind::MomoiOriginal,
            CharacterKind::YuukaOriginal,
        ];
        let enable_character_button = !self.is_selected;
        let character_buttons: Vec<_> = CHARACTERS
            .into_iter()
            .map(|kind| {
                egui::Button::new(kind.to_string())
                    .fill(egui::Color32::DARK_GRAY)
                    .corner_radius(1.5)
                    .min_size((200.0 * scale, 84.0 * scale).into())
                    .stroke(egui::Stroke::new(
                        1.0,
                        match self.select_character {
                            Some(character) if character == kind => egui::Color32::YELLOW,
                            _ => egui::Color32::BLACK,
                        },
                    ))
            })
            .collect();

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

        egui::Area::new(egui::Id::new("Title_Layout"))
            .anchor(egui::Align2::LEFT_TOP, (16.0 * scale, 16.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(title_text);
                });
            });

        egui::Area::new(egui::Id::new("Timer_Layout"))
            .anchor(egui::Align2::CENTER_TOP, (0.0 * scale, 24.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(timer_text);
                });
            });

        egui::Area::new(egui::Id::new("Character_Btn_Layout"))
            .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
            .show(app.egui_ctx(), |ui| {
                ui.add_enabled_ui(enable_character_button, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.columns(2, |cols| {
                            for (i, button) in character_buttons.into_iter().enumerate() {
                                let ui = &mut cols[i % 2];
                                if ui.add(button).clicked() {
                                    self.select_character = Some(CHARACTERS[i]);
                                }
                            }
                        });
                    });
                })
            });

        egui::Area::new(egui::Id::new("Select_Btn_Layout"))
            .anchor(egui::Align2::CENTER_BOTTOM, (0.0 * scale, -64.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(enable_button, |ui| {
                        if ui.add(select_button).clicked() {
                            if !self.is_selected {
                                // 패킷을 전송합니다.
                                let packet = FormationSelectPacket::new(
                                    self.user_id,
                                    self.token,
                                    self.select_character
                                        .expect("there are no selected character!"),
                                );
                                let net_manager = app.net_manager();
                                let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                                socket.push_packet(packet.as_raw());
                            }
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
