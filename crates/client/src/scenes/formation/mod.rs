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
        CharacterKind, GameTier, LoginToken, ProfileIcon, Team, UserId, MAX_IN_GAME_PLAYERS,
        MAX_IN_GAME_TEAM_PLAYERS, NUM_CHARACTERS,
    },
    protocol::{FormationDataUpdatePacket, Packet, PacketType, RawPacket},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_FORMATION_URI, CHARACTER_IMG_URI, EMBLEM_BG_URI,
        NOTOSANS_BOLD, NOTOSANS_REGULAR, PROFILE_ICON_URI,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
        FONT_COLOR, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR, TEAM_COLOR,
    },
    SERVER_TCP_ADDR,
};

pub use self::player::*;

use super::{MessageSceneLayer, BASE_WIDTH};

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
            select_character: None,
            is_selected: false,
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
            .order(egui::Order::Middle)
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
        const WIDTH: f32 = 1280.0;
        const HEIGHT: f32 = 152.0;
        let pos = self.clip_rect.left_bottom() + egui::vec2(0.0, -HEIGHT * self.ui_scale);
        let size = egui::vec2(WIDTH, HEIGHT) * self.ui_scale;
        egui::Area::new(egui::Id::new("Character_Select"))
            .fixed_pos(pos)
            .default_size(size)
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink(false)
                    .max_width(WIDTH * self.ui_scale)
                    .max_height(HEIGHT * self.ui_scale)
                    .show(ui, |ui| {
                        ui.set_min_size(size);

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
                        let available_width = WIDTH * self.ui_scale;
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
                                let tint = match state {
                                    ButtonState::Pressed | ButtonState::Clicked => {
                                        egui::Color32::from_gray(128)
                                    }
                                    ButtonState::Hovered => egui::Color32::from_gray(192),
                                    ButtonState::Idle => egui::Color32::from_gray(255),
                                };
                                let image = egui::Image::new(texture)
                                    .tint(tint)
                                    .sense(egui::Sense::all())
                                    .fit_to_exact_size(size);
                                let response = ui.add(image);
                                *state = if response.clicked() {
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
}

impl GameScene for CharacterFormationScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_background_texture(device, ui_renderer);
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
                // TODO
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
        let ctx = app.egui_ctx();
        self.draw_background(ctx);
        self.draw_profile(ctx);
        self.draw_characters(ctx);
    }
}
