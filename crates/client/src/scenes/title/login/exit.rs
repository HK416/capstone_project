use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::RawPacket;
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    component::ButtonState,
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, GameLoginModalScene, BASE_WIDTH},
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["안내"];
/// 애플리케이션 표시 언어에 따른 매시지 텍스트
const MESSAGE_TEXTS: [&'static str; NUM_LOCALE] = ["게임을 종료하시겠습니까?"];
/// 애플리케이션 표시 언어에 따른 `예` 버튼 텍스트
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["예"];
/// 애플리케이션 표시 언어에 따른 `아니오` 버튼 텍스트
const CANCEL_TEXTS: [&'static str; NUM_LOCALE] = ["아니오"];

pub struct GameExitModalScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 확인 버튼 상태
    okay_button_state: ButtonState,
    /// 아니오 버튼 상태
    cancal_button_state: ButtonState,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,
}

impl GameExitModalScene {
    /// 새로운 `GameExitModalScene`을 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            okay_button_state: ButtonState::Idle,
            cancal_button_state: ButtonState::Idle,
            delay_time_sec: 0.3,
        }
    }
}

impl GameScene for GameExitModalScene {
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

    fn on_received_packet(&mut self, _: RawPacket, _: &dyn AppHandle) -> Option<RawPacket> {
        None
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
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
        let text = MESSAGE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let message_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `예` 버튼 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `아니오` 버튼 텍스트
        let text = CANCEL_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let cancel_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);

        // `예` 버튼
        let fill = match self.okay_button_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let okay_button = egui::Button::new(okay_text)
            .sense(egui::Sense::all())
            .fill(fill)
            .corner_radius(3.0)
            .min_size((180.0 * scale, 45.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        // `아니오` 버튼
        let fill = match self.cancal_button_state {
            ButtonState::Idle => egui::Color32::WHITE,
            ButtonState::Hovered => egui::Color32::LIGHT_GRAY,
            ButtonState::Pressed | ButtonState::Clicked => egui::Color32::GRAY,
        };
        let cancel_button = egui::Button::new(cancel_text)
            .sense(egui::Sense::all())
            .fill(fill)
            .corner_radius(3.0)
            .min_size((180.0 * scale, 45.0 * scale).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));

        let frame = egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .corner_radius(3.0)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        egui::Modal::new(egui::Id::new("Exit_Onemore"))
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

                    let enable = self.okay_button_state != ButtonState::Clicked
                        && self.cancal_button_state != ButtonState::Clicked;
                    ui.add_enabled_ui(enable, |ui| {
                        egui::Grid::new(egui::Id::new("Button_Grid"))
                            .min_col_width(640.0 * 0.5 * scale)
                            .max_col_width(640.0 * 0.5 * scale)
                            .show(ui, |ui| {
                                ui.set_max_height(45.0 * scale);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // 예 버튼
                                        let response = ui.add(okay_button);
                                        if response.clicked() && self.delay_time_sec <= 0.0 {
                                            self.okay_button_state = ButtonState::Clicked;

                                            // 모든 게임 장면을 제거합니다.
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
                                    },
                                );

                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        // 취소 버튼
                                        let response = ui.add(cancel_button);
                                        if response.clicked() && self.delay_time_sec <= 0.0 {
                                            self.cancal_button_state = ButtonState::Clicked;

                                            // 게임 장면을 전환합니다.
                                            let next_scene =
                                                Box::new(GameLoginModalScene::new(self.locale));
                                            let scene_flow = GameSceneFlow::Change(next_scene);
                                            let event = AppEvent::AddGameSceneFlow(scene_flow);
                                            let event_loop_proxy = app.event_loop_proxy();
                                            event_loop_proxy.send_event(event).unwrap();
                                        } else if response.is_pointer_button_down_on() {
                                            self.cancal_button_state = ButtonState::Pressed;
                                        } else if response.hovered() | response.has_focus() {
                                            self.cancal_button_state = ButtonState::Hovered;
                                        } else {
                                            self.cancal_button_state = ButtonState::Idle;
                                        }
                                    },
                                );
                            });
                    });
                });
                ui.add_space(18.0 * scale);
            });
    }
}
