mod enter;
mod exit;
mod join;
mod layer;
mod option;

use std::{num::NonZeroU32, sync::Arc, time::Instant};

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, Viewport},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{GameTier, LoginToken, ProfileIcon, UserId, UserName},
    protocol::{JoinFailedReason, JoinRoomFailedPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_DECO_URI, BG_MAIN_LOBBY_URI, EMBLEM_BG_URI,
        HUD_EXIT_ICON_URI, HUD_LAYOUT_URI_01, HUD_LAYOUT_URI_02, HUD_OPTION_ICON_URI,
        NOTOSANS_BOLD, NOTOSANS_REGULAR, PROFILE_ICON_URI,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        lobby::option::LobbyCommonOptionModalLayer, FatalErrorSceneLayer, MessageSceneLayer,
        ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS, FONT_COLOR,
    }, SERVER_TCP_ADDR,
};

pub use self::{enter::*, exit::*, join::*, layer::*};

use super::BASE_WIDTH;

/// 애플리케이션 표시 언어에 따른 `커스텀 게임 생성` 버튼 텍스트입니다.
const CREATE_GAME_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["게임 생성"];
/// 애플리케이션 표시 언어에 따른 `커스텀 게임 참가` 버튼 텍스트입니다.
const JOIN_GAME_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["게임 참가"];

