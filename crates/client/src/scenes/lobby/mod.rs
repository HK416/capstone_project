mod enter;
mod join;
mod layer;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{GameTier, LoginToken, ProfileIcon, UserId, UserName},
    protocol::{PacketType, RawPacket},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_MAIN_LOBBY_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR,
        PROFILE_BG_URI, PROFILE_ICON_URIS, RANK_ICON_URI,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
    },
};

pub use self::{enter::*, join::*, layer::*};

use super::BASE_WIDTH;

/// 애플리케이션 표시 언어에 따른 `커스텀 게임 생성` 버튼 텍스트입니다.
const CREATE_GAME_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["게임 생성"];
/// 애플리케이션 표시 언어에 따른 `커스텀 게임 참가` 버튼 텍스트입니다.
const JOIN_GAME_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["게임 참가"];

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
const ERR_LIMITS_TEXTS: [&'static str; NUM_LOCALE] = ["게임 월드 생성에 실패했습니다."];

/// 게임의 메인 로비 화면입니다.
pub struct MainLobbyScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 사용자 이름 (게임 장면이 유지되는 동안 존재합니다)
    name: UserName,
    /// 사용자 게임 티어
    tier: GameTier,
    /// 프로필 아이콘
    profile_icon: ProfileIcon,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 버튼의 활성화 여부입니다.
    button_enabled: bool,

    /// Ui 배경화면 텍스처
    bg_texture: egui::load::SizedTexture,
    /// Ui 프로필 배경 텍스처
    profile_bg_texture: egui::load::SizedTexture,
    /// Ui 프로필 아이콘 텍스처
    profile_icon_texture: egui::load::SizedTexture,
    /// Ui 프로필 랭크(티어) 아이콘 텍스처
    rank_texture: egui::load::SizedTexture,

    /// Ui 스케일
    ui_scale: f32,
    /// 클립 영역 사각형
    clip_rect: egui::Rect,
    /// Ui - 콘텐츠 레이아웃 영역입니다.
    content_layout_rect: egui::Rect,
    /// Ui - 콘텐츠 레이아웃의 곡률입니다.
    content_layout_corner: f32,
    /// Ui - 플레이어 이름입니다.
    player_name_text: egui::RichText,
    /// Ui - 프로필 정보 레이아웃 영역입니다.
    profile_layout_rect: egui::Rect,
    /// Ui - 프로필 캐릭터 아이콘 영역입니다.
    profile_character_rect: egui::Rect,
    /// Ui - 프로필 랭킹 아이콘 영역입니다.
    profile_rank_rect: egui::Rect,
    /// Ui - 배경화면 레이아웃 영역입니다.
    background_rect: egui::Rect,

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
        profile_icon: ProfileIcon,
        token: LoginToken,
        texture_pool: TexturePool,
    ) -> Self {
        Self {
            locale,
            uid,
            name,
            tier,
            profile_icon,
            token,
            button_enabled: true,
            bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            profile_bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            profile_icon_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            rank_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            content_layout_rect: egui::Rect::ZERO,
            content_layout_corner: 0.0,
            player_name_text: egui::RichText::default(),
            profile_layout_rect: egui::Rect::ZERO,
            profile_character_rect: egui::Rect::ZERO,
            profile_rank_rect: egui::Rect::ZERO,
            background_rect: egui::Rect::ZERO,
            texture_pool,
            texture_view_pool: TextureViewPool::new(),
        }
    }

    /// 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_background_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
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
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.bg_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 프로필 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_bg_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(PROFILE_BG_URI)
            .expect("Profile background texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_bg_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 프로필 랭크(티어) 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_rank_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(RANK_ICON_URI)
            .expect("Rank icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: self.tier as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.rank_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 프로필 캐릭터 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_character_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
    ) {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let uri = PROFILE_ICON_URIS[self.profile_icon as usize];
        let texture = self
            .texture_pool
            .get(uri)
            .expect("Profile character texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_icon_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// Ui의 크기를 재설정합니다.
    fn resize_ui(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형을 재설정합니다.
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        self.ui_scale = viewport.width / scale_factor / BASE_WIDTH;
        self.clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // Ui 콘텐츠 레이아웃 사각형의 크기를 재조정합니다.
        let min_y = self.clip_rect.min.y;
        let max_x = self.clip_rect.max.x;
        self.content_layout_rect = egui::Rect::from_min_max(
            egui::pos2(max_x - 420.0 * self.ui_scale, min_y - 20.0),
            egui::pos2(max_x + 20.0 * self.ui_scale, min_y + 72.0 * self.ui_scale),
        );
        self.content_layout_corner = 20.0 * self.ui_scale;

        // Ui 플레이어 이름 폰트 크기를 재조정합니다.
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(22.0 * self.ui_scale, family);
        self.player_name_text = egui::RichText::new(self.name)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // Ui 프로필 정보 사각형의 크기를 재조정합니다.
        const PROFILE_MARGIN: f32 = 32.0;
        const PROFILE_WIDTH: f32 = 420.0;
        static_assertions::const_assert!(0.0 < PROFILE_MARGIN);
        static_assertions::const_assert!(0.0 < PROFILE_WIDTH);
        static_assertions::const_assert!(PROFILE_WIDTH < BASE_WIDTH);
        let min_x = self.clip_rect.min.x;
        let max_y = self.clip_rect.max.y;
        let source = &self.profile_bg_texture;
        let image_ratio = source.size.x / source.size.y;
        let image_width = PROFILE_WIDTH * self.ui_scale;
        let image_height = image_width / image_ratio;
        self.profile_layout_rect = egui::Rect::from_min_max(
            egui::pos2(
                min_x + PROFILE_MARGIN * self.ui_scale,
                max_y - (PROFILE_MARGIN * self.ui_scale + image_height),
            ),
            egui::pos2(
                min_x + PROFILE_MARGIN + PROFILE_WIDTH * self.ui_scale,
                max_y - PROFILE_MARGIN * self.ui_scale,
            ),
        );

        // Ui 프로필 캐릭터 사각형의 크기를 재조정합니다.
        let source = &self.profile_icon_texture;
        let image_ratio = source.size.x / source.size.y;
        let image_height = self.profile_layout_rect.height() * 0.8;
        let image_width = image_height * image_ratio;
        let image_size = egui::vec2(image_width, image_height);
        let min = self.profile_layout_rect.min
            + egui::vec2(0.0, self.profile_layout_rect.height() * 0.05);
        self.profile_character_rect = egui::Rect::from_min_max(min, min + image_size);

        // Ui 프로필 랭킹 사각형의 크기를 재조정합니다.
        const HALF_RANK_WIDTH: f32 = 24.0;
        const RANK_WIDTH: f32 = HALF_RANK_WIDTH * 2.0;
        static_assertions::const_assert!(HALF_RANK_WIDTH <= PROFILE_MARGIN);
        static_assertions::const_assert!(0.0 < HALF_RANK_WIDTH);
        let source = &self.rank_texture;
        let image_ratio = source.size.x / source.size.y;
        let image_width = RANK_WIDTH * self.ui_scale;
        let image_height = image_width / image_ratio;
        let size = egui::vec2(image_width, image_height);
        let center = self.profile_layout_rect.left_top() + egui::vec2(4.0 * self.ui_scale, 0.0);
        self.profile_rank_rect = egui::Rect::from_center_size(center, size);

        // Ui 배경화면 영역을 재조정합니다.
        let source = &self.bg_texture;
        let center = self.clip_rect.center();
        let image_ratio = source.size.x / source.size.y;
        let image_width = self.clip_rect.width();
        let image_height = image_width / image_ratio;
        let image_size = egui::vec2(image_width, image_height);
        self.background_rect =
            egui::Rect::from_min_max(center - 0.5 * image_size, center + 0.5 * image_size);
    }

    /// Ui 콘텐츠를 그립니다.
    fn draw_ui_content_layout(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Content_layout")).show(ctx, |ui| {
            ui.shrink_clip_rect(self.clip_rect);
            ui.painter().rect(
                self.content_layout_rect,
                self.content_layout_corner,
                egui::Color32::WHITE,
                egui::Stroke::new(1.5 * self.ui_scale, egui::Color32::from_rgb(124, 208, 255)),
                egui::StrokeKind::Middle,
            );
        });
    }

    /// 플레이어 정보를 그립니다.
    fn draw_ui_player_info_layout(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new(egui::Id::new("Player_Info_Layout"))).show(ctx, |ui| {
            ui.shrink_clip_rect(self.clip_rect);

            // 레이아웃
            egui::Image::new(self.profile_bg_texture).paint_at(ui, self.profile_layout_rect);

            // 프로필 캐릭터
            egui::Image::new(self.profile_icon_texture).paint_at(ui, self.profile_character_rect);

            // 랭크
            egui::Image::new(self.rank_texture).paint_at(ui, self.profile_rank_rect);

            // 이름
            let label = egui::Label::new(self.player_name_text.clone())
                .wrap_mode(egui::TextWrapMode::Truncate)
                .halign(egui::Align::Center)
                .sense(egui::Sense::empty())
                .selectable(false);
            let label_rect = egui::Rect::from_min_max(
                self.profile_layout_rect.center_top()
                    - egui::vec2(self.profile_layout_rect.width() * 0.25, 0.0),
                self.profile_layout_rect.max,
            );
            ui.put(label_rect, label);
        });
    }

    /// 배경화면을 그립니다.
    fn draw_ui_background(&mut self, ctx: &egui::Context) {
        let source = self.bg_texture;
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                egui::Image::new(source).paint_at(ui, self.background_rect);
            });
    }
}

