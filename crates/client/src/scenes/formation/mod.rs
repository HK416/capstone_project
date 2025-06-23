mod player;

use std::cmp;

use ahash::{HashMap, HashSet, RandomState};
use mod_app::{
    app::AppHandle,
    etc::{AppEvent, Viewport},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        CharacterKind, GameTier, LoginToken, ProfileIcon, SelectResult, Team, UserId,
        MAX_IN_GAME_PLAYERS, MAX_IN_GAME_TEAM_PLAYERS, NUM_CHARACTERS,
    },
    protocol::{
        CharacterReleaseNotifyPacket, CharacterSelectRequestPacket, CharacterSelectResponsePacket,
        EnterGameFailedPacket, EnterGameFailedResson, FormationDataUpdatePacket,
        InGameDataInitPacket, Packet, PacketType, RawPacket,
    },
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_FORMATION_URI, CHARACTER_IMG_URI, EMBLEM_BG_URI,
        HUD_LAYOUT_URI_02, NOTOSANS_BOLD, NOTOSANS_REGULAR, PROFILE_ICON_URI,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, InGameLoadScene, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR,
        TEAM_COLOR,
    },
    SERVER_TCP_ADDR,
};

pub use self::player::*;

use super::{MessageSceneLayer, BASE_WIDTH};

