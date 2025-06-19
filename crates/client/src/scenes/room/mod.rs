//! 커스텀 게임 장면과 관련된 코드를 작성합니다.
//!
use ahash::{HashMap, HashSet, RandomState};
use mod_app::{
    app::AppHandle,
    etc::{AppEvent, Viewport},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        CustomRoomPlayerData, GameTier, LoginToken, ProfileIcon, StageKind, UserId, WorldId,
        MAX_IN_GAME_PLAYERS, MAX_IN_GAME_TEAM_PLAYERS, NUM_PROFILE_ICONS, NUM_TIER,
    },
    protocol::{
        Packet, PacketType, RawPacket, RoomDataUpdatePacket, RoomLeaveNotifyPacket,
        RoomReadyRequestPacket, StartFailedReason, StartGameFailedPacket,
    },
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_DECO_URI, BG_MAIN_LOBBY_URI, EMBLEM_BG_URI,
        HUD_CANCEL_ICON_URI, HUD_LAYOUT_URI_00, HUD_LAYOUT_URI_02, IMG_FONT_READY_URI,
        NOTOSANS_BOLD, PROFILE_ICON_URI,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
        FONT_COLOR, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR,
    },
    SERVER_TCP_ADDR,
};

use super::{MessageSceneLayer, BASE_WIDTH, TEAM_COLOR};

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임"];
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
    #[allow(dead_code)]
    world_id: WorldId,
    /// 지형 종류입니다.
    stage_kind: StageKind,
    /// 캐릭터 중복 허용 여부
    allow_duplicates: bool,
    /// 팀 밸런스 불균형 허용 여부
    allow_unbalanced: bool,
    /// 현재 커스텀 게임에 참가한 플레이어 목록입니다.
    players: Vec<CustomRoomPlayerData>,

    /// Ui 스케일
    ui_scale: f32,
    /// 클립 사각형 영역
    clip_rect: egui::Rect,

    /// 배경화면 사각형 영역
    bg_rect: egui::Rect,
    /// 배경화면 텍스처의 식별자입니다.
    bg_texture: egui::load::SizedTexture,

    /// 배경화면 꾸밈 텍스처
    bg_deco_texture: egui::load::SizedTexture,
    /// 왼쪽 꾸밈 영역
    bg_deco_left_rect: egui::Rect,
    /// 오른쪽 꾸밈 영역
    bg_deco_right_rect: egui::Rect,

    /// 프로필 배경 텍스처 크기
    profile_bg_texture_size: egui::Vec2,
    /// 프로필 배경 텍스처
    profile_bg_textures: HashMap<GameTier, egui::load::SizedTexture>,
    /// 프로필 아이콘 텍스처
    profile_icon_textures: HashMap<ProfileIcon, egui::load::SizedTexture>,
    /// 프로필 영역
    profile_rects: Vec<egui::Rect>,

    /// 패널 라벨 텍스트
    pannel_label_text: String,
    /// 패널 배경 텍스처
    pannel_bg_texture: egui::load::SizedTexture,
    /// 패널 배경 영역
    pannel_bg_rect: egui::Rect,

    /// 취소 아이콘 텍스처
    cancel_icon_texture: egui::load::SizedTexture,
    /// 취소 아이콘 영역
    cancel_icon_rect: egui::Rect,
    /// 취소 버튼 상태
    cancel_btn_state: ButtonState,

    /// 준비 이미지 폰트 텍스처
    img_font_ready_texture: egui::load::SizedTexture,

    /// 버튼 텍스처
    button_texture: egui::load::SizedTexture,
    /// 준비 버튼 상태
    ready_btn_state: ButtonState,
    /// 준비 버튼 영역
    ready_btn_rect: egui::Rect,

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
        mut players: Vec<CustomRoomPlayerData>,
    ) -> Self {
        assert_ne!(uid, UserId::NULL, "invalid user identifier");
        assert_ne!(world_id, WorldId::NULL, "invalid world identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        // UID 순서로 정렬합니다.
        players.sort_by_key(|data| data.uid);

        Self {
            locale,
            uid,
            token,
            world_id,
            players,
            stage_kind,
            allow_duplicates,
            allow_unbalanced,
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            bg_rect: egui::Rect::ZERO,
            bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            bg_deco_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            bg_deco_left_rect: egui::Rect::ZERO,
            bg_deco_right_rect: egui::Rect::ZERO,
            profile_bg_texture_size: egui::Vec2::splat(1.0),
            profile_bg_textures: HashMap::with_capacity_and_hasher(NUM_TIER, RandomState::new()),
            profile_icon_textures: HashMap::with_capacity_and_hasher(
                NUM_PROFILE_ICONS,
                RandomState::new(),
            ),
            profile_rects: Vec::with_capacity(MAX_IN_GAME_PLAYERS),
            pannel_label_text: format!("{} - {}", &HEAD_TEXTS[locale as usize], &world_id),
            pannel_bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            pannel_bg_rect: egui::Rect::ZERO,
            cancel_icon_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            cancel_icon_rect: egui::Rect::ZERO,
            cancel_btn_state: ButtonState::Idle,
            img_font_ready_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            button_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            ready_btn_state: ButtonState::Idle,
            ready_btn_rect: egui::Rect::ZERO,
            texture_pool,
            texture_view_pool,
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

    /// 배경화면 꾸밈 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_background_deco_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
    ) {
        // 배경화면 꾸밈 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(BG_DECO_URI)
            .expect("BG_Deco_00 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 배경화면 꾸밈 텍스처의 텍스처 뷰를 생성합니다.
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

    /// 프로필 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_bg_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
        tier: GameTier,
    ) {
        // 프로필 배경 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(EMBLEM_BG_URI)
            .expect("Emblem_BG texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);
        self.profile_bg_texture_size = texture_size;

        // 프로필 배경 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: tier as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_bg_textures.insert(
            tier,
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 프로필 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_icon_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
        icon: ProfileIcon,
    ) {
        // 프로필 아이콘 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(PROFILE_ICON_URI)
            .expect("Profile_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 프로필 아이콘 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: icon as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_icon_textures.insert(
            icon,
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 취소 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_cancel_icon_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 취소 아이콘 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_CANCEL_ICON_URI)
            .expect("HUD_Cancel_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 취소 아이콘 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.cancel_icon_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 준비 폰트 이미지 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_img_font_ready_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
    ) {
        // 준비 폰트 이미지 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(IMG_FONT_READY_URI)
            .expect("ImgFont_Ready texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 준비 폰트 이미지 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.img_font_ready_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 버튼 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_button_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 버튼 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_LAYOUT_URI_00)
            .expect("HUD_Layout_00 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 준비 폰트 이미지 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.button_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// Ui 크기를 재조정합니다.
    fn resize_ui(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형 영역을 재조정합니다.
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        (self.clip_rect, self.ui_scale) = Self::resize_clip_rect(viewport, scale_factor);

        // 배경 사각형 영역을 재조정합니다.
        let texture_size = &self.bg_texture.size;
        self.bg_rect = Self::resize_background_rect(texture_size, &self.clip_rect);

        // 배경 꾸밈 사각형 영역을 재조정합니다.
        let texture_size = &self.bg_deco_texture.size;
        (self.bg_deco_left_rect, self.bg_deco_right_rect) =
            Self::resize_background_deco_rect(texture_size, &self.clip_rect, self.ui_scale);

        // 패널 배경 영역을 재조정합니다.
        self.pannel_bg_rect = Self::resize_pannel_bg_rect(&self.clip_rect, self.ui_scale);

        // 프로필 영역을 재조정합니다.
        let texture_size = &self.profile_bg_texture_size;
        self.profile_rects =
            Self::resize_profile_rects(texture_size, &self.clip_rect, self.ui_scale);

        // 취소 아이콘 영역을 재조정합니다.
        let texture_size = &self.cancel_icon_texture.size;
        self.cancel_icon_rect =
            Self::resize_cancel_icon_rect(texture_size, &self.clip_rect, self.ui_scale);

        // 준비 버튼 영역을 재조정합니다.
        let texture_size = &self.button_texture.size;
        self.ready_btn_rect =
            Self::resize_ready_btn_rect(texture_size, &self.clip_rect, self.ui_scale);
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
    fn resize_background_rect(texture_size: &egui::Vec2, clip_rect: &egui::Rect) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let width = clip_rect.width();
        let height = width / ratio;
        let size = egui::vec2(width, height);
        egui::Rect::from_center_size(clip_rect.center(), size)
    }

    /// 배경 꾸밈 사각형 영역의 크기를 재조정합니다.
    fn resize_background_deco_rect(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> (egui::Rect, egui::Rect) {
        let ratio = texture_size.x / texture_size.y;
        let width = 320.0 * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);

        let center = clip_rect.left_center() + egui::vec2(0.5 * width, 0.0);
        let left = egui::Rect::from_center_size(center, size);

        let center = clip_rect.right_center() - egui::vec2(0.5 * width, 0.0);
        let right = egui::Rect::from_center_size(center, size);

        (left, right)
    }

    /// 프로필 영역의 크기를 재조정합니다.
    fn resize_profile_rects(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> Vec<egui::Rect> {
        const OFFSET: egui::Vec2 = egui::vec2(240.0, 16.0);
        const WIDTH: f32 = 360.0;
        static_assertions::const_assert!(0.0 <= WIDTH);
        static_assertions::const_assert!(WIDTH <= BASE_WIDTH);

        let ratio = texture_size.x / texture_size.y;
        let width = WIDTH * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);

        let mut rects = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        let mut pos = clip_rect.center_bottom() - egui::vec2(0.0, 168.0 * scale);
        for _ in 0..MAX_IN_GAME_TEAM_PLAYERS {
            // Left
            let left_bottom = pos + OFFSET * egui::vec2(1.0, -1.0) * scale;
            let right_top = left_bottom + size * egui::vec2(1.0, -1.0);
            rects.push(egui::Rect::from_two_pos(left_bottom, right_top));

            // Right
            let right_bottom = pos + OFFSET * egui::vec2(-1.0, -1.0) * scale;
            let left_top = right_bottom + size * egui::vec2(-1.0, -1.0);
            rects.push(egui::Rect::from_two_pos(left_top, right_bottom));

            pos = ((left_top.to_vec2() + right_top.to_vec2()) * 0.5).to_pos2();
        }

        rects.reverse();
        rects
    }

    /// 패널 배경 영역의 크기를 재조정합니다.
    fn resize_pannel_bg_rect(clip_rect: &egui::Rect, scale: f32) -> egui::Rect {
        const MARGIN: egui::Vec2 = egui::vec2(24.0, 16.0);
        const WIDTH: f32 = 480.0;
        const HEIGHT: f32 = 72.0;
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

    /// 취소 아이콘 영역의 크기를 재조정합니다.
    fn resize_cancel_icon_rect(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let width = 24.0 * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);
        let offset = size * 1.5;
        let min = clip_rect.min + offset;
        egui::Rect::from_min_size(min, size)
    }

    /// 준비 이미지 폰트의 크기를 재조정합니다.
    fn resize_ready_font_rect(texture_size: &egui::Vec2, profile_rect: &egui::Rect) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let width = profile_rect.width() * 0.3;
        let height = width / ratio;
        let center = profile_rect.min;
        let size = egui::vec2(width, height);
        egui::Rect::from_center_size(center, size)
    }

    /// 준비 버튼 영역의 크기를 재조정합니다.
    fn resize_ready_btn_rect(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let width = 240.0 * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);
        let center = clip_rect.center_bottom()
            - egui::vec2(0.0, height * 0.5)
            - egui::vec2(0.0, 28.0 * scale);
        egui::Rect::from_center_size(center, size)
    }

    /// Ui 입력을 처리합니다.
    fn handle_ui_input(&mut self, ctx: &egui::Context, app: &dyn AppHandle) {
        egui::Area::new(egui::Id::new("Handle_Input"))
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                // 취소 버튼 입력을 처리합니다.
                let response = ui.allocate_rect(self.cancel_icon_rect, egui::Sense::all());
                if response.clicked() {
                    // 패킷을 전송합니다.
                    let packet = RoomLeaveNotifyPacket::new(self.uid, self.token);
                    let net = app.net_manager();
                    let socket = net.get(&SERVER_TCP_ADDR).unwrap();
                    socket.push_packet(packet.as_raw());

                    // 장면을 전환합니다.
                    let flow = GameSceneFlow::Pop;
                    let event = AppEvent::AddGameSceneFlow(flow);
                    let event_loop_event = app.event_loop_proxy();
                    event_loop_event.send_event(event).unwrap();

                    self.cancel_btn_state = ButtonState::Clicked;
                } else if response.is_pointer_button_down_on() {
                    self.cancel_btn_state = ButtonState::Pressed;
                } else if response.hovered() | response.has_focus() {
                    self.cancel_btn_state = ButtonState::Hovered;
                } else {
                    self.cancel_btn_state = ButtonState::Idle;
                }

                // 준비 버튼 입력을 처리합니다.
                let response = ui.allocate_rect(self.ready_btn_rect, egui::Sense::all());
                if response.clicked() {
                    // 패킷을 전송합니다.
                    let i = self
                        .players
                        .binary_search_by_key(&self.uid, |data| data.uid)
                        .unwrap();
                    let ready_to_play = self.players[i].is_ready_to_play();
                    let packet = RoomReadyRequestPacket::new(self.uid, self.token, !ready_to_play);
                    let net = app.net_manager();
                    let socket = net.get(&SERVER_TCP_ADDR).unwrap();
                    socket.push_packet(packet.as_raw());

                    self.ready_btn_state = ButtonState::Clicked;
                } else if response.is_pointer_button_down_on() {
                    self.ready_btn_state = ButtonState::Pressed;
                } else if response.hovered() | response.has_focus() {
                    self.ready_btn_state = ButtonState::Hovered;
                } else {
                    self.ready_btn_state = ButtonState::Idle;
                }
            });
    }

    /// 배경을 그립니다.
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
                    .paint_at(ui, self.bg_deco_left_rect);
                egui::Image::new(self.bg_deco_texture)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.bg_deco_right_rect);
            });
    }

    /// 프로필을 그립니다.
    fn draw_profile(&mut self, ctx: &egui::Context) {
        const BG_COLOR: egui::Color32 = egui::Color32::from_black_alpha(96);
        const FOCUS_COLOR: egui::Color32 = egui::Color32::from_rgb(242, 201, 76);
        let alpha = egui::Color32::from_white_alpha(192);

        egui::Area::new(egui::Id::new("Profile"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let mut iterator = self.players.iter();
                for &rect in self.profile_rects.iter() {
                    match iterator.next() {
                        Some(data) => {
                            let bg_color = TEAM_COLOR[data.team() as usize] * alpha;
                            let line_color = match data.uid == self.uid {
                                true => FOCUS_COLOR,
                                false => BG_COLOR,
                            } * alpha;
                            ui.painter().rect(
                                rect,
                                12.0 * self.ui_scale,
                                bg_color,
                                egui::Stroke::new(4.0 * self.ui_scale, line_color),
                                egui::StrokeKind::Middle,
                            );

                            let source =
                                self.profile_bg_textures.get(&data.tier()).cloned().unwrap();
                            egui::Image::new(source)
                                .sense(egui::Sense::empty())
                                .paint_at(ui, rect);

                            if data.is_ready_to_play() {
                                let rect = Self::resize_ready_font_rect(
                                    &self.img_font_ready_texture.size,
                                    &rect,
                                );
                                egui::Image::new(self.img_font_ready_texture)
                                    .sense(egui::Sense::empty())
                                    .paint_at(ui, rect);
                            }
                        }
                        None => {
                            let color = BG_COLOR * alpha;
                            ui.painter().rect(
                                rect,
                                12.0 * self.ui_scale,
                                color,
                                egui::Stroke::new(4.0 * self.ui_scale, color),
                                egui::StrokeKind::Middle,
                            );
                        }
                    };
                }
            });
    }

    /// 패널을 그립니다.
    fn draw_pannel(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Pannel"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                self.draw_pannel_background(ui);
                self.draw_pannel_label(ui);
            });
    }

    /// 패널 배경화면을 그립니다.
    fn draw_pannel_background(&self, ui: &mut egui::Ui) {
        const SIZE: f32 = 256.0;
        const LEFT: f32 = 65.0;
        const RIGHT: f32 = 185.0;
        const DECO: f32 = 36.0;

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

    /// 패널 라벨을 그립니다.
    fn draw_pannel_label(&self, ui: &mut egui::Ui) {
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * self.ui_scale, family);
        let text = egui::RichText::new(&self.pannel_label_text)
            .font(font_id)
            .color(FONT_COLOR);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);

        ui.put(self.pannel_bg_rect, label);
    }

    /// 준비 버튼을 그립니다.
    fn draw_ready_button(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Ready_Button"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let tint = match self.ready_btn_state {
                    ButtonState::Idle => NORM_COLOR,
                    ButtonState::Hovered => NORM_FOCUS_COLOR,
                    ButtonState::Clicked | ButtonState::Pressed => NORM_EXP_COLOR,
                };

                // 준비 버튼
                egui::Image::new(self.button_texture)
                    .tint(tint)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.ready_btn_rect);
            });
    }

    /// 취소 버튼을 그립니다.
    fn draw_cancel_button(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Exit_Button"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let center = self.cancel_icon_rect.center();
                let radius = self.cancel_icon_rect.size().max_elem();
                let (bg_color, line_color) = match self.cancel_btn_state {
                    ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
                    ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
                    ButtonState::Clicked | ButtonState::Pressed => {
                        (NORM_EXP_COLOR, egui::Color32::BLACK)
                    }
                };

                // 취소 아이콘 배경
                ui.painter().circle(
                    center,
                    radius,
                    bg_color,
                    egui::Stroke::new(1.0 * self.ui_scale, line_color),
                );

                // 취소 아이콘
                egui::Image::new(self.cancel_icon_texture)
                    .tint(bg_color)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, self.cancel_icon_rect);
            });
    }
}

