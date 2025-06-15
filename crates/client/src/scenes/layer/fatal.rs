use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::RawPacket;
use mod_render::UiRenderer;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{BASE_WIDTH, FONT_COLOR, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR},
};

/// 애플리케이션 표시 언어에 따른 `확인` 버튼 텍스트
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

/// 오류 메시지를 출력하는 게임 장면입니다.
pub struct FatalErrorSceneLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

    /// 모달 대화상자의 타이틀 텍스트입니다.
    title: String,
    /// 모달 대화상자의 메시지 텍스트입니다.
    message: String,

    /// `확인` 버튼 상태입니다.
    okay_button_state: ButtonState,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,
}

impl FatalErrorSceneLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new<T, M>(locale: Locale, title: T, message: M) -> Self
    where
        T: Into<String>,
        M: Into<String>,
    {
        Self {
            locale,
            title: title.into(),
            message: message.into(),
            okay_button_state: ButtonState::Idle,
            delay_time_sec: 0.3,
        }
    }
}

impl GameScene for FatalErrorSceneLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_enter_foreground(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(
        &mut self,
        _packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Option<RawPacket> {
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
            if code == KeyCode::Enter {
                self.okay_button_state = ButtonState::Clicked;

                let scene_flow = GameSceneFlow::Clear;
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
        }

        true
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
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title_text = egui::RichText::new(&self.title)
            .font(font_id)
            .color(FONT_COLOR);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 메시지 텍스트
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let message_text = egui::RichText::new(&self.message)
            .font(font_id)
            .color(FONT_COLOR);
        let message_label = egui::Label::new(message_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 확인 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // 확인 버튼
        let btn_width = 180.0 * scale;
        let btn_height = btn_width * 0.25;
        let (bg_color, line_color) = match self.okay_button_state {
            ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
            ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
            ButtonState::Pressed | ButtonState::Clicked => (NORM_EXP_COLOR, egui::Color32::BLACK),
        };
        let okay_button = egui::Button::new(okay_text)
            .fill(bg_color)
            .sense(egui::Sense::all())
            .corner_radius(5.0 * scale)
            .min_size((btn_width, btn_height).into())
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        let frame = egui::Frame::new()
            .corner_radius(20.0 * scale)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Fatal"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96))
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(640.0 * scale);
                ui.set_max_width(640.0 * scale);

                ui.vertical_centered(|ui| {
                    ui.add_space(8.0 * scale);
                    ui.add(title_label);
                    ui.separator();

                    ui.add_space(8.0 * scale);
                    ui.add(message_label);
                    ui.add_space(16.0 * scale);

                    let enable = self.okay_button_state != ButtonState::Clicked;
                    ui.add_enabled_ui(enable, |ui| {
                        let response = ui.add(okay_button);
                        if response.clicked() && self.delay_time_sec <= 0.0 {
                            self.okay_button_state = ButtonState::Clicked;

                            let scene_flow = GameSceneFlow::Clear;
                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                            let event_loop_proxy = app.event_loop_proxy();
                            event_loop_proxy.send_event(event).unwrap();
                        } else if response.is_pointer_button_down_on() {
                            self.okay_button_state = ButtonState::Pressed;
                        } else if response.hovered() | response.has_focus() {
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