/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["알림"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const EMPTY_BLUE_TEAM_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["블루 팀 인원이 비어있습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const EMPTY_RED_TEAM_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["레드 팀 인원이 비어있습니다"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const DUPLICATE_ERR_TEXTS: [&'static str; NUM_LOCALE] = ["이미 사용중인 캐릭터입니다"];

/// 애플리케이션 표시 언어에 따른 캐릭터 선택 텍스트
const SELECT_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["캐릭터 선택"];
/// 애플리케이션 표시 언어에 따른 캐릭터 선택 해제 텍스트
const RELEASE_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["선택 해제"];

/// 지연 시간
const DEALY_TIME: f32 = 0.2;

/// 인 게임 장면에 진입하기 전 캐릭터를 편성하는 장면입니다.  
pub struct CharacterFormationScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 캐릭터 편성까지 남은 시간
    remaining_time_sec: f32,

    /// 플레이어 집합
    players: HashMap<UserId, FormationPlayerData>,

    /// Ui 스케일
    ui_scale: f32,
    /// 클립 영역 사각형
    clip_rect: egui::Rect,
    /// 배경화면 텍스처
    bg_texture: egui::load::SizedTexture,
    /// 배경화면 영역
    bg_rect: egui::Rect,

    /// 프로필 배경 텍스처 크기
    profile_bg_texture_size: egui::Vec2,
    /// 프로필 배경 텍스처
    profile_bg_textures: HashMap<GameTier, egui::load::SizedTexture>,
    /// 프로필 아이콘 텍스처
    profile_icon_textures: HashMap<ProfileIcon, egui::load::SizedTexture>,
    /// 블루 팀 프로필 영역
    blue_team_profile_rects: Vec<(egui::Rect, egui::Rect)>,
    /// 레드 팀 프로필 영역
    red_team_profile_rects: Vec<(egui::Rect, egui::Rect)>,

    /// 캐릭터 이미지 텍스처
    character_textures: Vec<egui::load::SizedTexture>,
    /// 캐릭터 이미지 버튼
    character_btn_states: Vec<ButtonState>,

    /// 선택 버튼 상태
    select_btn_state: ButtonState,
    /// 선택 버튼 영역
    select_btn_rect: egui::Rect,
    /// 현재 선택한 캐릭터 종류
    select_character: Option<CharacterKind>,
    /// 최근 전달 받은 선택 결과
    received_select_result: Option<SelectResult>,

    /// 타이머 배경 텍스처
    timer_bg_texture: egui::load::SizedTexture,
    /// 타이머 배경 영역
    timer_bg_rect: egui::Rect,

    /// 지연 시간
    delay_time_sec: f32,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl CharacterFormationScene {
    /// 새로운 게임 장면을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        remaining_time_sec: f32,
        players: HashMap<UserId, FormationPlayerData>,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            remaining_time_sec,
            players,
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            bg_rect: egui::Rect::ZERO,
            profile_bg_texture_size: egui::Vec2::ZERO,
            profile_bg_textures: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
            profile_icon_textures: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
            blue_team_profile_rects: Vec::with_capacity(MAX_IN_GAME_TEAM_PLAYERS),
            red_team_profile_rects: Vec::with_capacity(MAX_IN_GAME_TEAM_PLAYERS),
            character_textures: Vec::with_capacity(NUM_CHARACTERS),
            character_btn_states: vec![ButtonState::Idle; NUM_CHARACTERS],
            select_btn_state: ButtonState::Idle,
            select_character: None,
            select_btn_rect: egui::Rect::ZERO,
            received_select_result: None,
            timer_bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            timer_bg_rect: egui::Rect::ZERO,
            delay_time_sec: 0.0,
            texture_pool,
            texture_view_pool,
        }
    }

    /// 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_background_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 캐릭터 편성 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(BG_FORMATION_URI)
            .expect("BG_Formation texture must be preloaded!");
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

    /// 캐릭터 이미지 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_character_img_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
        character_kind: CharacterKind,
    ) -> egui::load::SizedTexture {
        // 캐릭터 편성 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(CHARACTER_IMG_URI)
            .expect("Character_Img texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: character_kind as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        }
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

    /// 타이머 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_timer_background_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
    ) {
        // 타이머 배경 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_LAYOUT_URI_02)
            .expect("HUD_Layout_02 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 타이머 배경 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.timer_bg_texture = egui::load::SizedTexture {
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

    /// 블루 팀 플레이어의 프로필 영역의 크기를 재조정합니다.
    fn resize_blue_team_profile_rects(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> Vec<(egui::Rect, egui::Rect)> {
        const OFFSET: egui::Vec2 = egui::vec2(250.0, 16.0);
        const WIDTH: f32 = 380.0;
        static_assertions::const_assert!(0.0 <= WIDTH);
        static_assertions::const_assert!(WIDTH <= BASE_WIDTH);

        let ratio = texture_size.x / texture_size.y;
        let width = WIDTH * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);

        let mut rects = Vec::with_capacity(MAX_IN_GAME_TEAM_PLAYERS);
        let mut pos = clip_rect.center_bottom() - egui::vec2(0.0, 168.0 * scale);
        for _ in 0..MAX_IN_GAME_TEAM_PLAYERS {
            let right_bottom = pos + OFFSET * egui::vec2(-1.0, -1.0) * scale;
            let left_top = right_bottom + size * egui::vec2(-1.0, -1.0);
            let profile_rect = egui::Rect::from_two_pos(left_top, right_bottom);

            let right_bottom =
                left_top + size * egui::vec2(1.0, 1.0) + egui::vec2(profile_rect.height(), 0.0);
            let bg_rect = egui::Rect::from_two_pos(left_top, right_bottom);

            rects.push((profile_rect, bg_rect));
            pos = egui::pos2(clip_rect.center().x, left_top.y);
        }

        rects.reverse();
        rects
    }

    /// 레드 팀 플레이어의 프로필 영역의 크기를 재조정합니다.
    fn resize_red_team_profile_rects(
        texture_size: &egui::Vec2,
        clip_rect: &egui::Rect,
        scale: f32,
    ) -> Vec<(egui::Rect, egui::Rect)> {
        const OFFSET: egui::Vec2 = egui::vec2(250.0, 16.0);
        const WIDTH: f32 = 380.0;
        static_assertions::const_assert!(0.0 <= WIDTH);
        static_assertions::const_assert!(WIDTH <= BASE_WIDTH);

        let ratio = texture_size.x / texture_size.y;
        let width = WIDTH * scale;
        let height = width / ratio;
        let size = egui::vec2(width, height);

        let mut rects = Vec::with_capacity(MAX_IN_GAME_TEAM_PLAYERS);
        let mut pos = clip_rect.center_bottom() - egui::vec2(0.0, 168.0 * scale);
        for _ in 0..MAX_IN_GAME_TEAM_PLAYERS {
            let left_bottom = pos + OFFSET * egui::vec2(1.0, -1.0) * scale;
            let right_top = left_bottom + size * egui::vec2(1.0, -1.0);
            let profile_rect = egui::Rect::from_two_pos(right_top, left_bottom);

            let left_bottom =
                right_top + size * egui::vec2(-1.0, 1.0) + egui::vec2(-profile_rect.height(), 0.0);
            let bg_rect = egui::Rect::from_two_pos(right_top, left_bottom);

            rects.push((profile_rect, bg_rect));
            pos = egui::pos2(clip_rect.center().x, right_top.y);
        }

        rects.reverse();
        rects
    }

    /// 프로필 아이콘 영역의 크기를 재조정합니다.
    fn resize_profile_icon_rect(
        texture_size: &egui::Vec2,
        profile_rect: &egui::Rect,
    ) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let height = profile_rect.height() * 0.85;
        let width = height * ratio;
        let margin = egui::vec2(0.0, profile_rect.height() * 0.05);
        let min = profile_rect.min + margin;
        let size = egui::vec2(width, height);
        egui::Rect::from_min_size(min, size)
    }

    /// 캐릭터 이미지의 크기를 재조정합니다.
    fn resize_character_img_rect(
        team: Team,
        texture_size: &egui::Vec2,
        profile_rect: &egui::Rect,
        bg_rect: &egui::Rect,
    ) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let height = profile_rect.height() * 0.85;
        let width = height * ratio;
        let size = egui::vec2(width, height);
        let center = match team {
            Team::Blue => {
                (profile_rect.right_center().to_vec2() + bg_rect.right_center().to_vec2()) * 0.5
            }
            Team::Red => {
                (profile_rect.left_center().to_vec2() + bg_rect.left_center().to_vec2()) * 0.5
            }
        }
        .to_pos2();
        egui::Rect::from_center_size(center, size)
    }

    /// 선택 버튼 영역의 크기를 재조정합니다.
    fn resize_select_btn_rect(clip_rect: &egui::Rect, scale: f32) -> egui::Rect {
        let width = 240.0 * scale;
        let height = 42.0 * scale;
        let size = egui::vec2(width, height);
        let offset = egui::vec2(0.0, 152.0 + 72.0) * scale;
        let center = clip_rect.center_bottom() - (size * egui::vec2(0.0, 0.5) + offset);
        egui::Rect::from_center_size(center, size)
    }

    /// 타이머 배경 영역의 크기를 재조정합니다.
    fn resize_timer_background_rect(clip_rect: &egui::Rect, scale: f32) -> egui::Rect {
        let width = 440.0 * scale;
        let height = 72.0 * scale;
        let size = egui::vec2(width, height);
        let center =
            clip_rect.center_top() + egui::vec2(0.0, 24.0 * scale) + size * egui::vec2(0.0, 0.5);
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

        // 프로필 영역을 재조정합니다.
        let texture_size = &self.profile_bg_texture_size;
        self.blue_team_profile_rects =
            Self::resize_blue_team_profile_rects(texture_size, &self.clip_rect, self.ui_scale);
        self.red_team_profile_rects =
            Self::resize_red_team_profile_rects(texture_size, &self.clip_rect, self.ui_scale);

        // 선택 버튼 영역을 재조정합니다.
        self.select_btn_rect = Self::resize_select_btn_rect(&self.clip_rect, self.ui_scale);

        // 타이머 배경 영역을 재조정합니다.
        self.timer_bg_rect = Self::resize_timer_background_rect(&self.clip_rect, self.ui_scale);
    }

    /// Ui 입력을 처리합니다.
    fn handle_ui_input(&mut self, ctx: &egui::Context, app: &dyn AppHandle) {
        egui::Area::new(egui::Id::new("Handle_Input"))
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);

                // 현재 플레이어가 선택한 캐릭터를 가져옵니다.
                let current = self
                    .players
                    .get(&self.uid)
                    .map(|data| data.character_kind)
                    .flatten();

                // 선택 버튼의 입력을 처리합니다.
                if let Some(select) = self.select_character {
                    let response = ui.allocate_rect(self.select_btn_rect, egui::Sense::all());
                    if response.clicked() && self.delay_time_sec <= 0.0 {
                        self.delay_time_sec = DEALY_TIME;

                        if current.is_some_and(|curr| curr == select) {
                            // 현재 플레이어 선택 캐릭터와 선택한 캐릭터가 동일한 경우 선택 해제 처리
                            let packet = CharacterReleaseNotifyPacket::new(self.uid, self.token);
                            let net = app.net_manager();
                            let socket = net.get(&SERVER_TCP_ADDR).unwrap();
                            socket.push_packet(packet.as_raw());
                        } else {
                            // 현재 플레이어 선택 캐릭터와 선택한 캐릭터가 다른 경우 선택 요청 처리
                            let packet =
                                CharacterSelectRequestPacket::new(self.uid, self.token, select);
                            let net = app.net_manager();
                            let socket = net.get(&SERVER_TCP_ADDR).unwrap();
                            socket.push_packet(packet.as_raw());
                        }

                        self.select_btn_state = ButtonState::Clicked;
                    } else if response.is_pointer_button_down_on() {
                        self.select_btn_state = ButtonState::Pressed;
                    } else if response.hovered() | response.has_focus() {
                        self.select_btn_state = ButtonState::Hovered;
                    } else {
                        self.select_btn_state = ButtonState::Idle;
                    }
                }
            });
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
            });
    }

    fn draw_profile(&mut self, ctx: &egui::Context) {
        const BG_COLOR: egui::Color32 = egui::Color32::from_black_alpha(96);
        const FOCUS_COLOR: egui::Color32 = egui::Color32::from_rgb(242, 201, 76);
        let alpha = egui::Color32::from_white_alpha(192);

        egui::Area::new(egui::Id::new("Profile"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let (mut blue_team, mut red_team): (Vec<_>, Vec<_>) = self
                    .players
                    .values()
                    .partition(|data| data.team() == Team::Blue);
                blue_team.sort_by_key(|data| cmp::Reverse(data.team_index()));
                red_team.sort_by_key(|data| cmp::Reverse(data.team_index()));

                let mut iterator = blue_team.iter();
                for &(profile_rect, bg_rect) in self.blue_team_profile_rects.iter() {
                    match iterator.next() {
                        Some(data) => {
                            // 배경
                            let bg_color = TEAM_COLOR[data.team() as usize] * alpha;
                            let line_color = match data.uid == self.uid {
                                true => FOCUS_COLOR,
                                false => BG_COLOR,
                            } * alpha;
                            ui.painter().rect(
                                bg_rect,
                                12.0 * self.ui_scale,
                                bg_color,
                                egui::Stroke::new(4.0 * self.ui_scale, line_color),
                                egui::StrokeKind::Middle,
                            );

                            // 프로필 배경
                            let source =
                                self.profile_bg_textures.get(&data.tier()).cloned().unwrap();
                            egui::Image::new(source)
                                .sense(egui::Sense::empty())
                                .paint_at(ui, profile_rect);

                            // 프로필 아이콘
                            let source = self
                                .profile_icon_textures
                                .get(&data.profile_icon)
                                .cloned()
                                .unwrap();
                            let icon_rect =
                                Self::resize_profile_icon_rect(&source.size, &profile_rect);
                            egui::Image::new(source)
                                .sense(egui::Sense::empty())
                                .paint_at(ui, icon_rect);

                            // 이름
                            let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
                            let font_id = egui::FontId::new(18.0 * self.ui_scale, family);
                            let text = egui::RichText::new(data.name)
                                .font(font_id)
                                .color(FONT_COLOR);
                            let label = egui::Label::new(text)
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .halign(egui::Align::Center)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            let label_rect = egui::Rect::from_min_max(
                                profile_rect.center_top()
                                    - egui::vec2(profile_rect.width() * 0.25, 0.0),
                                profile_rect.max,
                            );
                            ui.put(label_rect, label);

                            if !data.is_connected() {
                                // 비활성화
                                ui.painter().rect_filled(
                                    bg_rect,
                                    12.0 * self.ui_scale,
                                    egui::Color32::from_black_alpha(192),
                                );

                                // 텍스트
                                let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
                                let font_id = egui::FontId::new(24.0 * self.ui_scale, family);
                                let text = egui::RichText::new("Disconnect")
                                    .font(font_id)
                                    .color(egui::Color32::WHITE);
                                let label = egui::Label::new(text)
                                    .sense(egui::Sense::empty())
                                    .selectable(false);
                                ui.put(bg_rect, label);
                            } else if let Some(kind) = data.character_kind {
                                let texture = self.character_textures[kind as usize];
                                let rect = Self::resize_character_img_rect(
                                    Team::Blue,
                                    &texture.size,
                                    &profile_rect,
                                    &bg_rect,
                                );
                                egui::Image::new(texture)
                                    .sense(egui::Sense::empty())
                                    .paint_at(ui, rect);
                            };
                        }
                        None => {
                            let color = BG_COLOR * alpha;
                            ui.painter().rect(
                                bg_rect,
                                12.0 * self.ui_scale,
                                color,
                                egui::Stroke::new(4.0 * self.ui_scale, color),
                                egui::StrokeKind::Middle,
                            );
                        }
                    };
                }

                let mut iterator = red_team.iter();
                for &(profile_rect, bg_rect) in self.red_team_profile_rects.iter() {
                    match iterator.next() {
                        Some(data) => {
                            // 배경
                            let bg_color = TEAM_COLOR[data.team() as usize] * alpha;
                            let line_color = match data.uid == self.uid {
                                true => FOCUS_COLOR,
                                false => BG_COLOR,
                            } * alpha;
                            ui.painter().rect(
                                bg_rect,
                                12.0 * self.ui_scale,
                                bg_color,
                                egui::Stroke::new(4.0 * self.ui_scale, line_color),
                                egui::StrokeKind::Middle,
                            );

                            // 프로필 배경
                            let source =
                                self.profile_bg_textures.get(&data.tier()).cloned().unwrap();
                            egui::Image::new(source)
                                .sense(egui::Sense::empty())
                                .paint_at(ui, profile_rect);

                            // 프로필 아이콘
                            let source = self
                                .profile_icon_textures
                                .get(&data.profile_icon)
                                .cloned()
                                .unwrap();
                            let icon_rect =
                                Self::resize_profile_icon_rect(&source.size, &profile_rect);
                            egui::Image::new(source)
                                .sense(egui::Sense::empty())
                                .paint_at(ui, icon_rect);

                            // 이름
                            let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
                            let font_id = egui::FontId::new(18.0 * self.ui_scale, family);
                            let text = egui::RichText::new(data.name)
                                .font(font_id)
                                .color(FONT_COLOR);
                            let label = egui::Label::new(text)
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .halign(egui::Align::Center)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            let label_rect = egui::Rect::from_min_max(
                                profile_rect.center_top()
                                    - egui::vec2(profile_rect.width() * 0.25, 0.0),
                                profile_rect.max,
                            );
                            ui.put(label_rect, label);

                            if !data.is_connected() {
                                // 비활성화
                                ui.painter().rect_filled(
                                    bg_rect,
                                    12.0 * self.ui_scale,
                                    egui::Color32::from_black_alpha(192),
                                );

                                // 텍스트
                                let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
                                let font_id = egui::FontId::new(24.0 * self.ui_scale, family);
                                let text = egui::RichText::new("Disconnect")
                                    .font(font_id)
                                    .color(egui::Color32::WHITE);
                                let label = egui::Label::new(text)
                                    .sense(egui::Sense::empty())
                                    .selectable(false);
                                ui.put(bg_rect, label);
                            } else if let Some(kind) = data.character_kind {
                                let texture = self.character_textures[kind as usize];
                                let rect = Self::resize_character_img_rect(
                                    Team::Red,
                                    &texture.size,
                                    &profile_rect,
                                    &bg_rect,
                                );
                                egui::Image::new(texture)
                                    .sense(egui::Sense::empty())
                                    .paint_at(ui, rect);
                            };
                        }
                        None => {
                            let color = BG_COLOR * alpha;
                            ui.painter().rect(
                                bg_rect,
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

    /// 캐릭터를 그립니다.
    fn draw_characters(&mut self, ctx: &egui::Context) {
        const FOCUS_COLOR: egui::Color32 = egui::Color32::from_rgb(242, 201, 76);
        const HEIGHT: f32 = 152.0;
        let pos = self.clip_rect.left_bottom() + egui::vec2(0.0, -HEIGHT * self.ui_scale);
        let size = egui::vec2(self.clip_rect.width(), HEIGHT * self.ui_scale);
        egui::Area::new(egui::Id::new("Character_Select"))
            .fixed_pos(pos)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                ui.set_min_size(size);
                ui.set_max_size(size);
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.set_min_size(size);
                    ui.set_max_size(size);

                    // 캐릭터 수를 세기 위해 임시로 계산
                    let mut val = 0;
                    let mut total_width = 0.0f32;
                    let spacing = 8.0 * self.ui_scale;
                    while let Some(kind) = CharacterKind::new(val) {
                        let texture = self.character_textures[kind as usize];
                        let ratio = texture.size.x / texture.size.y;
                        let height = HEIGHT * 0.8 * self.ui_scale;
                        let width = height * ratio;
                        total_width += width + spacing;
                        val += 1;
                    }
                    if val > 0 {
                        total_width -= spacing; // 마지막 spacing 제외
                    }

                    // 중앙 정렬을 위한 좌측 padding
                    let available_width = ui.available_width();
                    let offset_x = if total_width < available_width {
                        (available_width - total_width) / 2.0
                    } else {
                        0.0
                    };

                    ui.horizontal(|ui| {
                        ui.add_space(offset_x);

                        let mut val = 0;
                        while let Some(kind) = CharacterKind::new(val) {
                            let texture = self.character_textures[kind as usize];
                            let ratio = texture.size.x / texture.size.y;
                            let height = HEIGHT * 0.8 * self.ui_scale;
                            let width = height * ratio;
                            let size = egui::vec2(width, height);

                            let state = &mut self.character_btn_states[kind as usize];
                            let tint = if self.select_character.is_some_and(|select| select == kind)
                            {
                                let min = ui.cursor().min;
                                let rect = egui::Rect::from_min_size(min, size);
                                ui.painter().rect_stroke(
                                    rect,
                                    4.0 * self.ui_scale,
                                    egui::Stroke::new(8.0 * self.ui_scale, FOCUS_COLOR),
                                    egui::StrokeKind::Middle,
                                );

                                NORM_COLOR
                            } else {
                                match state {
                                    ButtonState::Pressed | ButtonState::Clicked => NORM_COLOR,
                                    ButtonState::Hovered => NORM_FOCUS_COLOR,
                                    ButtonState::Idle => NORM_EXP_COLOR,
                                }
                            };
                            let image = egui::Image::new(texture)
                                .tint(tint)
                                .sense(egui::Sense::all())
                                .fit_to_exact_size(size);
                            let response = ui.add(image);
                            *state = if response.clicked() && self.delay_time_sec <= 0.0 {
                                self.delay_time_sec = DEALY_TIME;
                                self.select_character = Some(kind);
                                self.received_select_result = None;
                                ButtonState::Clicked
                            } else if response.is_pointer_button_down_on() {
                                ButtonState::Pressed
                            } else if response.hovered() | response.has_focus() {
                                ButtonState::Hovered
                            } else {
                                ButtonState::Idle
                            };

                            if CharacterKind::new(val + 1).is_some() {
                                ui.add_space(spacing);
                            }
                            val += 1;
                        }
                    });
                });
            });
    }

    /// 선택 버튼을 그립니다.
    fn draw_select_button(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Select_Button"))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                // 현재 플레이어가 선택한 캐릭터를 가져옵니다.
                let current = self
                    .players
                    .get(&self.uid)
                    .map(|data| data.character_kind)
                    .flatten();

                let fill_color = match self.select_btn_state {
                    ButtonState::Idle => NORM_COLOR,
                    ButtonState::Hovered => NORM_FOCUS_COLOR,
                    ButtonState::Pressed | ButtonState::Clicked => NORM_EXP_COLOR,
                };
                ui.painter().rect(
                    self.select_btn_rect,
                    16.0 * self.ui_scale,
                    fill_color,
                    egui::Stroke::new(1.0 * self.ui_scale, egui::Color32::BLACK),
                    egui::StrokeKind::Middle,
                );

                let text = if current
                    .is_some_and(|curr| self.select_character.is_some_and(|select| select == curr))
                {
                    RELEASE_BTN_TEXTS[self.locale as usize]
                } else {
                    SELECT_BTN_TEXTS[self.locale as usize]
                };
                let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                let font_id = egui::FontId::new(22.0 * self.ui_scale, family);
                let text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
                let label = egui::Label::new(text)
                    .sense(egui::Sense::empty())
                    .selectable(false);
                ui.put(self.select_btn_rect, label);
            });
    }

    /// 경고 메시지를 화면에 표시합니다.
    fn draw_warning_message(&mut self, ctx: &egui::Context) {
        if let Some(select_result) = self.received_select_result {
            match select_result {
                SelectResult::Duplicates => {
                    egui::Area::new(egui::Id::new("Warning_Message"))
                        .order(egui::Order::Background)
                        .sense(egui::Sense::empty())
                        .show(ctx, |ui| {
                            ui.shrink_clip_rect(self.clip_rect);

                            let text = DUPLICATE_ERR_TEXTS[self.locale as usize];
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(18.0 * self.ui_scale, family);
                            let text = egui::RichText::new(text)
                                .font(font_id)
                                .color(egui::Color32::DARK_RED);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);

                            let center = self.select_btn_rect.center_bottom()
                                + egui::vec2(0.0, 72.0) * 0.5 * self.ui_scale;
                            let size = self.select_btn_rect.size() * egui::vec2(1.5, 1.0);
                            let rect = egui::Rect::from_center_size(center, size);
                            ui.put(rect, label)
                        });
                }
                _ => {}
            }
        }
    }

    /// 남은 시간을 출력합니다.
    fn draw_remaining_time(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Remaining_Timer"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                self.draw_timer_background(ui);
                self.draw_timer_label(ui);
            });
    }

    /// 타이머 배경화면을 그립니다.
    fn draw_timer_background(&self, ui: &mut egui::Ui) {
        const SIZE: f32 = 256.0;
        const LEFT: f32 = 65.0;
        const RIGHT: f32 = 185.0;
        const DECO: f32 = 36.0;

        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(LEFT / SIZE, 1.0));
        let rect = egui::Rect::from_min_max(
            self.timer_bg_rect.left_top() - egui::vec2(DECO, 0.0) * self.ui_scale,
            self.timer_bg_rect.left_bottom(),
        );
        egui::Image::new(self.timer_bg_texture)
            .sense(egui::Sense::empty())
            .uv(uv)
            .paint_at(ui, rect);

        let uv =
            egui::Rect::from_min_max(egui::pos2(LEFT / SIZE, 0.0), egui::pos2(RIGHT / SIZE, 1.0));
        let rect = egui::Rect::from_min_max(
            self.timer_bg_rect.left_top(),
            self.timer_bg_rect.right_bottom(),
        );
        egui::Image::new(self.timer_bg_texture)
            .sense(egui::Sense::empty())
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(egui::pos2(RIGHT / SIZE, 0.0), egui::pos2(1.0, 1.0));
        let rect = egui::Rect::from_min_max(
            self.timer_bg_rect.right_top(),
            self.timer_bg_rect.right_bottom() + egui::vec2(DECO, 0.0) * self.ui_scale,
        );
        egui::Image::new(self.timer_bg_texture)
            .sense(egui::Sense::empty())
            .uv(uv)
            .paint_at(ui, rect);
    }

    /// 타이머 텍스트를 그립니다.
    fn draw_timer_label(&self, ui: &mut egui::Ui) {
        let text = match self.locale {
            Locale::KOR => format!(
                "남은 편성 시간: {}초",
                self.remaining_time_sec.round() as u32
            ),
        };
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * self.ui_scale, family);
        let text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.put(self.timer_bg_rect, label);
    }
}

