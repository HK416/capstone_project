use std::ptr::NonNull;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::GameInput,
    protocol::{PacketType, RawPacket},
};
use mod_render::UiRenderer;
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{CHARACTER_ICON_URIS, NOTOSANS_REGULAR},
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH, TEAM_COLOR},
};

use super::InGameDominationModeScene;

/// 인게임 종합전술시험(점령전)의 현재 게임 진행 상태를 출력하는 게임 장면입니다.
pub struct InGameDominationModeStatusLayer {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,

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
    pub fn new(locale: Locale, prev_scene: NonNull<InGameDominationModeScene>) -> Self {
        Self { locale, prev_scene }
    }
}

impl GameScene for InGameDominationModeStatusLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn should_update_subscene(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, _app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let prev_scene = unsafe { self.prev_scene.as_mut() };
        prev_scene.set_show_status(true);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let prev_scene = unsafe { self.prev_scene.as_mut() };
        prev_scene.set_show_status(false);
    }

    fn on_enter_foreground(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorDisable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_enter_background(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        if packet.packet_type() == PacketType::FinishStage {
            let scene_flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        Some(packet)
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
                    let event = AppEvent::AddGameSceneFlow(scene_flow);
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
        _button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        false
    }

    fn on_mouse_btn_released(
        &mut self,
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

        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 현재 플레이어 데이터를 가져옵니다.
        let prev_scene = unsafe { self.prev_scene.as_mut() };
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

                    // 배경
                    let ui_area = egui::Rect::from_min_max(
                        egui::pos2(beg_x, beg_y),
                        egui::pos2(end_x, end_y),
                    );
                    ui.painter().rect(
                        ui_area,
                        4.0,
                        egui::Color32::from_white_alpha(128),
                        egui::Stroke::new(1.0 * scale, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );

                    // 캐릭터 아이콘 배경
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
                    ui.painter().rect(
                        icon_area,
                        4.0,
                        egui::Color32::WHITE,
                        egui::Stroke::new(2.0 * scale, TEAM_COLOR[data.team as usize]),
                        egui::StrokeKind::Middle,
                    );

                    // 캐릭터 아이콘
                    egui::Image::new(icon).paint_at(ui, icon_area);

                    // 플레이어 체력 배경
                    let percent = data.health_point.percent();
                    let x = beg_x + 88.0 * scale;
                    let y = beg_y + 52.0 * scale;
                    ui.painter().rect(
                        egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * scale, y + 8.0 * scale),
                        ),
                        4.0,
                        egui::Color32::DARK_GRAY,
                        egui::Stroke::new(1.0 * scale, egui::Color32::BLACK),
                        egui::StrokeKind::Middle,
                    );

                    // 플레이어 체력
                    ui.painter().rect(
                        egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * percent * scale, y + 8.0 * scale),
                        ),
                        4.0,
                        egui::Color32::GREEN,
                        egui::Stroke::NONE,
                        egui::StrokeKind::Middle,
                    );

                    // 색상 데이터
                    let fill_color = match data.connected {
                        true => match data.alive {
                            true => egui::Color32::TRANSPARENT,
                            false => egui::Color32::from_black_alpha(96),
                        },
                        false => egui::Color32::from_black_alpha(160),
                    };
                    ui.painter().rect(
                        ui_area,
                        4.0,
                        fill_color,
                        egui::Stroke::new(1.0 * scale, fill_color),
                        egui::StrokeKind::Middle,
                    );

                    if data.connected {
                        // 플레이어 닉네임
                        let font_id = egui::FontId::new(18.0 * scale, main_font_family.clone());
                        let text = egui::RichText::new(data.account.name.to_string())
                            .font(font_id)
                            .color(egui::Color32::BLACK);
                        let x = beg_x + 88.0 * scale;
                        let y = beg_y + 12.0 * scale;
                        let text_area = egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * scale, y + 32.0 * scale),
                        );
                        ui.put(
                            text_area,
                            egui::Label::new(text)
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .sense(egui::Sense::empty()),
                        );
                    } else {
                        let font_id = egui::FontId::new(26.0 * scale, main_font_family.clone());
                        let text = egui::RichText::new("Disconnect")
                            .font(font_id)
                            .color(egui::Color32::WHITE);
                        let x = beg_x + 88.0 * scale;
                        let y = beg_y + 12.0 * scale;
                        let text_area = egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * scale, y + 32.0 * scale),
                        );
                        ui.put(
                            text_area,
                            egui::Label::new(text)
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .sense(egui::Sense::empty()),
                        );
                    }

                    beg_y = end_y + 8.0 * scale;
                }

                let beg_x = 648.0 * scale;
                let end_x = beg_x + 456.0 * scale;
                beg_y = 258.0 * scale;
                for data in red {
                    end_y = beg_y + 64.0 * scale;

                    // 배경
                    let ui_area = egui::Rect::from_min_max(
                        egui::pos2(beg_x, beg_y),
                        egui::pos2(end_x, end_y),
                    );
                    ui.painter().rect(
                        ui_area,
                        4.0,
                        egui::Color32::from_white_alpha(128),
                        egui::Stroke::new(1.0 * scale, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );

                    // 캐릭터 아이콘 배경
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
                    ui.painter().rect(
                        icon_area,
                        4.0,
                        egui::Color32::WHITE,
                        egui::Stroke::new(2.0 * scale, TEAM_COLOR[data.team as usize]),
                        egui::StrokeKind::Middle,
                    );

                    // 캐릭터 아이콘
                    egui::Image::new(icon).paint_at(ui, icon_area);

                    // 플레이어 체력 배경
                    let percent = data.health_point.percent();
                    let x = beg_x + 88.0 * scale;
                    let y = beg_y + 52.0 * scale;
                    ui.painter().rect(
                        egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * scale, y + 8.0 * scale),
                        ),
                        4.0,
                        egui::Color32::DARK_GRAY,
                        egui::Stroke::new(1.0 * scale, egui::Color32::BLACK),
                        egui::StrokeKind::Middle,
                    );

                    // 플레이어 체력
                    ui.painter().rect(
                        egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * percent * scale, y + 8.0 * scale),
                        ),
                        4.0,
                        egui::Color32::GREEN,
                        egui::Stroke::NONE,
                        egui::StrokeKind::Middle,
                    );

                    // 색상 데이터
                    let fill_color = match data.connected {
                        true => match data.alive {
                            true => egui::Color32::TRANSPARENT,
                            false => egui::Color32::from_black_alpha(96),
                        },
                        false => egui::Color32::from_black_alpha(160),
                    };
                    ui.painter().rect(
                        ui_area,
                        4.0,
                        fill_color,
                        egui::Stroke::new(1.0 * scale, fill_color),
                        egui::StrokeKind::Middle,
                    );

                    if data.connected {
                        // 플레이어 닉네임
                        let font_id = egui::FontId::new(18.0 * scale, main_font_family.clone());
                        let text = egui::RichText::new(data.account.name.to_string())
                            .font(font_id)
                            .color(egui::Color32::BLACK);
                        let x = beg_x + 88.0 * scale;
                        let y = beg_y + 12.0 * scale;
                        let text_area = egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * scale, y + 32.0 * scale),
                        );
                        ui.put(
                            text_area,
                            egui::Label::new(text)
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .sense(egui::Sense::empty()),
                        );
                    } else {
                        let font_id = egui::FontId::new(26.0 * scale, main_font_family.clone());
                        let text = egui::RichText::new("Disconnect")
                            .font(font_id)
                            .color(egui::Color32::WHITE);
                        let x = beg_x + 88.0 * scale;
                        let y = beg_y + 12.0 * scale;
                        let text_area = egui::Rect::from_min_max(
                            egui::pos2(x, y),
                            egui::pos2(x + 250.0 * scale, y + 32.0 * scale),
                        );
                        ui.put(
                            text_area,
                            egui::Label::new(text)
                                .wrap_mode(egui::TextWrapMode::Truncate)
                                .sense(egui::Sense::empty()),
                        );
                    }

                    beg_y = end_y + 8.0 * scale;
                }
            });
    }
}

unsafe impl Send for InGameDominationModeStatusLayer {}
