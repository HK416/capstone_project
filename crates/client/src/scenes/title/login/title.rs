use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        TexturePool, TextureViewPool, BG_LOGIN_TITLE_0_URI, BG_LOGIN_TITLE_1_URI,
        BG_LOGIN_TITLE_2_URI, BG_LOGIN_TITLE_3_URI, BG_LOGIN_TITLE_4_URI, BG_LOGIN_TITLE_5_URI,
        NOTOSANS_BOLD,
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
    /// 키 눌림 여부
    is_pressed: bool,

    /// 게임 배경화면 텍스처의 텍스처 식별자입니다.
    bg_texture_ids: Vec<egui::load::SizedTexture>,

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
            is_pressed: false,
            bg_texture_ids: Vec::with_capacity(6),
            texture_pool,
            texture_view_pool,
        }
    }

    /// 폰트 색상을 가져옵니다.
    fn get_font_color(&self) -> egui::Color32 {
        use core::f32::consts::PI;
        let s = (self.elapsed_time_sec % FONT_APPEAR_CYCLE) / FONT_APPEAR_CYCLE;
        let c = (s * PI).sin();
        egui::Color32::from_black_alpha((255.0 * c) as u8)
    }
}

impl GameScene for GameLoginTitleScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        for uri in BG_TEXTURE_URI {
            // 로그인 배경화면 텍스처를 가져옵니다.
            let texture = self
                .texture_pool
                .remove(uri)
                .expect("BG_Login_Title_* texture must be preloaded!");
            let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

            // 로그인 배경화면 텍스처의 텍스처 뷰를 생성합니다.
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
            self.bg_texture_ids.push(egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            });
        }
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

    fn on_mouse_btn_pressed(
        &mut self,
        _button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        self.is_pressed = true;
        true
    }

    fn on_keyboard_pressed(
        &mut self,
        _code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        _repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        self.is_pressed = true;
        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 게임 장면 경과 시간을 갱신합니다.
        self.elapsed_time_sec = (self.elapsed_time_sec + elapsed_time_sec) % MAX_SCENE_DURATION;

        // 다음 게임 장면으로 전환합니다.
        if self.is_pressed {
            let next_scene = Box::new(GameLoginModalScene::new(self.locale));
            let scene_flow = GameSceneFlow::Push(next_scene);
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let head_font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let head_font_color = self.get_font_color();

        // 텍스트
        let i = self.locale as usize;
        let text = HEAD_TEXTS[i];
        let head_text = egui::RichText::new(text)
            .font(head_font_id)
            .color(head_font_color);

        egui::Area::new(egui::Id::new("Layout_Enter"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -128.0 * scale])
            .show(app.egui_ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(head_text);
                })
            });

        let index = (self.elapsed_time_sec / CUT_SWITCH_CYCLE).floor() as usize;
        let source = self.bg_texture_ids[index];
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

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                egui::Image::new(source).paint_at(ui, rect);
            });
    }
}
