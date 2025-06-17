use std::sync::Arc;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::protocol::{PacketType, RawPacket};
use mod_parallelism::collections::Queue;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR, USER_CONFIG},
    component::ButtonState,
    config::{Locale, UserConfig},
    scenes::{
        FatalErrorSceneLayer, MessageSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS,
        ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, NORM_COLOR, NORM_EXP_COLOR,
        NORM_FOCUS_COLOR, POSI_COLOR, POSI_FOCUS_COLOR,
    },
};

use super::*;

/// 애플리케이션 표시 언어에 따른 언어 설정 텍스트입니다.
const LOCALE_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["언어"];

/// 로비 장면에서 일반 설정 모달 레이어입니다.
pub struct LobbyCommonOptionModalLayer {
    /// 초기 애플리케이션 언어
    prev_locale: Locale,
    /// 현재 애플리케이션 언어
    locale: Locale,

    /// 지연 시간
    delay_time_sec: f32,

    /// 확인 버튼 상태
    exit_btn_state: ButtonState,
    /// 저장 버튼 상태
    save_btn_state: ButtonState,

    /// 남은 작업의 수
    num_remaining_tasks: usize,
    /// 작업 결과 목록
    task_results: Arc<Queue<TaskResult>>,
}

impl LobbyCommonOptionModalLayer {
    /// 새로운 `CommonOptionModalLayer`를 생성합니다.
    pub fn new(
        locale: Locale,
        num_remaining_tasks: usize,
        task_results: Arc<Queue<TaskResult>>,
    ) -> Self {
        Self {
            prev_locale: locale,
            locale,
            delay_time_sec: 0.3,
            exit_btn_state: ButtonState::Idle,
            save_btn_state: ButtonState::Idle,
            num_remaining_tasks,
            task_results,
        }
    }

    /// 메뉴를 그립니다.
    fn draw_menu(&mut self, ui: &mut egui::Ui, i: usize, scale: f32, app: &dyn AppHandle) {
        ui.add_space(2.0 * scale);
        self.draw_common_opt_menu(ui, i, scale, app);
        ui.add_space(2.0 * scale);
        self.draw_graphics_opt_menu(ui, i, scale, app);
        ui.add_space(2.0 * scale);
        self.draw_control_opt_menu(ui, i, scale, app);
        ui.add_space(2.0 * scale);
        self.draw_sound_opt_menu(ui, i, scale, app);
    }