impl GameScene for CustomGameRoomScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_background_texture(device, ui_renderer);
        self.regist_background_deco_texture(device, ui_renderer);
        let (tier_set, icon_set): (HashSet<_>, HashSet<_>) = self
            .players
            .iter()
            .map(|data| (data.tier(), data.profile_icon))
            .unzip();
        for tier in tier_set {
            self.regist_profile_bg_texture(device, ui_renderer, tier);
        }
        for icon in icon_set {
            self.regist_profile_icon_texture(device, ui_renderer, icon);
        }
        self.regist_pannel_bg_texture(device, ui_renderer);
        self.regist_cancel_icon_texture(device, ui_renderer);
        self.regist_img_font_ready_texture(device, ui_renderer);
        self.regist_button_texture(device, ui_renderer);
        self.resize_ui(window, app);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        ui_renderer.free_texture(&self.bg_texture.id);
        ui_renderer.free_texture(&self.bg_deco_texture.id);
        let iterator = self
            .profile_bg_textures
            .values()
            .chain(self.profile_icon_textures.values());
        for texture in iterator {
            ui_renderer.free_texture(&texture.id);
        }
        ui_renderer.free_texture(&self.pannel_bg_texture.id);
        ui_renderer.free_texture(&self.cancel_icon_texture.id);
        ui_renderer.free_texture(&self.button_texture.id);
        ui_renderer.free_texture(&self.img_font_ready_texture.id);
    }

    fn on_resume(&mut self, _window: &Window, _app: &dyn AppHandle) {}

    fn on_pause(&mut self, _window: &Window, _app: &dyn AppHandle) {}

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::RoomDataUpdate => {
                let packet = RoomDataUpdatePacket::from_raw(packet);
                self.stage_kind = packet.stage_kind();
                self.allow_duplicates = packet.allow_duplicates();
                self.allow_unbalanced = packet.allow_unbalanced();
                self.players = packet.players;
                self.players.sort_by_key(|data| data.uid);
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

    fn ui_callback(&mut self, _window: &Window, app: &dyn AppHandle) {
        let ctx = app.egui_ctx();
        self.handle_ui_input(ctx, app);
        self.draw_cancel_button(ctx);
        self.draw_pannel(ctx);
        self.draw_profile(ctx);
        self.draw_ready_button(ctx);
        self.draw_background(ctx);
    }
}
