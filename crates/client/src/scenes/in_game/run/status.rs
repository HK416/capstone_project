use std::ptr::NonNull;

use ahash::HashMap;
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{CharacterKind, GameInput, Team, UserId};
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::CHARACTER_ICON_URIS,
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

use super::{InGameDominationModeScene, TEAM_COLOR};

/// 인게임 종합전술시험(점령전)의 현재 게임 진행 상태를 출력하는 게임 장면입니다.
pub struct InGameDominationModeStatusLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 플레이어 식별자입니다.
    user_id: UserId,

    /// 이전 게임 장면입니다.
    ///
    /// # Safe
    /// InGameDominationModeScene에서 SceneFlow::Push로 호출된 경우
    /// InGameDominationModeScene의 주소 값이 유효하고,
    /// 모든 GameScene은 메인 스레드에서 호출되므로
    /// 안전하게 사용할 수 있습니다.
    ///
    prev_scene: NonNull<InGameDominationModeScene>,
}

impl InGameDominationModeStatusLayer {
    /// 새로운 게임 장면 레이어를 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        prev_scene: NonNull<InGameDominationModeScene>,
    ) -> Self {
        Self {
            locale,
            user_id,
            prev_scene,
        }
    }
}

impl GameScene for InGameDominationModeStatusLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn should_update_subscene(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, _app: &dyn AppHandle) {
        let prev_scene = unsafe { self.prev_scene.as_mut() };
        prev_scene.set_show_status(true);
    }

    fn on_exit(&mut self, _window: Option<&Window>, _app: &dyn AppHandle) {
        let prev_scene = unsafe { self.prev_scene.as_mut() };
        prev_scene.set_show_status(false);
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
        let event = AppEvent::SetGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
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
        let prev_scene = unsafe { self.prev_scene.as_mut() };
        let (width, _): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 현재 플레이어 데이터를 가져옵니다.
        let (blue, red) = prev_scene.get_player_data();

        // 대화 상자 속성
        // 기준 가로 길이: 960
        // 기준 세로 길이: 480
        let wnd_width = 960.0 * scale;
        let wnd_height = 480.0 * scale;
        let frame = egui::Frame::new()
            .corner_radius(16.0)
            .fill(egui::Color32::from_black_alpha(194))
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        // 플레이어 아이콘 배경 속성
        // 기준 가로 길이: 456
        // 기준 세로 길이: 64
        // 간격: 8

        egui::Modal::new(egui::Id::new("Modal_Window_Layout"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(64))
            .show(app.egui_ctx(), |ui| {
                ui.set_width(wnd_width);
                ui.set_height(wnd_height);

                let beg_x = 176.0 * scale;
                let end_x = beg_x + 456.0 * scale;
                let mut beg_y = 258.0 * scale;
                let mut end_y;
                for data in blue {
                    end_y = beg_y + 64.0 * scale;

                    ui.painter().rect(
                        egui::Rect::from_min_max(
                            egui::pos2(beg_x, beg_y),
                            egui::pos2(end_x, end_y),
                        ),
                        4.0,
                        egui::Color32::from_white_alpha(128),
                        egui::Stroke::new(1.0 * scale, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );

                    let i = data.character_kind as usize;
                    let icon = prev_scene
                        .get_ui_texture(CHARACTER_ICON_URIS[i])
                        .expect("the Character Icon must exist!");
                    let ratio = icon.size.x / icon.size.y;
                    let x = beg_x + 2.0 * scale;
                    let y = beg_y + 2.0 * scale;
                    let height = 60.0 * scale;
                    let width = height * ratio;
                    let icon_area = egui::Rect::from_min_max(
                        egui::pos2(x, y),
                        egui::pos2(x + width, y + height),
                    );
                    egui::Image::new(icon).paint_at(ui, icon_area);

                    beg_y = end_y + 8.0 * scale;
                }

                let beg_x = 648.0 * scale;
                let end_x = beg_x + 456.0 * scale;
                beg_y = 258.0 * scale;
                for data in red {
                    end_y = beg_y + 64.0 * scale;

                    ui.painter().rect(
                        egui::Rect::from_min_max(
                            egui::pos2(beg_x, beg_y),
                            egui::pos2(end_x, end_y),
                        ),
                        4.0,
                        egui::Color32::from_white_alpha(128),
                        egui::Stroke::new(1.0 * scale, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );

                    let i = data.character_kind as usize;
                    let icon = prev_scene
                        .get_ui_texture(CHARACTER_ICON_URIS[i])
                        .expect("the Character Icon must exist!");
                    let ratio = icon.size.x / icon.size.y;
                    let x = beg_x + 2.0 * scale;
                    let y = beg_y + 2.0 * scale;
                    let height = 60.0 * scale;
                    let width = height * ratio;
                    let icon_area = egui::Rect::from_min_max(
                        egui::pos2(x, y),
                        egui::pos2(x + width, y + height),
                    );
                    egui::Image::new(icon).paint_at(ui, icon_area);

                    beg_y = end_y + 8.0 * scale;
                }
            });
    }
}

unsafe impl Send for InGameDominationModeStatusLayer {}
