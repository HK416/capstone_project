use std::collections::VecDeque;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::CharacterKind;
use mod_render::UiRenderer;
use rodio::{Sink, Source};
use winit::window::Window;

use crate::{
    asset::{
        SoundDataPool, TexturePool, BG_SOUND_THEME_01, CV_SOUND_TITLE, CV_YUUKA_OPTION,
        NOTOSANS_BOLD, NOTOSANS_REGULAR, UI_BUTTON_TOUCH,
    },
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{InitFinishScene, BASE_WIDTH},
};

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["사운드 설정"];
/// 애플리케이션 표시 언어에 따른 `배경음` 텍스트입니다.
const BG_SOUND_TEXTS: [&'static str; NUM_LOCALE] = ["배경음"];
/// 애플리케이션 표시 언어에 따른 `효과음` 텍스트입니다.
const EFX_SOUND_TEXTS: [&'static str; NUM_LOCALE] = ["효과음"];
/// 애플리케이션 표시 언어에 따른 `목소리` 텍스트입니다.
const VC_SOUND_TEXTS: [&'static str; NUM_LOCALE] = ["보이스"];

/// 애플리케이션 표시 언어에 따른 확인 버튼 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성 설정하는 장면입니다.
/// 애플리케이션 음량을 조절합니다.
pub struct InitSoundScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 지연 시간
    delay_time_sec: f32,
    count: usize,

    /// 현재 재생 중인 배경음
    background_sounds: VecDeque<Sink>,

    /// 설정 완료 여부
    completed: bool,
    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InitSoundScene {
    /// 새로운 `InitSoundScene`을 생성합니다.
    pub fn new(locale: Locale, texture_pool: TexturePool, sound_data_pool: SoundDataPool) -> Self {
        Self {
            locale,
            background_volume: 204,
            effect_volume: 204,
            voice_volume: 204,
            delay_time_sec: 0.3,
            count: 0,
            background_sounds: VecDeque::with_capacity(8),
            completed: false,
            texture_pool,
            sound_data_pool,
        }
    }

    /// 타이틀 라벨을 그립니다.
    fn draw_title_label(&self, ui: &mut egui::Ui, i: usize, scale: f32) {
        // 해상도 텍스트
        let text = TITLE_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add(label);
    }

    /// 배경음 옵션 라벨을 그립니다.
    fn draw_background_sound_label(&mut self, ui: &mut egui::Ui, scale: f32, i: usize) {
        let text = BG_SOUND_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add(label);
    }

    /// 배경음 조절 옵션을 그립니다.
    fn draw_background_sound_opt(&mut self, ui: &mut egui::Ui) {
        let slider = egui::Slider::new(&mut self.background_volume, 0..=255).show_value(false);
        let response = ui.add(slider);
        if response.changed() {
            for sink in self.background_sounds.iter_mut() {
                sink.set_volume(self.background_volume as f32 / 255.0);
            }
        }
    }

    /// 배경음 볼륨 라벨을 그립니다.
    fn draw_background_volume_label(&mut self, ui: &mut egui::Ui, scale: f32, size: egui::Vec2) {
        let percent = self.background_volume as f32 / 255.0 * 100.0;
        let text = format!("{}", percent.round() as u32);
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add_sized(size, label);
    }

    /// 효과음 옵션 라벨을 그립니다.
    fn draw_effect_sound_label(&mut self, ui: &mut egui::Ui, scale: f32, i: usize) {
        let text = EFX_SOUND_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add(label);
    }

    /// 효과음 조절 옵션을 그립니다.
    fn draw_effect_sound_opt(&mut self, ui: &mut egui::Ui, app: &dyn AppHandle) {
        let slider = egui::Slider::new(&mut self.effect_volume, 0..=255).show_value(false);
        let response = ui.add(slider);
        if response.drag_stopped() {
            let decoded = self
                .sound_data_pool
                .get(UI_BUTTON_TOUCH)
                .expect("UI_Button_Touch sound must be preloaded!");
            let source = decoded.as_source();
            let sink = Sink::connect_new(app.audio_mixer());
            sink.set_volume(self.effect_volume as f32 / 255.0);
            sink.append(source);
            sink.play();
            sink.detach();
        }
    }

    /// 효과음 볼륨 라벨을 그립니다.
    fn draw_effect_volume_label(&mut self, ui: &mut egui::Ui, scale: f32, size: egui::Vec2) {
        let percent = self.effect_volume as f32 / 255.0 * 100.0;
        let text = format!("{}", percent.round() as u32);
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add_sized(size, label);
    }

    /// 보이스 옵션 라벨을 그립니다.
    fn draw_voice_sound_label(&mut self, ui: &mut egui::Ui, scale: f32, i: usize) {
        let text = VC_SOUND_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add(label);
    }

    /// 보이스 조절 옵션을 그립니다.
    fn draw_voice_sound_opt(&mut self, ui: &mut egui::Ui, app: &dyn AppHandle) {
        let slider = egui::Slider::new(&mut self.voice_volume, 0..=255).show_value(false);
        let response = ui.add(slider);
        if response.drag_stopped() {
            self.count = (self.count + 1) % 5;

            let i = CharacterKind::YuukaOriginal as usize;
            let uri = if self.count % 10 == 0 {
                CV_YUUKA_OPTION
            } else {
                CV_SOUND_TITLE[i]
            };
            let decoded = self
                .sound_data_pool
                .get(uri)
                .expect("UI_Button_Touch sound must be preloaded!");
            let source = decoded.as_source();
            let sink = Sink::connect_new(app.audio_mixer());
            sink.set_volume(self.voice_volume as f32 / 255.0);
            sink.append(source);
            sink.play();
            sink.detach();
        }
    }

    /// 보이스 볼륨 라벨을 그립니다.
    fn draw_voice_volume_label(&mut self, ui: &mut egui::Ui, scale: f32, size: egui::Vec2) {
        let percent = self.voice_volume as f32 / 255.0 * 100.0;
        let text = format!("{}", percent.round() as u32);
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        ui.add_sized(size, label);
    }

    fn draw_okay_button(&mut self, ui: &mut egui::Ui, scale: f32, i: usize) {
        let text = OKAY_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(22.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let min_size = egui::vec2(256.0, 64.0) * scale;
        let button = egui::Button::new(text)
            .corner_radius(5.0 * scale)
            .min_size(min_size);
        let enabled = !self.completed && self.delay_time_sec <= 0.0;
        let response = ui.add_enabled(enabled, button);
        if response.clicked() {
            // 설정을 변경합니다.
            self.completed = true;
            self.delay_time_sec = 0.3;
        }
    }
}

impl GameScene for InitSoundScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
        window.set_cursor_visible(true);

        // 배경 음악을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(BG_SOUND_THEME_01)
            .expect("Theme_01 sound must be preloaded!");
        let source = decoded.as_source();
        let source = source.repeat_infinite();

        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.background_volume as f32 / 255.0);
        sink.append(source);
        sink.play();

        self.background_sounds.push_back(sink);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let mut config = UserConfig::get();
        config.background_volume = self.background_volume;
        config.effect_volume = self.effect_volume;
        config.voice_volume = self.voice_volume;
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);

        if self.completed {
            // 다음 게임 장면으로 전환합니다.
            let next_scene =
                InitFinishScene::new(self.texture_pool.clone(), self.sound_data_pool.clone());
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        const WIDTH: f32 = 960.0;
        const HEIGHT: f32 = 52.0;
        let ctx = app.egui_ctx();
        let width = WIDTH * scale;
        let height = HEIGHT * scale;
        let old_style = (*ctx.style()).clone();
        let mut new_style = old_style.clone();
        new_style.spacing.slider_width = width * 0.35;
        ctx.set_style(new_style);
        egui::Area::new(egui::Id::new("Options"))
            .order(egui::Order::Background)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.set_min_width(width);
                ui.set_max_width(width);

                ui.vertical_centered(|ui| {
                    ui.set_min_width(width);
                    ui.set_max_width(width);

                    self.draw_title_label(ui, i, scale);
                    ui.add_space(8.0 * scale);
                    ui.separator();
                    ui.add_space(4.0 * scale);

                    egui::Grid::new("Background_Opt_Grid")
                        .num_columns(2)
                        .min_col_width(width * 0.5)
                        .max_col_width(width * 0.5)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_background_sound_label(ui, scale, i);
                                },
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_background_volume_label(
                                        ui,
                                        scale,
                                        egui::vec2(width * 0.15, height),
                                    );
                                    self.draw_background_sound_opt(ui);
                                },
                            );
                        });
                    ui.add_space(4.0 * scale);

                    egui::Grid::new("Effect_Opt_Grid")
                        .num_columns(2)
                        .min_col_width(width * 0.5)
                        .max_col_width(width * 0.5)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_effect_sound_label(ui, scale, i);
                                },
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_effect_volume_label(
                                        ui,
                                        scale,
                                        egui::vec2(width * 0.15, height),
                                    );
                                    self.draw_effect_sound_opt(ui, app);
                                },
                            );
                        });
                    ui.add_space(4.0 * scale);

                    egui::Grid::new("Voice_Opt_Grid")
                        .num_columns(2)
                        .min_col_width(width * 0.5)
                        .max_col_width(width * 0.5)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_voice_sound_label(ui, scale, i);
                                },
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_voice_volume_label(
                                        ui,
                                        scale,
                                        egui::vec2(width * 0.15, height),
                                    );
                                    self.draw_voice_sound_opt(ui, app);
                                },
                            );
                        });

                    ui.add_space(4.0 * scale);
                    ui.separator();
                    ui.add_space(8.0 * scale);
                    self.draw_okay_button(ui, scale, i);
                });
            });
        ctx.set_style(old_style);
    }
}