    /// 일반 설정 메뉴를 그립니다.
    fn draw_common_opt_menu(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        scale: f32,
        _app: &dyn AppHandle,
    ) {
        let min = ui.cursor().min + egui::vec2(4.0, 0.0) * scale;
        let size = egui::vec2(MENU_WIDTH - 8.0, MENU_HEIGHT) * scale;
        let rect = egui::Rect::from_min_size(min, size);

        ui.painter().rect_filled(rect, 5.0 * scale, POSI_COLOR);
        let text = COMMON_OPT_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(MENU_FONT_SIZE * scale, family);
        let menu_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let label = egui::Label::new(menu_text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.put(rect, label);
    }

    /// 그래픽 설정 메뉴를 그립니다.
    fn draw_graphics_opt_menu(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        scale: f32,
        app: &dyn AppHandle,
    ) {
        let min = ui.cursor().min + egui::vec2(4.0, 0.0) * scale;
        let size = egui::vec2(MENU_WIDTH - 8.0, MENU_HEIGHT) * scale;
        let rect = egui::Rect::from_min_size(min, size);

        let response = ui.allocate_rect(rect, egui::Sense::all());
        let color = if response.clicked() {
            // 다른 게임 장면으로 전환합니다.
            let event_loop_proxy = app.event_loop_proxy();
            let scene = LobbyGraphicsOptionModalLayer::new(
                self.locale,
                self.num_remaining_tasks,
                self.task_results.clone(),
            );
            let flow = GameSceneFlow::Change(Box::new(scene));
            let event = if !self.is_changed() {
                AppEvent::AddGameSceneFlow(flow)
            } else {
                let scene = LobbyOptionSaveGuardLayer::new(
                    self.locale,
                    ChangeOption::Common {
                        locale: self.locale,
                    },
                    flow,
                );
                let flow = GameSceneFlow::Push(Box::new(scene));
                AppEvent::AddGameSceneFlow(flow)
            };
            event_loop_proxy.send_event(event).unwrap();

            POSI_FOCUS_COLOR
        } else if response.is_pointer_button_down_on() {
            POSI_FOCUS_COLOR
        } else if response.hovered() | response.has_focus() {
            NORM_FOCUS_COLOR
        } else {
            NORM_COLOR
        };

        ui.painter().rect_filled(rect, 5.0 * scale, color);
        let text = GRAPHICS_OPT_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(MENU_FONT_SIZE * scale, family);
        let menu_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let label = egui::Label::new(menu_text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.put(rect, label);
    }

    /// 조작키 설정 메뉴를 그립니다.
    fn draw_control_opt_menu(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        scale: f32,
        app: &dyn AppHandle,
    ) {
        let min = ui.cursor().min + egui::vec2(4.0, 0.0) * scale;
        let size = egui::vec2(MENU_WIDTH - 8.0, MENU_HEIGHT) * scale;
        let rect = egui::Rect::from_min_size(min, size);

        let response = ui.allocate_rect(rect, egui::Sense::all());
        let color = if response.clicked() {
            // 다른 게임 장면으로 전환합니다.
            let event_loop_proxy = app.event_loop_proxy();
            let scene = LobbyControlOptionModalLayer::new(
                self.locale,
                self.num_remaining_tasks,
                self.task_results.clone(),
            );
            let flow = GameSceneFlow::Change(Box::new(scene));
            let event = if !self.is_changed() {
                AppEvent::AddGameSceneFlow(flow)
            } else {
                let scene = LobbyOptionSaveGuardLayer::new(
                    self.locale,
                    ChangeOption::Common {
                        locale: self.locale,
                    },
                    flow,
                );
                let flow = GameSceneFlow::Push(Box::new(scene));
                AppEvent::AddGameSceneFlow(flow)
            };
            event_loop_proxy.send_event(event).unwrap();

            POSI_FOCUS_COLOR
        } else if response.is_pointer_button_down_on() {
            POSI_FOCUS_COLOR
        } else if response.hovered() | response.has_focus() {
            NORM_FOCUS_COLOR
        } else {
            NORM_COLOR
        };

        ui.painter().rect_filled(rect, 5.0 * scale, color);
        let text = CONTROL_OPT_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(MENU_FONT_SIZE * scale, family);
        let menu_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let label = egui::Label::new(menu_text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.put(rect, label);
    }

    /// 사운드 설정 메뉴를 그립니다.
    fn draw_sound_opt_menu(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        scale: f32,
        app: &dyn AppHandle,
    ) {
        let min = ui.cursor().min + egui::vec2(4.0, 0.0) * scale;
        let size = egui::vec2(MENU_WIDTH - 8.0, MENU_HEIGHT) * scale;
        let rect = egui::Rect::from_min_size(min, size);

        let response = ui.allocate_rect(rect, egui::Sense::all());
        let color = if response.clicked() {
            // 다른 게임 장면으로 전환합니다.
            let event_loop_proxy = app.event_loop_proxy();
            let scene = LobbySoundOptionModalLayer::new(
                self.locale,
                self.num_remaining_tasks,
                self.task_results.clone(),
            );
            let flow = GameSceneFlow::Change(Box::new(scene));
            let event = if !self.is_changed() {
                AppEvent::AddGameSceneFlow(flow)
            } else {
                let scene = LobbyOptionSaveGuardLayer::new(
                    self.locale,
                    ChangeOption::Common {
                        locale: self.locale,
                    },
                    flow,
                );
                let flow = GameSceneFlow::Push(Box::new(scene));
                AppEvent::AddGameSceneFlow(flow)
            };
            event_loop_proxy.send_event(event).unwrap();

            POSI_FOCUS_COLOR
        } else if response.is_pointer_button_down_on() {
            POSI_FOCUS_COLOR
        } else if response.hovered() | response.has_focus() {
            NORM_FOCUS_COLOR
        } else {
            NORM_COLOR
        };

        ui.painter().rect_filled(rect, 5.0 * scale, color);
        let text = SOUND_OPT_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(MENU_FONT_SIZE * scale, family);
        let menu_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let label = egui::Label::new(menu_text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.put(rect, label);
    }

    /// 설정을 그립니다.
    fn draw_options(&mut self, ui: &mut egui::Ui, i: usize, scale: f32, app: &dyn AppHandle) {
        ui.add_space(2.0 * scale);
        ui.horizontal(|ui| {
            ui.add_space(4.0 * scale);
            self.draw_locale_opt(ui, i, scale, app);
        });
    }

    /// 언어 선택 옵션을 그립니다.
    fn draw_locale_opt(&mut self, ui: &mut egui::Ui, i: usize, scale: f32, _app: &dyn AppHandle) {
        let text = LOCALE_OPT_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(MAIN_FONT_SIZE * scale, family);
        let locale_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let local_label = egui::Label::new(locale_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        ui.with_layout(egui::Layout::left_to_right(egui::Align::BOTTOM), |ui| {
            ui.set_min_width((CONTENT_WIDTH * 0.5 - 4.0) * scale);
            ui.set_max_width((CONTENT_WIDTH * 0.5 - 4.0) * scale);
            ui.set_min_height(SUB_HEIGHT * scale);
            ui.set_max_height(SUB_HEIGHT * scale);
            ui.add(local_label);
        });

        let pos = ui.cursor().min + egui::vec2(0.0, SUB_HEIGHT * 0.1 * scale);
        let width = (ui.available_width() - 4.0 * scale).max(0.0);
        let old_style = (*ui.ctx().style()).clone();
        let mut new_style = old_style.clone();
        new_style.spacing.interact_size.y = SUB_HEIGHT * 0.8 * scale;
        new_style.visuals = egui::Visuals::light();
        ui.ctx().set_style(new_style);
        egui::Area::new(egui::Id::new("Locale_List_Order"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                ui.set_min_height(SUB_HEIGHT * scale);
                ui.set_max_height(SUB_HEIGHT * scale);
                let text = self.locale.to_string();
                let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                let font_id = egui::FontId::new(SUB_FONT_SIZE * scale, family);
                let locale_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

                let response = egui::ComboBox::from_id_salt(egui::Id::new("Locale_List"))
                    .width(width)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .selected_text(locale_text)
                    .show_ui(ui, |ui| {
                        let mut val = 0;
                        while let Some(locale) = Locale::new(val) {
                            let text = locale.to_string();
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(SUB_FONT_SIZE * scale, family);
                            let locale_text =
                                egui::RichText::new(text).font(font_id).color(FONT_COLOR);
                            ui.selectable_value(&mut self.locale, locale, locale_text);
                            val += 1;
                        }
                    });
                if response.response.changed() {
                    self.delay_time_sec = 0.3;
                }
            });
        ui.ctx().set_style(old_style);
    }

    /// 설정의 변경 여부를 반환합니다.
    fn is_changed(&self) -> bool {
        !(self.prev_locale == self.locale)
    }

    /// 저장 버튼을 그립니다.
    fn draw_save_button(&mut self, ui: &mut egui::Ui, i: usize, scale: f32, app: &dyn AppHandle) {
        // 저장 버튼 텍스트
        let text = SAVE_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(BTN_FONT_SIZE * scale, family);
        let save_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // 저장 버튼
        let (bg_color, line_color) = match self.save_btn_state {
            ButtonState::Idle => (POSI_COLOR, egui::Color32::TRANSPARENT),
            ButtonState::Hovered => (POSI_COLOR, POSI_FOCUS_COLOR),
            ButtonState::Pressed | ButtonState::Clicked => (POSI_FOCUS_COLOR, POSI_FOCUS_COLOR),
        };
        let save_button = egui::Button::new(save_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .min_size(BTN_SIZE * scale)
            .corner_radius(BTN_CORNER * scale)
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        // 저장 버튼 출력
        let enabled = self.is_changed() && self.num_remaining_tasks == 0;
        let response = ui.add_enabled(enabled, save_button);
        if response.clicked() && self.delay_time_sec <= 0.0 {
            self.delay_time_sec = 0.3;
            self.prev_locale = self.locale;
            self.save_btn_state = ButtonState::Clicked;

            // 사용자 구성 설정을 변경합니다.
            {
                let mut config = UserConfig::get();
                config.locale = self.locale;
            }

            // 사용자 구성 설정을 저장합니다.
            let mut path = app.current_dir().to_path_buf();
            path.push(format!("assets/{}", USER_CONFIG));

            let task_results = self.task_results.clone();
            app.io_threads().spawn(move || {
                let result = match UserConfig::store_from_file(path) {
                    Ok(_) => TaskResult::Success,
                    Err(e) => TaskResult::Failed(e),
                };
                task_results.push(result);
            });

            self.num_remaining_tasks += 1;
        } else if response.is_pointer_button_down_on() {
            self.save_btn_state = ButtonState::Pressed;
        } else if response.hovered() | response.has_focus() {
            self.save_btn_state = ButtonState::Hovered;
        } else {
            self.save_btn_state = ButtonState::Idle;
        }
    }

    /// 나가기 버튼을 그립니다.
    fn draw_exit_button(&mut self, ui: &mut egui::Ui, i: usize, scale: f32, app: &dyn AppHandle) {
        // 나가기 버튼 텍스트
        let text = EXIT_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(BTN_FONT_SIZE * scale, family);
        let exit_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // 나가기 버튼
        let (bg_color, line_color) = match self.exit_btn_state {
            ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
            ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
            ButtonState::Pressed | ButtonState::Clicked => (NORM_EXP_COLOR, egui::Color32::BLACK),
        };
        let exit_button = egui::Button::new(exit_text)
            .sense(egui::Sense::all())
            .fill(bg_color)
            .min_size(BTN_SIZE * scale)
            .corner_radius(BTN_CORNER * scale)
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        // 나가기 버튼 출력
        let enabled = self.num_remaining_tasks == 0;
        let response = ui.add_enabled(enabled, exit_button);
        if response.clicked() && self.delay_time_sec <= 0.0 {
            self.exit_btn_state = ButtonState::Clicked;

            // 다른 게임 장면으로 전환합니다.
            let event_loop_proxy = app.event_loop_proxy();
            let flow = GameSceneFlow::Pop;
            let event = if !self.is_changed() {
                // 게임 장면에서 빠져나옵니다.
                AppEvent::AddGameSceneFlow(flow)
            } else {
                let scene = LobbyOptionSaveGuardLayer::new(
                    self.locale,
                    ChangeOption::Common {
                        locale: self.locale,
                    },
                    flow,
                );
                let flow = GameSceneFlow::Push(Box::new(scene));
                AppEvent::AddGameSceneFlow(flow)
            };
            event_loop_proxy.send_event(event).unwrap();
        } else if response.is_pointer_button_down_on() {
            self.exit_btn_state = ButtonState::Pressed;
        } else if response.hovered() | response.has_focus() {
            self.exit_btn_state = ButtonState::Hovered;
        } else {
            self.exit_btn_state = ButtonState::Idle;
        }
    }
}

impl GameScene for LobbyCommonOptionModalLayer {
    fn transparents(&self) -> bool {
        true
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
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, _app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::LobbyDataUpdate => Some(packet),
            _ => None,
        }
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
        if !repeat && self.delay_time_sec <= 0.0 && self.num_remaining_tasks == 0 {
            match code {
                KeyCode::Escape => {
                    // 다른 게임 장면으로 전환합니다.
                    let event_loop_proxy = app.event_loop_proxy();
                    let flow = GameSceneFlow::Pop;
                    let event = if !self.is_changed() {
                        // 게임 장면에서 빠져나옵니다.
                        AppEvent::AddGameSceneFlow(flow)
                    } else {
                        let scene = LobbyOptionSaveGuardLayer::new(
                            self.locale,
                            ChangeOption::Common {
                                locale: self.locale,
                            },
                            flow,
                        );
                        let flow = GameSceneFlow::Push(Box::new(scene));
                        AppEvent::AddGameSceneFlow(flow)
                    };
                    event_loop_proxy.send_event(event).unwrap();
                }
                _ => {}
            }
        }

        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);

        if let Some(result) = self.task_results.pop() {
            self.num_remaining_tasks -= 1;
            if let TaskResult::Failed(e) = result {
                log::error!("failed to store user configuration! (REASON:{})", &e);
                let i = self.locale as usize;
                let title = ERR_TITLE_TEXTS[i];
                let message = e.to_string();
                let scene = MessageSceneLayer::new(self.locale, title, message, None);
                let flow = GameSceneFlow::Change(Box::new(scene));
                let event = AppEvent::AddGameSceneFlow(flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            };
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형 영역을 재조정합니다.
        let ctx = app.egui_ctx();
        let i = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        let text = TITLE_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(TITLE_FONT_SIZE * scale, family);
        let title_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        let frame = egui::Frame::new()
            .corner_radius(20.0 * scale)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(2.0 * scale, egui::Color32::BLACK));
        let mut modal = egui::Modal::new(egui::Id::new("Option_Modal"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(64));
        modal.area = modal.area.order(egui::Order::Middle);
        modal.show(ctx, |ui| {
            ui.shrink_clip_rect(clip_rect);
            ui.set_min_width(MODAL_WIDTH * scale);
            ui.set_max_width(MODAL_WIDTH * scale);
            ui.set_max_height(MODAL_HEIGHT * scale);

            ui.vertical_centered(|ui| {
                ui.set_min_width(MODAL_WIDTH * scale);
                ui.set_max_width(MODAL_WIDTH * scale);

                ui.add_space(8.0 * scale);
                ui.add(title_label);
                ui.add_space(8.0 * scale);

                let center_x = clip_rect.center().x;
                let beg_y = ui.next_widget_position().y;
                let beg = egui::pos2(center_x - 0.5 * MODAL_WIDTH * scale, beg_y);
                let end = egui::pos2(center_x + 0.5 * MODAL_WIDTH * scale, beg_y);
                ui.painter().line_segment(
                    [beg, end],
                    egui::Stroke::new(2.0 * scale, egui::Color32::BLACK),
                );

                ui.add_space(4.0 * scale);
                ui.horizontal(|ui| {
                    ui.set_min_height(CONTENT_HEIGHT * scale);
                    ui.set_max_height(CONTENT_HEIGHT * scale);
                    egui::ScrollArea::vertical()
                        .id_salt(egui::Id::new("Menu_Scroll"))
                        .show(ui, |ui| {
                            ui.set_min_width(MENU_WIDTH * scale);
                            ui.set_max_width(MENU_WIDTH * scale);
                            ui.set_min_height(CONTENT_HEIGHT * scale);
                            ui.set_max_height(CONTENT_HEIGHT * scale);
                            ui.vertical(|ui| {
                                self.draw_menu(ui, i, scale, app);
                            })
                        });

                    egui::ScrollArea::vertical()
                        .id_salt(egui::Id::new("Option_Scroll"))
                        .show(ui, |ui| {
                            ui.set_min_width(CONTENT_WIDTH * scale);
                            ui.set_max_width(CONTENT_WIDTH * scale);
                            ui.set_max_height(CONTENT_HEIGHT * scale);
                            ui.set_max_height(CONTENT_HEIGHT * scale);
                            ui.vertical(|ui| {
                                self.draw_options(ui, i, scale, app);
                            });
                        });
                });

                let end_y = ui.next_widget_position().y;
                let beg = egui::pos2(center_x + (MENU_WIDTH - 0.5 * MODAL_WIDTH) * scale, beg_y);
                let end = egui::pos2(center_x + (MENU_WIDTH - 0.5 * MODAL_WIDTH) * scale, end_y);
                ui.painter().line_segment(
                    [beg, end],
                    egui::Stroke::new(1.0 * scale, egui::Color32::BLACK),
                );
                let beg = egui::pos2(center_x - 0.5 * MODAL_WIDTH * scale, end_y);
                let end = egui::pos2(center_x + 0.5 * MODAL_WIDTH * scale, end_y);
                ui.painter().line_segment(
                    [beg, end],
                    egui::Stroke::new(2.0 * scale, egui::Color32::BLACK),
                );

                ui.add_space(12.0 * scale);
                egui::Grid::new("Button")
                    .num_columns(2)
                    .min_col_width(MODAL_WIDTH * 0.5 * scale)
                    .max_col_width(MODAL_WIDTH * 0.5 * scale)
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.set_min_height(42.0 * scale);
                            ui.set_max_height(42.0 * scale);
                            self.draw_save_button(ui, i, scale, app);
                        });

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.set_min_height(42.0 * scale);
                            ui.set_max_height(42.0 * scale);
                            self.draw_exit_button(ui, i, scale, app);
                        });
                    });
                ui.add_space(18.0 * scale);
            });
        });
    }
}