impl GameScene for CharacterFormationScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_background_texture(device, ui_renderer);
        self.regist_timer_background_texture(device, ui_renderer);
        let (tier_set, icon_set): (HashSet<_>, HashSet<_>) = self
            .players
            .values()
            .map(|data| (data.tier(), data.profile_icon))
            .unzip();
        let mut val = 0;
        while let Some(character_kind) = CharacterKind::new(val) {
            let texture = self.regist_character_img_texture(device, ui_renderer, character_kind);
            self.character_textures.push(texture);
            val += 1;
        }
        for tier in tier_set {
            self.regist_profile_bg_texture(device, ui_renderer, tier);
        }
        for icon in icon_set {
            self.regist_profile_icon_texture(device, ui_renderer, icon);
        }
        self.resize_ui(window, app);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        ui_renderer.free_texture(&self.bg_texture.id);
        ui_renderer.free_texture(&self.timer_bg_texture.id);
        let iterator = self
            .profile_bg_textures
            .values()
            .chain(self.profile_icon_textures.values())
            .chain(self.character_textures.iter());
        for texture in iterator {
            ui_renderer.free_texture(&texture.id);
        }
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::FormationDataUpdate => {
                let packet = FormationDataUpdatePacket::from_raw(packet);

                // 남은 시간을 갱신합니다.
                const TIME_EPSILON: f32 = 0.5;
                if (self.remaining_time_sec - packet.remaining_time_sec).abs() > TIME_EPSILON {
                    self.remaining_time_sec = packet.remaining_time_sec;
                }

                // 플레이어 데이터를 갱신합니다.
                for pull_data in packet.players.iter() {
                    let data = self.players.get_mut(&pull_data.uid).unwrap();
                    data.set_connected(pull_data.is_connected());
                    data.set_network_state(pull_data.network_state());
                    data.set_permission_state(pull_data.permission());
                    data.character_kind = pull_data.character_kind();
                }
            }
            PacketType::CharacterSelectResponse => {
                let packet = CharacterSelectResponsePacket::from_raw(packet);
                self.received_select_result = Some(packet.result);
            }
            PacketType::EnterGameFailed => {
                let packet = EnterGameFailedPacket::from_raw(packet);

                let i = self.locale as usize;
                let title = ERR_TITLE_TEXTS[i];
                let message = match packet.reason {
                    EnterGameFailedResson::BlueTeamEmpty => EMPTY_BLUE_TEAM_ERR_TEXTS[i],
                    EnterGameFailedResson::RedTeamEmpty => EMPTY_RED_TEAM_ERR_TEXTS[i],
                };
                let scene = MessageSceneLayer::new(self.locale, title, message, None);
                let flow = GameSceneFlow::Change(Box::new(scene));
                let event = AppEvent::AddGameSceneFlow(flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::InGameDataInit => {
                let packet = InGameDataInitPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let scene = InGameLoadScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    packet,
                    &self.texture_pool,
                );
                let flow = GameSceneFlow::Change(Box::new(scene));
                let event = AppEvent::AddGameSceneFlow(flow);
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

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
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

    fn ui_callback(&mut self, _window: &Window, app: &dyn mod_app::app::AppHandle) {
        let ctx = app.egui_ctx();
        self.handle_ui_input(ctx, app);
        self.draw_background(ctx);
        self.draw_profile(ctx);
        self.draw_characters(ctx);
        self.draw_select_button(ctx);
        self.draw_warning_message(ctx);
        self.draw_remaining_time(ctx);
    }
}
