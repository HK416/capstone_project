use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{SoundDataPool, TexturePool, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{InitSoundScene, BASE_WIDTH},
};

use super::InitFinishScene;

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["창 설정"];

/// 애플리케이션 표시 언어에 따른 창 화면 모드 설정 텍스트입니다.
const WINDOW_MODE_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["창 모드"];
/// 애플리케이션 표시 언어에 따른 창 화면 모드 텍스트입니다.
const WINDOW_MODE_TEXTS: [&'static str; NUM_LOCALE] = ["창 모드"];
/// 애플리케이션 표시 언어에 따른 전체 창 화면 모드 텍스트입니다.
const FULLSCREEN_MODE_TEXTS: [&'static str; NUM_LOCALE] = ["전체 화면"];

/// 애플리케이션 표시 언어에 따른 해상도 텍스트입니다.
const WINDOW_SIZE_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["해상도"];

/// 애플리케이션 표시 언어에 따른 확인 버튼 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성을 설정하는 장면입니다.  
/// 애플리케이션 창의 속성을 설정합니다.
pub struct InitWindowScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 최대 창 크기
    max_window_size: WindowSize,

    /// 창 크기
    window_size: WindowSize,
    /// 전체화면 여부
    is_fullscreen: bool,

    /// 설정이 완료된 여부
    completed: bool,
    /// 지연 시간
    delay_time_sec: f32,

    // 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InitWindowScene {
    /// 새로운 `InitWindowScene`을 생성합니다.
    pub fn new(locale: Locale, texture_pool: TexturePool, sound_data_pool: SoundDataPool) -> Self {
        Self {
            locale,
            max_window_size: WindowSize::MAX,
            window_size: WindowSize::MAX,
            is_fullscreen: true,
            completed: false,
            delay_time_sec: 0.3,
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

    /// 창 모드 옵션 라벨을 그립니다.
    fn draw_screen_mode_opt_label(&self, ui: &mut egui::Ui, i: usize, scale: f32) {
        let text = WINDOW_MODE_OPT_TEXTS[i];
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

    /// 창 모드 버튼을 그립니다.
    fn draw_screen_mode_buttons(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        scale: f32,
        app: &dyn AppHandle,
    ) {
        let min_size = ui.available_size() * egui::vec2(0.5, 1.0);

        let text = WINDOW_MODE_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(22.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let button = egui::Button::new(text)
            .corner_radius(5.0 * scale)
            .min_size(min_size);
        let enabled = !self.completed && self.is_fullscreen && self.delay_time_sec <= 0.0;
        let response = ui.add_enabled(enabled, button);
        if response.clicked() {
            // 설정을 변경합니다.
            self.is_fullscreen = false;
            self.delay_time_sec = 0.3;
            let event = AppEvent::FullScreenRequest(self.is_fullscreen);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        let text = FULLSCREEN_MODE_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(22.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let button = egui::Button::new(text)
            .corner_radius(5.0 * scale)
            .min_size(min_size);
        let enabled = !self.completed && !self.is_fullscreen && self.delay_time_sec <= 0.3;
        let response = ui.add_enabled(enabled, button);
        if response.clicked() {
            // 설정을 변경합니다.
            self.is_fullscreen = true;
            self.delay_time_sec = 0.3;
            let event = AppEvent::FullScreenRequest(self.is_fullscreen);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    /// 화면 크기 옵션 라벨을 그립니다.
    fn draw_screen_size_label(&self, ui: &mut egui::Ui, i: usize, scale: f32) {
        let text = WINDOW_SIZE_OPT_TEXTS[i];
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

    /// 화면 크기 리스트 콤보박스를 그립니다.
    fn draw_screen_size_list(&mut self, ui: &mut egui::Ui, scale: f32, app: &dyn AppHandle) {
        let text = self.window_size.to_string();
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(22.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        let mut changed = false;
        let width = ui.available_width();
        egui::ComboBox::from_id_salt(egui::Id::new("Resolution_List"))
            .width(width)
            .wrap_mode(egui::TextWrapMode::Truncate)
            .selected_text(text)
            .show_ui(ui, |ui| {
                ui.set_max_width(width);

                let mut val = self.max_window_size;
                while let Some(size) = val.downgrade() {
                    let text = size.to_string();
                    let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                    let font_id = egui::FontId::new(22.0 * scale, family);
                    let text = egui::RichText::new(text)
                        .font(font_id)
                        .color(egui::Color32::WHITE);
                    let response = ui.selectable_value(&mut self.window_size, size, text);
                    if response.clicked() {
                        changed = true;
                    }
                    val = size;
                }
            });

        if changed {
            // 설정을 변경합니다.
            let event = AppEvent::ResizeRequest(self.window_size);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
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

impl GameScene for InitWindowScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
        window.set_cursor_visible(true);

        // 최대 윈도우 크기를 설정합니다.
        self.max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor.size()))
            .flatten()
            .unwrap_or(WindowSize::MAX);
        self.window_size = app.window_size();
        self.is_fullscreen = app.is_fullscreen();
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let mut config = UserConfig::get();
        config.window_size = self.window_size;
        config.is_fullscreen = self.is_fullscreen;
    }

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        self.window_size = app.window_size();
        self.is_fullscreen = app.is_fullscreen();
        self.max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor.size()))
            .flatten()
            .unwrap_or(WindowSize::MAX);
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        self.delay_time_sec = (self.delay_time_sec - elapsed_time_sec).max(0.0);

        if self.completed {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = InitSoundScene::new(
                self.locale,
                self.texture_pool.clone(),
                self.sound_data_pool.clone(),
            );
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({:?})", &self)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
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
        let width = WIDTH * scale;
        let height = HEIGHT * scale;
        let ctx = app.egui_ctx();
        let old_style = (*ctx.style()).clone();
        let mut new_style = old_style.clone();
        new_style.spacing.interact_size.y = height * 0.8;
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

                    ui.add_space(8.0 * scale);
                    egui::Grid::new("Window_Mode_Grid")
                        .num_columns(2)
                        .min_col_width(width * 0.5)
                        .max_col_width(width * 0.5)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_screen_mode_opt_label(ui, i, scale);
                                },
                            );

                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_screen_mode_buttons(ui, i, scale, app);
                                },
                            );
                        });

                    ui.add_space(4.0 * scale);
                    egui::Grid::new("Window_Size_Grid")
                        .num_columns(2)
                        .min_col_width(width * 0.5)
                        .max_col_width(width * 0.5)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    self.draw_screen_size_label(ui, i, scale);
                                },
                            );

                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(height);
                                    ui.set_max_height(height);
                                    let pos = ui.cursor().min + egui::vec2(0.0, height * 0.1);
                                    let width = (ui.available_width() - 4.0 * scale).max(0.0);
                                    egui::Area::new(egui::Id::new("Resolution_List_Order"))
                                        .order(egui::Order::Middle)
                                        .fixed_pos(pos)
                                        .show(ui.ctx(), |ui| {
                                            ui.set_min_width(width);
                                            ui.set_max_width(width);
                                            ui.set_min_height(height);
                                            ui.set_max_height(height);
                                            let enabled =
                                                !self.completed && self.delay_time_sec <= 0.0;
                                            ui.add_enabled_ui(enabled, |ui| {
                                                self.draw_screen_size_list(ui, scale, app);
                                            });
                                        });
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
