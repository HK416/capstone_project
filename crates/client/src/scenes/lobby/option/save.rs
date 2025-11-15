use std::sync::Arc;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use rodio::Sink;
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        NOTOSANS_BOLD, NOTOSANS_REGULAR, SoundDataPool, UI_BUTTON_BACK, UI_LOADING, UI_NOTICE,
        USER_CONFIG,
    },
    component::ButtonState,
    config::{Locale, NUM_LOCALE, UserConfig},
    scenes::{
        BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS, FONT_COLOR,
        FatalErrorSceneLayer, MessageSceneLayer, NORM_COLOR, NORM_EXP_COLOR, NORM_FOCUS_COLOR,
        POSI_COLOR, POSI_FOCUS_COLOR,
    },
};

use super::*;

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["설정 저장"];
/// 애플리케이션 표시 언어에 따른 메시지 텍스트입니다.
const MESSAGE_TEXTS: [&'static str; NUM_LOCALE] = ["변경된 설정을 저장하시겠습니까?"];

/// 애플리케이션 표시 언어에 따른 `예` 버튼 텍스트입니다.
const YES_TEXTS: [&'static str; NUM_LOCALE] = ["예"];
/// 애플리케이션 표시 언어에 따른 `아니오` 버튼 텍스트입니다.
const NO_TEXTS: [&'static str; NUM_LOCALE] = ["아니오"];

/// 사용자에게 한번 더 질문하는 레이어입니다.
pub struct LobbyOptionSaveGuardLayer {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,
    /// 변경된 설정 내용이 담긴 사용자 구성 데이터
    option: ChangeOption,
    /// 다음 게임 장면 흐름
    flow: Option<GameSceneFlow>,

    /// 예 버튼 상태
    yes_btn_state: ButtonState,
    /// 아니오 버튼 상태
    no_btn_state: ButtonState,
    /// 입력 지연 시간입니다.
    delay_time_sec: f32,

    /// 남은 작업의 수
    num_remaining_tasks: usize,
    /// 작업 결과 목록
    task_results: Arc<Queue<TaskResult>>,

    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl LobbyOptionSaveGuardLayer {
    /// 새로운 `LobbyOptionSaveGuardLayer`를 생성합니다.
    pub fn new(
        locale: Locale,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        option: ChangeOption,
        flow: GameSceneFlow,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            background_volume,
            effect_volume,
            voice_volume,
            option,
            flow: Some(flow),
            yes_btn_state: ButtonState::Idle,
            no_btn_state: ButtonState::Idle,
            delay_time_sec: 0.3,
            num_remaining_tasks: 0,
            task_results: Arc::new(Queue::new()),
            sound_data_pool,
        }
    }
}

impl GameScene for LobbyOptionSaveGuardLayer {
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
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            title,
            message,
            self.sound_data_pool.clone(),
        );
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 효과음을 재생합니다.
        if let Some(mixer) = app.audio_mixer() {
            let decoded = self
                .sound_data_pool
                .get(UI_NOTICE)
                .expect("UI_Notice sound must be preloaded!");
            let source = decoded.as_source();
            let sink = Sink::connect_new(mixer);
            sink.set_volume(self.effect_volume as f32 / 255.0);
            sink.append(source);
            sink.play();
            sink.detach();
        }
    }

    fn on_keyboard_released(
        &mut self,
        _code: KeyCode,
        _location: KeyLocation,
        _modifiers: Modifiers,
        _repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);

        if let Some(result) = self.task_results.pop() {
            if let Some(next_flow) = self.flow.take() {
                // 장면을 전환합니다.
                let event_loop_proxy = app.event_loop_proxy();
                match result {
                    TaskResult::Success => {
                        let flow = GameSceneFlow::Pop;
                        let event = AppEvent::AddGameSceneFlow(flow);
                        event_loop_proxy.send_event(event).unwrap();
                        let event = AppEvent::AddGameSceneFlow(next_flow);
                        event_loop_proxy.send_event(event).unwrap();
                    }
                    TaskResult::Failed(e) => {
                        log::error!("failed to store user configuration! (REASON:{})", &e);
                        let i = self.locale as usize;
                        let title = ERR_TITLE_TEXTS[i];
                        let message = e.to_string();
                        let scene = MessageSceneLayer::new(
                            self.locale,
                            self.background_volume,
                            self.effect_volume,
                            self.voice_volume,
                            title,
                            message,
                            Some(next_flow),
                            self.sound_data_pool.clone(),
                        );
                        let flow = GameSceneFlow::Change(Box::new(scene));
                        let event = AppEvent::AddGameSceneFlow(flow);
                        event_loop_proxy.send_event(event).unwrap();

                        // 효과음을 재생합니다.
                        if let Some(mixer) = app.audio_mixer() {
                            let decoded = self
                                .sound_data_pool
                                .get(UI_NOTICE)
                                .expect("UI_Notice sound must be preloaded!");
                            let source = decoded.as_source();
                            let sink = Sink::connect_new(mixer);
                            sink.set_volume(self.effect_volume as f32 / 255.0);
                            sink.append(source);
                            sink.play();
                            sink.detach();
                        }
                    }
                };
            }
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

        // 타이틀 텍스트
        let text = TITLE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(36.0 * scale, family);
        let title_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let title_label = egui::Label::new(title_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // 메시지 텍스트
        let text = MESSAGE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let message_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
        let message_label = egui::Label::new(message_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        // `예` 버튼 텍스트
        let text = YES_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let yes_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // `아니오` 버튼 텍스트
        let text = NO_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let no_text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);

        // 예 버튼
        let (bg_color, line_color) = match self.yes_btn_state {
            ButtonState::Idle => (POSI_COLOR, egui::Color32::TRANSPARENT),
            ButtonState::Hovered => (POSI_COLOR, POSI_FOCUS_COLOR),
            ButtonState::Pressed | ButtonState::Clicked => (POSI_FOCUS_COLOR, POSI_FOCUS_COLOR),
        };
        let yes_button = egui::Button::new(yes_text)
            .fill(bg_color)
            .sense(egui::Sense::all())
            .min_size(BTN_SIZE * scale)
            .corner_radius(BTN_CORNER * scale)
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        // `아니오` 버튼
        let (bg_color, line_color) = match self.no_btn_state {
            ButtonState::Idle => (NORM_COLOR, egui::Color32::BLACK),
            ButtonState::Hovered => (NORM_FOCUS_COLOR, egui::Color32::BLACK),
            ButtonState::Pressed | ButtonState::Clicked => (NORM_EXP_COLOR, egui::Color32::BLACK),
        };
        let no_button = egui::Button::new(no_text)
            .fill(bg_color)
            .sense(egui::Sense::all())
            .min_size(BTN_SIZE * scale)
            .corner_radius(BTN_CORNER * scale)
            .stroke(egui::Stroke::new(1.0 * scale, line_color));

        let frame = egui::Frame::new()
            .corner_radius(20.0 * scale)
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::BLACK));
        let mut modal = egui::Modal::new(egui::Id::new("Save_Guard"))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(96));
        modal.area = modal.area.order(egui::Order::Tooltip);
        modal.show(app.egui_ctx(), |ui| {
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

                let enable = self.yes_btn_state != ButtonState::Clicked
                    && self.no_btn_state != ButtonState::Clicked
                    && self.flow.is_some();
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
                                    let enabled =
                                        self.flow.is_some() && self.num_remaining_tasks == 0;
                                    let response = ui.add_enabled(enabled, yes_button);
                                    if response.clicked() && self.delay_time_sec <= 0.0 {
                                        self.yes_btn_state = ButtonState::Clicked;

                                        {
                                            let mut config = UserConfig::get();
                                            match self.option {
                                                ChangeOption::Common { locale } => {
                                                    config.locale = locale;
                                                }
                                                ChangeOption::Graphics {
                                                    window_size,
                                                    is_fullscreen,
                                                } => {
                                                    config.window_size = window_size;
                                                    config.is_fullscreen = is_fullscreen;
                                                }
                                                ChangeOption::Control {} => todo!(),
                                                ChangeOption::Sound {
                                                    background_volume,
                                                    effect_volume,
                                                    voice_volume,
                                                } => {
                                                    config.background_volume = background_volume;
                                                    config.effect_volume = effect_volume;
                                                    config.voice_volume = voice_volume;
                                                }
                                            }
                                        }

                                        // 저장 작업을 처리합니다.
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

                                        // 효과음을 재생합니다.
                                        if let Some(mixer) = app.audio_mixer() {
                                            let decoded = self
                                                .sound_data_pool
                                                .get(UI_LOADING)
                                                .expect("UI_Loading sound must be preloaded!");
                                            let source = decoded.as_source();
                                            let sink = Sink::connect_new(mixer);
                                            sink.set_volume(self.effect_volume as f32 / 255.0);
                                            sink.append(source);
                                            sink.play();
                                            sink.detach();
                                        }
                                    } else if response.is_pointer_button_down_on() {
                                        self.yes_btn_state = ButtonState::Pressed;
                                    } else if response.hovered() | response.has_focus() {
                                        self.yes_btn_state = ButtonState::Hovered;
                                    } else {
                                        self.yes_btn_state = ButtonState::Idle;
                                    }
                                },
                            );

                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    // 아니오 버튼
                                    let enabled =
                                        self.flow.is_some() && self.num_remaining_tasks == 0;
                                    let response = ui.add_enabled(enabled, no_button);
                                    if response.clicked() && self.delay_time_sec <= 0.0 {
                                        self.no_btn_state = ButtonState::Clicked;
                                        self.task_results.push(TaskResult::Success);
                                        self.num_remaining_tasks += 1;

                                        // 효과음을 재생합니다.
                                        if let Some(mixer) = app.audio_mixer() {
                                            let decoded = self
                                                .sound_data_pool
                                                .get(UI_BUTTON_BACK)
                                                .expect("UI_Button_Back sound must be preloaded!");
                                            let source = decoded.as_source();
                                            let sink = Sink::connect_new(mixer);
                                            sink.set_volume(self.effect_volume as f32 / 255.0);
                                            sink.append(source);
                                            sink.play();
                                            sink.detach();
                                        }
                                    } else if response.is_pointer_button_down_on() {
                                        self.no_btn_state = ButtonState::Pressed;
                                    } else if response.hovered() | response.has_focus() {
                                        self.no_btn_state = ButtonState::Hovered;
                                    } else {
                                        self.no_btn_state = ButtonState::Idle;
                                    }
                                },
                            );
                        });
                });
                ui.add_space(18.0 * scale);
            });
        });
    }
}
