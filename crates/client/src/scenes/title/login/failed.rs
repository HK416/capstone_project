use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::{LoginFailedReason, RawPacket, NUM_LOGIN_FAILED_REASONS};
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{TexturePool, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, GameLoginModalScene, BASE_WIDTH},
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["로그인 실패"];
/// 애플리케이션 표시 언어에 따른 `확인` 버튼 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];
/// 애플리케이션 표시 언어에 따른 실패 사유 텍스트입니다.
const MESSAGE_TEXTS: [[&'static str; NUM_LOGIN_FAILED_REASONS]; NUM_LOCALE] = [[
    "로그인 정보가 잘못되었습니다!",
    "서버에 접근할 수 없습니다!",
]];

/// 로그인 실패 메시지를 출력하는 게임 장면입니다.
pub struct LoginFailedModalScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// 로그인 실패 사유
    reason: LoginFailedReason,

    /// `확인` 버튼 상태입니다.
    okay_button_state: ButtonState,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl LoginFailedModalScene {
    /// 새로운 `LoginFailedModalScene`을 생성합니다.
    pub fn new(locale: Locale, reason: LoginFailedReason, texture_pool: TexturePool) -> Self {
        Self {
            locale,
            reason,
            okay_button_state: ButtonState::Idle,
            delay_time_sec: 0.3,
            texture_pool,
        }
    }
}

impl GameScene for LoginFailedModalScene {
    fn transparents(&self) -> bool {
        true
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
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, _app: &dyn AppHandle) -> Option<RawPacket> {
        Some(packet)
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
            if code == KeyCode::Enter {
                // 게임 장면을 전환합니다.
                let next_scene = Box::new(GameLoginModalScene::new(
                    self.locale,
                    self.texture_pool.clone(),
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
        }
        return false;
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _: &Window, _app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);
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

        // 타이틀 텍스트
        let text = TITLE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 메시지 텍스트
        let text = MESSAGE_TEXTS[locale][self.reason as usize];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let message_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 확인 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // 확인 버튼
        let btn_width = 180.0 * scale;
        let btn_height = btn_width * 0.25;
        let fill = match self.okay_button_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let okay_button = egui::Button::new(okay_text)
            .fill(fill)
            .sense(egui::Sense::all())
            .min_size((btn_width, btn_height).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Message"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96))
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(640.0 * scale);
                ui.set_max_width(640.0 * scale);

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.label(title_text);
                    ui.separator();

                    ui.add_space(8.0 * scale);
                    ui.label(message_text);
                    ui.add_space(16.0 * scale);

                    let enable = self.okay_button_state != ButtonState::Clicked;
                    ui.add_enabled_ui(enable, |ui| {
                        let response = ui.add(okay_button);
                        if response.clicked() && self.delay_time_sec <= 0.0 {
                            self.okay_button_state = ButtonState::Clicked;

                            // 게임 장면을 전환합니다.
                            let next_scene = Box::new(GameLoginModalScene::new(
                                self.locale,
                                self.texture_pool.clone(),
                            ));
                            let scene_flow = GameSceneFlow::Change(next_scene);
                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                            let event_loop_proxy = app.event_loop_proxy();
                            event_loop_proxy.send_event(event).unwrap();
                        } else if response.is_pointer_button_down_on() {
                            self.okay_button_state = ButtonState::Pressed;
                        } else if response.hovered() || response.has_focus() {
                            self.okay_button_state = ButtonState::Hovered;
                        } else {
                            self.okay_button_state = ButtonState::Idle;
                        }
                    });
                    ui.add_space(18.0 * scale);
                });
            });
    }
}