/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 타이틀 텍스트입니다.
const MSG_MODAL_TEXTS: [&'static str; NUM_LOCALE] = ["알림"];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_NOT_FOUND_TEXTS: [&'static str; NUM_LOCALE] =
    ["해당 커스텀 게임 대기실이 존재하지 않습니다."];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_FULL_CAPACITY_TEXTS: [&'static str; NUM_LOCALE] =
    ["해당 커스텀 게임 대기실 인원이 가득찼습니다."];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_IN_PROGRASS_TEXTS: [&'static str; NUM_LOCALE] = ["이미 게임이 진행 중 입니다."];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_LIMITS_TEXTS: [&'static str; NUM_LOCALE] = ["게임 월드 생성에 실패했습니다."];
/// 애플리케이션 표시 언어에 따른 `모달 대화상자` 메시지 텍스트입니다.
const ERR_BANNED_TEXTS: [&'static str; NUM_LOCALE] = ["게임 관리자에 의해 차단되었습니다."];

/// 게임의 메인 로비 화면입니다.
pub struct MainLobbyScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 사용자 이름
    name: UserName,
    /// 사용자 게임 티어
    tier: GameTier,
    /// 프로필 아이콘
    profile_icon: ProfileIcon,
    /// 현재 클라이언트의 로그인 토큰
    token: LoginToken,

    /// Ui 스케일
    ui_scale: f32,
    /// 클립 영역 사각형
    clip_rect: egui::Rect,
    /// 배경화면 텍스처
    bg_texture: egui::load::SizedTexture,
    /// 배경화면 레이아웃 영역
    bg_rect: egui::Rect,

    /// 배경화면 꾸밈 텍스처
    bg_deco_texture: egui::load::SizedTexture,
    /// 배경화면 꾸밈 레이아웃 영역
    bg_deco_rect: egui::Rect,

    /// 프로필 배경 텍스처
    profile_bg_texture: egui::load::SizedTexture,
    /// 프로필 정보 레이아웃 영역
    profile_bg_rect: egui::Rect,

    /// 프로필 아이콘 텍스처
    profile_icon_texture: egui::load::SizedTexture,
    /// 프로필 아이콘 영역
    profile_icon_rect: egui::Rect,
    /// 플레이어 이름 텍스트
    player_name_text: egui::RichText,

    /// 상단 패널의 배경 텍스처
    pannel_bg_texture: egui::load::SizedTexture,
    /// 상단 패널의 레이아웃 영역
    pannel_bg_rect: egui::Rect,

    /// 종료 아이콘 텍스처
    exit_icon_texture: egui::load::SizedTexture,
    /// 종료 버튼 레이아웃 영역
    exit_btn_rect: egui::Rect,
    /// 종료 버튼 상태
    exit_btn_state: ButtonState,

    /// 옵션 아이콘 텍스처
    option_icon_texture: egui::load::SizedTexture,
    /// 옵션 버튼 레이아웃 영역
    option_btn_rect: egui::Rect,
    /// 옵션 버튼 상태
    option_btn_state: ButtonState,

    /// 게임 버튼 텍스처
    game_button_texture: egui::load::SizedTexture,

    /// 매칭 버튼 영역
    matching_btn_rect: egui::Rect,
    /// 매칭 버튼 상태
    matching_btn_state: ButtonState,

    /// 커스텀 게임 생성 버튼 영역
    create_btn_rect: egui::Rect,
    /// 커스텀 게임 생성 버튼 상태
    create_btn_state: ButtonState,

    /// 커스텀 게임 참가 버튼 영역
    join_btn_rect: egui::Rect,
    /// 커스텀 게임 참가 버튼 상태
    join_btn_state: ButtonState,

    /// 입력 지연 시간
    delay_time_sec: f32,

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
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            bg_rect: egui::Rect::ZERO,
            bg_deco_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            bg_deco_rect: egui::Rect::ZERO,
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
            exit_icon_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            exit_btn_rect: egui::Rect::ZERO,
            exit_btn_state: ButtonState::Idle,
            option_icon_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            option_btn_rect: egui::Rect::ZERO,
            option_btn_state: ButtonState::Idle,
            game_button_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            matching_btn_rect: egui::Rect::ZERO,
            matching_btn_state: ButtonState::Idle,
            create_btn_rect: egui::Rect::ZERO,
            create_btn_state: ButtonState::Idle,
            join_btn_rect: egui::Rect::ZERO,
            join_btn_state: ButtonState::Idle,
            delay_time_sec: 0.0,
            texture_pool,
            texture_view_pool: TextureViewPool::new(),
        }
    }

    /// 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_textures(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        self.regist_background_texture(device, ui_renderer);
        self.regist_background_deco_texture(device, ui_renderer);
        self.regist_profile_bg_texture(device, ui_renderer);
        self.regist_profile_icon_texture(device, ui_renderer);
        self.regist_pannel_bg_texture(device, ui_renderer);
        self.regist_exit_icon_texture(device, ui_renderer);
        self.regist_option_icon_texture(device, ui_renderer);
        self.regist_game_button_texture(device, ui_renderer);
    }

    /// Ui 렌더러에 등록된 텍스처를 해제합니다.
    fn unregist_textures(&mut self, ui_renderer: &mut UiRenderer) {
        ui_renderer.free_texture(&self.bg_texture.id);
        ui_renderer.free_texture(&self.bg_deco_texture.id);
        ui_renderer.free_texture(&self.profile_bg_texture.id);
        ui_renderer.free_texture(&self.profile_icon_texture.id);
        ui_renderer.free_texture(&self.pannel_bg_texture.id);
        ui_renderer.free_texture(&self.exit_icon_texture.id);
        ui_renderer.free_texture(&self.option_icon_texture.id);
        ui_renderer.free_texture(&self.game_button_texture.id);
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

    /// 배경 꾸밈 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_background_deco_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
    ) {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(BG_DECO_URI)
            .expect("BG_Deco_00 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.bg_deco_texture = egui::load::SizedTexture {
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

    /// 종료 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_exit_icon_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 패널 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_EXIT_ICON_URI)
            .expect("HUD_Exit_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.exit_icon_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 옵션 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_option_icon_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 패널 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_OPTION_ICON_URI)
            .expect("HUD_Exit_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.option_icon_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 게임 버튼 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_game_button_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 패널 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_LAYOUT_URI_01)
            .expect("HUD_Layout_01 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.game_button_texture = egui::load::SizedTexture {
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

    /// 배경 꾸밈 사각형 영역의 크기를 재조정합니다.
    fn resize_background_deco(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let width = 320.0 * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);
        let min = clip_rect.right_center() - egui::vec2(1.0, 0.5) * size;
        egui::Rect::from_min_size(min, size)
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

    /// 패널의 아이콘 크기를 재조정합니다.
    fn resize_pannel_icon(
        texture_size: &egui::Vec2,
        pannel_bg_rect: &egui::Rect,
        num_blocks: NonZeroU32,
        i: u32,
    ) -> egui::Rect {
        let block_width = pannel_bg_rect.width() / (num_blocks.get() as f32);
        let half_block_width = 0.5 * block_width;

        let ratio = texture_size.x / texture_size.y;
        let height = pannel_bg_rect.height() * 0.6;
        let width = height * ratio;
        let size = egui::vec2(width, height);
        let offset = egui::vec2(block_width * (i as f32) + half_block_width, 0.0);
        let center = pannel_bg_rect.left_center() + offset;
        egui::Rect::from_center_size(center, size)
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

        // 배경 꾸밈 사각형 영역의 크기를 재조정합니다.
        let texture_size = &self.bg_deco_texture.size;
        self.bg_deco_rect =
            Self::resize_background_deco(texture_size, &self.clip_rect, self.ui_scale);

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
            .color(FONT_COLOR);

        // 상단 패널 배경화면 영역을 재조정합니다.
        self.pannel_bg_rect = Self::resize_pannel_background(&self.clip_rect, self.ui_scale);
        // Safety: 주어지는 정수는 0이 아님
        let num_blocks = unsafe { NonZeroU32::new_unchecked(2) };

        // 패널의 아이콘 영역을 재조정합니다.
        let texture_size = &self.exit_icon_texture.size;
        self.option_btn_rect =
            Self::resize_pannel_icon(texture_size, &self.pannel_bg_rect, num_blocks, 0);
        self.exit_btn_rect =
            Self::resize_pannel_icon(texture_size, &self.pannel_bg_rect, num_blocks, 1);

        // 매칭 버튼의 영역을 재조정합니다.
        let texture_size = self.game_button_texture.size;
        let ratio = texture_size.x / texture_size.y;
        let width = 260.0 * self.ui_scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);
        let max = self.clip_rect.max - egui::Vec2::splat(8.0 * self.ui_scale);
        let min = max - size;
        self.matching_btn_rect = egui::Rect::from_min_max(min, max);

        // 커스텀 게임 참가 버튼의 영역을 재조정합니다.
        let width = 260.0 * self.ui_scale;
        let height = width / ratio;
        let max = egui::pos2(max.x, min.y - 5.0 * self.ui_scale);
        let min = egui::pos2(max.x - width, max.y - height);
        self.join_btn_rect = egui::Rect::from_min_max(min, max);

        // 커스텀 게임 생성 버튼의 영역을 재조정합니다.
        let max = egui::pos2(max.x, min.y - 5.0 * self.ui_scale);
        let min = egui::pos2(max.x - width, max.y - height);
        self.create_btn_rect = egui::Rect::from_min_max(min, max);
    }

    /// 배경화면을 그립니다.
    fn draw_background(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                egui::Image::new(self.bg_texture)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.bg_rect);
                egui::Image::new(self.bg_deco_texture)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.bg_deco_rect);
            });
    }

    /// 플레이어 정보를 그립니다.
    fn draw_player_profile(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new(egui::Id::new("Player_Info_Layout")))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);

                // 프로필 배경
                egui::Image::new(self.profile_bg_texture)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.profile_bg_rect);

                // 프로필 캐릭터
                egui::Image::new(self.profile_icon_texture)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.profile_icon_rect);

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
    fn draw_pannel(&mut self, ctx: &egui::Context, _app: &dyn AppHandle) {
        egui::Area::new(egui::Id::new("Pannel"))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);

                // 배경
                self.draw_pannel_background(ui);

                // 옵션 아이콘
                self.draw_pannel_icon(
                    ui,
                    self.option_btn_state,
                    self.option_icon_texture,
                    self.option_btn_rect,
                );

                // 종료 아이콘
                self.draw_pannel_icon(
                    ui,
                    self.exit_btn_state,
                    self.exit_icon_texture,
                    self.exit_btn_rect,
                );
            });
    }

    /// 패널 배경화면을 그립니다.
    fn draw_pannel_background(&self, ui: &mut egui::Ui) {
        const SIZE: f32 = 256.0;
        const LEFT: f32 = 65.0;
        const RIGHT: f32 = 185.0;
        const DECO: f32 = 24.0;

        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(LEFT / SIZE, 1.0));
        let rect = egui::Rect::from_min_max(
            self.pannel_bg_rect.left_top() - egui::vec2(DECO, 0.0) * self.ui_scale,
            self.pannel_bg_rect.left_bottom(),
        );
        egui::Image::new(self.pannel_bg_texture)
            .sense(egui::Sense::empty())
            .uv(uv)
            .paint_at(ui, rect);

        let uv =
            egui::Rect::from_min_max(egui::pos2(LEFT / SIZE, 0.0), egui::pos2(RIGHT / SIZE, 1.0));
        let rect = egui::Rect::from_min_max(
            self.pannel_bg_rect.left_top(),
            self.pannel_bg_rect.right_bottom(),
        );
        egui::Image::new(self.pannel_bg_texture)
            .sense(egui::Sense::empty())
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(egui::pos2(RIGHT / SIZE, 0.0), egui::pos2(1.0, 1.0));
        let rect = egui::Rect::from_min_max(
            self.pannel_bg_rect.right_top(),
            self.pannel_bg_rect.right_bottom() + egui::vec2(DECO, 0.0) * self.ui_scale,
        );
        egui::Image::new(self.pannel_bg_texture)
            .sense(egui::Sense::empty())
            .uv(uv)
            .paint_at(ui, rect);
    }

    /// 패널 아이콘을 그립니다.
    fn draw_pannel_icon(
        &self,
        ui: &mut egui::Ui,
        state: ButtonState,
        source: egui::load::SizedTexture,
        rect: egui::Rect,
    ) {
        let tint = match state {
            ButtonState::Clicked | ButtonState::Pressed => egui::Color32::from_gray(96),
            ButtonState::Hovered => egui::Color32::from_gray(128),
            ButtonState::Idle => egui::Color32::from_gray(169),
        };
        egui::Image::new(source)
            .sense(egui::Sense::empty())
            .tint(tint)
            .paint_at(ui, rect);
    }

    /// Ui 입력을 처리합니다.
    fn handle_ui_inputs(&mut self, ctx: &egui::Context, app: &dyn AppHandle) {
        egui::Area::new(egui::Id::new("Control"))
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);

                // 옵션 아이콘의 이벤트를 처리합니다.
                let response = ui.allocate_rect(self.option_btn_rect, egui::Sense::all());
                self.option_btn_state = if response.clicked() && self.delay_time_sec <= 0.0 {
                    // 게임 장면을 전환합니다.
                    let scene =
                        LobbyCommonOptionModalLayer::new(self.locale, 0, Arc::new(Queue::new()));
                    let flow = GameSceneFlow::Push(Box::new(scene));
                    let event = AppEvent::AddGameSceneFlow(flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();

                    ButtonState::Clicked
                } else if response.is_pointer_button_down_on() {
                    ButtonState::Pressed
                } else if response.hovered() | response.has_focus() {
                    ButtonState::Hovered
                } else {
                    ButtonState::Idle
                };

                // 종료 아이콘의 이벤트를 처리합니다.
                let response = ui.allocate_rect(self.exit_btn_rect, egui::Sense::all());
                self.exit_btn_state = if response.clicked() && self.delay_time_sec <= 0.0 {
                    // 게임 장면을 전환합니다.
                    let next_scene = Box::new(MainLobbyExitModalScene::new(
                        self.locale,
                        self.texture_pool.clone(),
                        self.texture_view_pool.clone(),
                    ));
                    let scene_flow = GameSceneFlow::Push(next_scene);
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();

                    ButtonState::Clicked
                } else if response.is_pointer_button_down_on() {
                    ButtonState::Pressed
                } else if response.hovered() | response.has_focus() {
                    ButtonState::Hovered
                } else {
                    ButtonState::Idle
                };

                // 게임 생성 버튼의 이벤트를 처리합니다.
                let response = ui.allocate_rect(self.create_btn_rect, egui::Sense::all());
                self.create_btn_state = if response.clicked() && self.delay_time_sec <= 0.0 {
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

                    ButtonState::Clicked
                } else if response.is_pointer_button_down_on() {
                    ButtonState::Pressed
                } else if response.hovered() | response.has_focus() {
                    ButtonState::Hovered
                } else {
                    ButtonState::Idle
                };

                // 게임 참가 버튼의 이벤트를 처리합니다.
                let response = ui.allocate_rect(self.join_btn_rect, egui::Sense::all());
                self.join_btn_state = if response.clicked() && self.delay_time_sec <= 0.0 {
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

                    ButtonState::Clicked
                } else if response.is_pointer_button_down_on() {
                    ButtonState::Pressed
                } else if response.hovered() | response.has_focus() {
                    ButtonState::Hovered
                } else {
                    ButtonState::Idle
                };

                // 게임 매칭 버튼의 이벤트를 처리합니다.
                let response = ui.allocate_rect(self.matching_btn_rect, egui::Sense::all());
                self.matching_btn_state = if response.clicked() && self.delay_time_sec <= 0.0 {
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

                    ButtonState::Clicked
                } else if response.is_pointer_button_down_on() {
                    ButtonState::Pressed
                } else if response.hovered() | response.has_focus() {
                    ButtonState::Hovered
                } else {
                    ButtonState::Idle
                };
            });
    }

    /// 매칭을 시작하는 버튼을 그립니다.
    fn draw_matching_button(&mut self, ctx: &egui::Context, _app: &dyn AppHandle) {
        // let locale = self.locale as usize;
        // let text = CREATE_GAME_BTN_TEXTS[locale];
        // let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        // let font_id = egui::FontId::new(32.0 * self.ui_scale, family);
        // let text = egui::RichText::new(text)
        //     .font(font_id)
        //     .color(egui::Color32::BLACK);
        // let label = egui::Label::new(text).sense(egui::Sense::empty()).selectable(false);
        let tint = match self.matching_btn_state {
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::from_gray(212),
            ButtonState::Hovered => egui::Color32::from_gray(232),
            ButtonState::Idle => egui::Color32::WHITE,
        };

        egui::Area::new(egui::Id::new("Matching_Button"))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                egui::Image::new(self.game_button_texture)
                    .sense(egui::Sense::empty())
                    .tint(tint)
                    .paint_at(ui, self.matching_btn_rect);
                // ui.put(self.matching_btn_rect, label);
            });
    }

    /// 커스텀 게임을 생성하는 버튼을 그립니다.
    fn draw_room_create_button(&mut self, ctx: &egui::Context, _app: &dyn AppHandle) {
        let locale = self.locale as usize;
        let text = CREATE_GAME_BTN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * self.ui_scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        let tint = match self.create_btn_state {
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::from_gray(212),
            ButtonState::Hovered => egui::Color32::from_gray(232),
            ButtonState::Idle => egui::Color32::WHITE,
        };

        egui::Area::new(egui::Id::new("Create_Button"))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                egui::Image::new(self.game_button_texture)
                    .sense(egui::Sense::empty())
                    .tint(tint)
                    .paint_at(ui, self.create_btn_rect);
                ui.put(self.create_btn_rect, label);
            });
    }

    /// 커스텀 게임을 참여하는 버튼을 그립니다.
    fn draw_join_room_button(&mut self, ctx: &egui::Context, _app: &dyn AppHandle) {
        let locale = self.locale as usize;
        let text = JOIN_GAME_BTN_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * self.ui_scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        let tint = match self.join_btn_state {
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::from_gray(212),
            ButtonState::Hovered => egui::Color32::from_gray(232),
            ButtonState::Idle => egui::Color32::WHITE,
        };

        egui::Area::new(egui::Id::new("Join_Button"))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                egui::Image::new(self.game_button_texture)
                    .sense(egui::Sense::empty())
                    .tint(tint)
                    .paint_at(ui, self.join_btn_rect);
                ui.put(self.join_btn_rect, label);
            });
    }
}

