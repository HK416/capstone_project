use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_GROWTH_EFFECT_LABEL_URI, BG_LOGIN_TITLE_0_URI,
        BG_LOGIN_TITLE_1_URI, BG_LOGIN_TITLE_2_URI, BG_LOGIN_TITLE_3_URI, BG_LOGIN_TITLE_4_URI,
        BG_LOGIN_TITLE_5_URI, NOTOSANS_BOLD,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

use super::GameLoginModalScene;

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["아무 키나 눌러 게임을 시작"];
/// 게임 장면 경과 시간의 최대 지속 시간입니다.
const MAX_SCENE_DURATION: f32 = 15.0;
/// 게임 장면 배경화면 전환 주기입니다.
const CUT_SWITCH_CYCLE: f32 = 2.5;
/// 폰트 알파 값의 주기입니다.
const FONT_APPEAR_CYCLE: f32 = 4.0;
/// 로그인 배경화면 텍스처의 `Uri`입니다.
const BG_TEXTURE_URI: [&'static str; 6] = [
    BG_LOGIN_TITLE_0_URI,
    BG_LOGIN_TITLE_1_URI,
    BG_LOGIN_TITLE_2_URI,
    BG_LOGIN_TITLE_3_URI,
    BG_LOGIN_TITLE_4_URI,
    BG_LOGIN_TITLE_5_URI,
];

/// 게임 로그인 타이틀 화면을 표시하는 장면입니다.
pub struct GameLoginTitleScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,
    /// 라벨의 표시 여부
    visible_label: bool,
    /// 로그인 모달 진입 여부
    pressed_any_keys: bool,

    /// 게임 배경화면 텍스처의 텍스처 식별자입니다.
    bg_textures: Vec<egui::load::SizedTexture>,
    /// 게임 라벨 배경 텍스처의 텍스처 식별자입니다.
    bg_label_texture: egui::load::SizedTexture,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl GameLoginTitleScene {
    /// 새로운 `GameLoginTitleScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
    ) -> Self {
        Self {
            locale,
            elapsed_time_sec: 0.0,
            visible_label: true,
            pressed_any_keys: false,
            bg_textures: Vec::with_capacity(6),
            bg_label_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            delay_time_sec: 0.3,
            texture_pool,
            texture_view_pool,
        }
    }

    /// `uri`에 해당하는 텍스처를 `Ui`렌더러에 등록합니다.
    ///
    /// # Panics
    /// 텍스처 풀 객체에 주어진 `uri`에 해당하는 텍스처가 없는 경우 [`panic!`]을 호출합니다.
    ///
    fn register_texture(
        &self,
        uri: &str,
        device: &wgpu::Device,
        texture_filter: wgpu::FilterMode,
        ui_renderer: &mut UiRenderer,
    ) -> egui::load::SizedTexture {
        // 텍스처 풀 객체에서 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(uri)
            .expect(&format!("{} texture must be preloaded!", &uri));
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id = ui_renderer.register_native_texture(device, &texture, texture_filter);

        egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        }
    }

    /// 알파 값을 반환합니다.
    fn get_alpha_value(&self) -> u8 {
        use core::f32::consts::PI;
        let s = (self.elapsed_time_sec % FONT_APPEAR_CYCLE) / FONT_APPEAR_CYCLE;
        let c = 0.5 * (2.0 * s * PI).sin() + 0.5;
        (c * 255.0) as u8
    }
}

impl GameScene for GameLoginTitleScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        let texture_filter = wgpu::FilterMode::Linear;

        // 라벨 배경 텍스처를 Ui 렌더러에 등록합니다.
        self.bg_label_texture = self.register_texture(
            BG_GROWTH_EFFECT_LABEL_URI,
            device,
            texture_filter,
            ui_renderer,
        );

        // 로그인 배경 화면 텍스처를 Ui 렌더러에 등록합니다.
        for uri in BG_TEXTURE_URI {
            self.bg_textures
                .push(self.register_texture(uri, device, texture_filter, ui_renderer));
        }
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        // egui 렌더러에 등록된 텍스처를 제거합니다.
        ui_renderer.free_texture(&self.bg_label_texture.id);
        for bg_texture in self.bg_textures.iter() {
            ui_renderer.free_texture(&bg_texture.id);
        }
    }

    fn on_pause(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.visible_label = false;
    }

    fn on_resume(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.visible_label = true;
        self.pressed_any_keys = false;
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

    fn on_keyboard_pressed(
        &mut self,
        _code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if !repeat && self.delay_time_sec <= 0.0 {
            self.pressed_any_keys = true;
        }
        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 경과 시간을 갱신합니다.
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
        self.elapsed_time_sec = (self.elapsed_time_sec + elapsed_time_sec) % MAX_SCENE_DURATION;

        // 다음 게임 장면으로 전환합니다.
        if self.pressed_any_keys {
            let next_scene = Box::new(GameLoginModalScene::new(
                self.locale,
                self.texture_pool.clone(),
            ));
            let scene_flow = GameSceneFlow::Push(next_scene);
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
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

        // 텍스트
        let alpha = self.get_alpha_value();
        let text = HEAD_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let enter_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::from_black_alpha(alpha));
        let enter_label = egui::Label::new(enter_text)
            .halign(egui::Align::Center)
            .sense(egui::Sense::empty());

        // 라벨 배경 텍스처
        let hud_label_source = self.bg_label_texture;
        let image_ratio = self.bg_label_texture.size.x / self.bg_label_texture.size.y;
        let image_width = 1280.0 * scale;
        let image_height = image_width / image_ratio * 0.5;
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(
                clip_rect.center().x - image_width * 0.5,
                clip_rect.max.y - 192.0 * scale,
            ),
            egui::pos2(
                clip_rect.center().x + image_width * 0.5,
                clip_rect.max.y - 192.0 * scale + image_height,
            ),
        );

        // 배경 이미지 텍스처
        let index = (self.elapsed_time_sec / CUT_SWITCH_CYCLE).floor() as usize;
        let bg_source = self.bg_textures[index];
        let ratio = bg_source.size.x / bg_source.size.y;
        let center = clip_rect.center();
        let image_width = 1280.0 * scale;
        let image_height = image_width / ratio;
        let rect = egui::Rect {
            min: center - 0.5 * egui::vec2(image_width, image_height),
            max: center + 0.5 * egui::vec2(image_width, image_height),
        };

        egui::CentralPanel::default().show(app.egui_ctx(), |ui| {
            ui.shrink_clip_rect(clip_rect);
            ui.set_min_size(egui::vec2(1280.0, 720.0) * scale);
            ui.set_max_size(egui::vec2(1280.0, 720.0) * scale);

            ui.add_enabled_ui(!self.pressed_any_keys, |ui| {
                egui::Image::new(bg_source).paint_at(ui, rect);
                let entire_respones = ui.allocate_rect(clip_rect, egui::Sense::CLICK);
                if entire_respones.clicked() && self.delay_time_sec <= 0.0 {
                    self.pressed_any_keys = true;
                }

                if self.visible_label {
                    egui::Image::new(hud_label_source)
                        .sense(egui::Sense::empty())
                        .bg_fill(egui::Color32::from_black_alpha(128))
                        .paint_at(ui, label_rect);

                    ui.vertical_centered(|ui| {
                        ui.put(label_rect, enter_label);
                    });
                }
            });
        });
    }
}
