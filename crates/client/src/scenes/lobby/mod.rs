mod enter;
mod join;
mod layer;

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, Viewport},
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
        TexturePool, TextureViewPool, BG_MAIN_LOBBY_URI, EMBLEM_BG_URI, HUD_LAYOUT_URI_02,
        NOTOSANS_BOLD, NOTOSANS_REGULAR, PROFILE_ICON_URI,
    },
    component::ButtonState,
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

    /// Ui 스케일
    ui_scale: f32,
    /// 클립 영역 사각형
    clip_rect: egui::Rect,
    /// 배경화면 텍스처
    bg_texture: egui::load::SizedTexture,
    /// 배경화면 레이아웃 영역입니다.
    bg_rect: egui::Rect,

    /// 프로필 배경 텍스처
    profile_bg_texture: egui::load::SizedTexture,
    /// 프로필 정보 레이아웃 영역입니다.
    profile_bg_rect: egui::Rect,

    /// 프로필 아이콘 텍스처
    profile_icon_texture: egui::load::SizedTexture,
    /// 프로필 아이콘 영역입니다.
    profile_icon_rect: egui::Rect,
    /// 플레이어 이름입니다.
    player_name_text: egui::RichText,

    /// 상단 패널의 배경 텍스처입니다.
    pannel_bg_texture: egui::load::SizedTexture,
    /// 상단 패널의 레이아웃 영역입니다.
    pannel_bg_rect: egui::Rect,

    /// 종료 버튼 레이아웃 영역입니다.
    exit_btn_rect: egui::Rect,
    /// 종료 버튼 상태
    exit_btn_state: ButtonState,

    /// 옵션 버튼 레이아웃 영역입니다.
    option_btn_rect: egui::Rect,
    /// 옵션 버튼 상태
    option_btn_state: ButtonState,

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
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            bg_rect: egui::Rect::ZERO,
            profile_bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            profile_bg_rect: egui::Rect::ZERO,
            profile_icon_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            profile_icon_rect: egui::Rect::ZERO,
            player_name_text: egui::RichText::default(),
            pannel_bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            pannel_bg_rect: egui::Rect::ZERO,
            exit_btn_rect: egui::Rect::ZERO,
            exit_btn_state: ButtonState::Idle,
            option_btn_rect: egui::Rect::ZERO,
            option_btn_state: ButtonState::Idle,
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
        // 프로필 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(EMBLEM_BG_URI)
            .expect("Emblem_BG texture must be preloaded!");
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
        self.profile_bg_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 프로필 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_icon_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 프로필 아이콘 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(PROFILE_ICON_URI)
            .expect("Profile_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: self.profile_icon as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_icon_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 패널 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_pannel_bg_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 패널 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_LAYOUT_URI_02)
            .expect("HUD_Layout_02 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.pannel_bg_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 클립 사각형 영역의 크기를 재조정합니다.
    fn resize_clip_rect(viewport: &Viewport, scale_factor: f32) -> (egui::Rect, f32) {
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        (clip_rect, scale)
    }

    /// 배경 사각형 영역의 크기를 재조정합니다.
    fn resize_background(texture_size: &egui::Vec2, clip_rect: &egui::Rect) -> egui::Rect {
        let center = clip_rect.center();
        let ratio = texture_size.x / texture_size.y;
        let width = clip_rect.width();
        let height = width / ratio;
        let size = egui::vec2(width, height);
        egui::Rect::from_center_size(center, size)
    }

    /// 프로필 배경 사각형 영역의 크기를 재조정합니다.
    fn resize_profile_background(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> egui::Rect {
        const MARGIN: egui::Vec2 = egui::vec2(16.0, 16.0);
        const WIDTH: f32 = 420.0;
        static_assertions::const_assert!(0.0 <= MARGIN.x && 0.0 <= MARGIN.y);
        static_assertions::const_assert!(0.0 < WIDTH);
        static_assertions::const_assert!(WIDTH < BASE_WIDTH);

        let ratio = texture_size.x / texture_size.y;
        let width = WIDTH * scale;
        let height = width / ratio;
        let min = clip_rect.min + MARGIN * scale;
        let size = egui::vec2(width, height);
        egui::Rect::from_min_size(min, size)
    }

    /// 프로필 아이콘의 크기를 재조정합니다.
    fn resize_profile_icon(texture_size: &egui::Vec2, profile_rect: &egui::Rect) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let height = profile_rect.height() * 0.85;
        let width = height * ratio;
        let margin = egui::vec2(0.0, profile_rect.height() * 0.05);
        let min = profile_rect.min + margin;
        let size = egui::vec2(width, height);
        egui::Rect::from_min_size(min, size)
    }

    /// 상단 패널 배경화면의 크기를 재조정합니다.
    fn resize_pannel_background(clip_rect: &egui::Rect, scale: f32) -> egui::Rect {
        const MARGIN: egui::Vec2 = egui::vec2(24.0, 16.0);
        const WIDTH: f32 = 240.0;
        const HEIGHT: f32 = 48.0;
        static_assertions::const_assert!(0.0 <= MARGIN.x && 0.0 <= MARGIN.y);
        static_assertions::const_assert!(0.0 <= WIDTH);
        static_assertions::const_assert!(0.0 <= HEIGHT && HEIGHT <= WIDTH);
        static_assertions::const_assert!(WIDTH <= BASE_WIDTH);

        let width = WIDTH * scale;
        let height = HEIGHT * scale;
        let size = egui::vec2(width, height);
        let min = clip_rect.right_top()
            + MARGIN * egui::vec2(-1.0, 1.0) * scale
            + size * egui::vec2(-1.0, 0.0);
        egui::Rect::from_min_size(min, size)
    }

    /// Ui의 크기를 재설정합니다.
    fn resize_ui(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형 영역의 크기를 재조정합니다.
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        (self.clip_rect, self.ui_scale) = Self::resize_clip_rect(viewport, scale_factor);

        // 배경 사각형 영역의 크기를 재조정합니다.
        let texture_size = &self.bg_texture.size;
        self.bg_rect = Self::resize_background(texture_size, &self.clip_rect);

        // 프로필 배경 사각형 영역의 크기를 재조정합니다.
        let texture_size = &self.profile_bg_texture.size;
        self.profile_bg_rect =
            Self::resize_profile_background(texture_size, &self.clip_rect, self.ui_scale);

        // 프로필 아이콘 사각형 영역의 크기를 재조정합니다.
        let texture_size = &self.profile_icon_texture.size;
        self.profile_icon_rect = Self::resize_profile_icon(texture_size, &self.profile_bg_rect);

        // Ui 플레이어 이름 폰트 크기를 재조정합니다.
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(22.0 * self.ui_scale, family);
        self.player_name_text = egui::RichText::new(self.name)
            .font(font_id)
            .color(egui::Color32::DARK_GRAY);

        // 상단 패널 배경화면 영역을 재조정합니다.
        self.pannel_bg_rect = Self::resize_pannel_background(&self.clip_rect, self.ui_scale);
    }

    /// 배경화면을 그립니다.
    fn draw_background(&mut self, ctx: &egui::Context) {
        let source = self.bg_texture;
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                egui::Image::new(source).paint_at(ui, self.bg_rect);
            });
    }

    /// 플레이어 정보를 그립니다.
    fn draw_player_profile(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new(egui::Id::new("Player_Info_Layout"))).show(ctx, |ui| {
            ui.shrink_clip_rect(self.clip_rect);

            // 프로필 배경
            egui::Image::new(self.profile_bg_texture).paint_at(ui, self.profile_bg_rect);

            // 프로필 캐릭터
            egui::Image::new(self.profile_icon_texture).paint_at(ui, self.profile_icon_rect);

            // 이름
            let label = egui::Label::new(self.player_name_text.clone())
                .wrap_mode(egui::TextWrapMode::Truncate)
                .halign(egui::Align::Center)
                .sense(egui::Sense::empty())
                .selectable(false);
            let label_rect = egui::Rect::from_min_max(
                self.profile_bg_rect.center_top()
                    - egui::vec2(self.profile_bg_rect.width() * 0.25, 0.0),
                self.profile_bg_rect.max,
            );
            ui.put(label_rect, label);
        });
    }

    /// 상단 패널을 그립니다.
    fn draw_pannel(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Pannel")).show(ctx, |ui| {
            ui.shrink_clip_rect(self.clip_rect);

            // 배경
            self.draw_pannel_background(ui);
        });
    }

    /// 패널 배경화면을 그립니다.
    fn draw_pannel_background(&self, ui: &mut egui::Ui) {
        const SIZE: f32 = 256.0;
        const LEFT: f32 = 65.0;
        const RIGHT: f32 = 185.0;
        const DECO: f32 = 6.0;

        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(LEFT / SIZE, 1.0));
        let rect = egui::Rect::from_min_max(
            self.pannel_bg_rect.left_top() - egui::vec2(DECO, 0.0) * self.ui_scale,
            self.pannel_bg_rect.left_bottom(),
        );
        egui::Image::new(self.pannel_bg_texture)
            .uv(uv)
            .paint_at(ui, rect);

        let uv =
            egui::Rect::from_min_max(egui::pos2(LEFT / SIZE, 0.0), egui::pos2(RIGHT / SIZE, 1.0));
        let rect = egui::Rect::from_min_max(
            self.pannel_bg_rect.left_top(),
            self.pannel_bg_rect.right_bottom(),
        );
        egui::Image::new(self.pannel_bg_texture)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(egui::pos2(RIGHT / SIZE, 0.0), egui::pos2(1.0, 1.0));
        let rect = egui::Rect::from_min_max(
            self.pannel_bg_rect.right_top(),
            self.pannel_bg_rect.right_bottom() + egui::vec2(DECO, 0.0) * self.ui_scale,
        );
        egui::Image::new(self.pannel_bg_texture)
            .uv(uv)
            .paint_at(ui, rect);
    }
}

impl GameScene for MainLobbyScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_background_texture(device, ui_renderer);
        self.regist_profile_bg_texture(device, ui_renderer);
        self.regist_profile_icon_texture(device, ui_renderer);
        self.regist_pannel_bg_texture(device, ui_renderer);
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

        // 플레이어 정보 그리기
        self.draw_player_profile(ctx);

        // 패널 그리기
        self.draw_pannel(ctx);

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
        self.draw_background(ctx);
    }
}