impl GameScene for MainLobbyScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_textures(device, ui_renderer);
        self.resize_ui(window, app);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        self.unregist_textures(ui_renderer);
    }

    fn on_resume(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = 0.0
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

    fn on_received_packet(
        &mut self,
        _: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::JoinRoomFailed => {
                // 패킷을 생성합니다
                let packet = JoinRoomFailedPacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let i = self.locale as usize;
                let next_scene = Box::new(MessageSceneLayer::new(
                    self.locale,
                    MSG_MODAL_TEXTS[i],
                    match packet.reason {
                        JoinFailedReason::NotFound => ERR_NOT_FOUND_TEXTS[i],
                        JoinFailedReason::FullCapacity => ERR_FULL_CAPACITY_TEXTS[i],
                        JoinFailedReason::InProgress => ERR_IN_PROGRASS_TEXTS[i],
                        JoinFailedReason::CreationLimited => ERR_LIMITS_TEXTS[i],
                        JoinFailedReason::Banned => ERR_BANNED_TEXTS[i],
                    },
                    None,
                ));
                let scene_flow = GameSceneFlow::Push(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
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

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !repeat && self.delay_time_sec <= 0.0 {
            match code {
                KeyCode::Escape => {
                    // 게임 장면을 전환합니다.
                    let next_scene = Box::new(MainLobbyExitModalScene::new(
                        self.locale,
                        self.texture_pool.clone(),
                        self.texture_view_pool.clone(),
                    ));
                    let scene_flow = GameSceneFlow::Push(next_scene);
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
                _ => {}
            }
        }

        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
    }

    fn ui_callback(&mut self, _window: &Window, app: &dyn AppHandle) {
        let ctx = app.egui_ctx();

        // 입력 처리
        self.handle_ui_inputs(ctx, app);

        // 플레이어 정보 그리기
        self.draw_player_profile(ctx);

        // 패널 그리기
        self.draw_pannel(ctx, app);

        // 버튼 그리기
        self.draw_room_create_button(ctx, app);
        self.draw_join_room_button(ctx, app);
        self.draw_matching_button(ctx, app);

        // 배경화면
        self.draw_background(ctx);
    }
}
