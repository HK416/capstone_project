use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::GameInput;
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    config::{Locale, UserConfig},
    scenes::BASE_WIDTH,
};

/// 인게임 장면에서 현재 게임 진행 상태를 출력하는 게임 장면입니다.
pub struct InGameStatusLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
}

impl InGameStatusLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }
}

impl GameScene for InGameStatusLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn should_update_subscene(&self) -> bool {
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
        false
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !repeat {
            let config = UserConfig::get();
            if let Some(input) = config.get_keyboard_input(&(code, location)) {
                if input == GameInput::Status {
                    let scene_flow = GameSceneFlow::Pop;
                    let event = AppEvent::SetGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                    return true;
                }
            }
        }

        false
    }

    fn on_cursor_moved(
        &mut self,
        _x: f32,
        _y: f32,
        _dx: f32,
        _dy: f32,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn on_mouse_btn_pressed(
        &mut self,
        _x: f32,
        _y: f32,
        _button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn on_mouse_btn_released(
        &mut self,
        _x: f32,
        _y: f32,
        _button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 대화 상자 속성
        let wnd_width = 960.0 * scale;
        let wnd_height = 480.0 * scale;
        let frame = egui::Frame::new()
            .corner_radius(3.0)
            .fill(egui::Color32::from_black_alpha(194))
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        egui::Modal::new(egui::Id::new("Modal_Window_Layout"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(64))
            .show(app.egui_ctx(), |ui| {
                ui.set_width(wnd_width);
                ui.set_height(wnd_height);
            });
    }
}