impl GameScene for MainLobbyScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_profile_rank_texture(device, ui_renderer);
        self.regist_background_texture(device, ui_renderer);
        self.regist_profile_bg_texture(device, ui_renderer);
        self.regist_profile_character_texture(device, ui_renderer);
        self.resize_ui(window, app);
    }

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        // Ui 레이아웃을 재조정합니다.
        self.resize_ui(window, app);
    }

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

    fn on_received_packet(&mut self, packet: RawPacket, _app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::RoomDataUpdate => { /* IGNORED */ }
            PacketType::LobbyDataUpdate => {}
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
        let ctx = app.egui_ctx();
        let locale = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 콘텐츠 레이아웃 그리기
        self.draw_ui_content_layout(ctx);

        // 플레이어 정보 그리기
        self.draw_ui_player_info_layout(ctx);

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
                        // 다음 게임 장면으로 전환합니다.
                        let next_scene = MainLobbyWaitLayer::new(
                            self.locale,
                            self.uid,
                            self.token,
                            self.texture_pool.clone(),
                            self.texture_view_pool.clone(),
                        );
                        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    if ui.put(join_rect, join_button).clicked() {
                        // 다음 게임 장면으로 전환합니다.
                        let next_scene = MainLobbyJoinModalScene::new(
                            self.locale,
                            self.uid,
                            self.token,
                            self.texture_pool.clone(),
                            self.texture_view_pool.clone(),
                        );
                        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
                        let event = AppEvent::AddGameSceneFlow(scene_flow);
                        let event_loop_proxy = app.event_loop_proxy();
                        event_loop_proxy.send_event(event).unwrap();
                    }
                });
            });
        });

        // 배경화면
        self.draw_ui_background(ctx);
    }
}
