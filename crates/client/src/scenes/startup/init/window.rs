use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{TexturePool, NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::BASE_WIDTH,
};

use super::InitFinishScene;

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["창 설정"];
/// 애플리케이션 표시 언어에 따른 해상도 텍스트입니다.
const SIZE_TEXTS: [&'static str; NUM_LOCALE] = ["해상도"];
/// 애플리케이션 표시 언어에 따른 전체 창 화면 텍스트입니다.
const FULLSCREEN_TEXT: [&'static str; NUM_LOCALE] = ["전체 화면"];
/// 애플리케이션 표시 언어에 따른 전체 화면 모드 텍스트입니다.
const SCREEN_MODE_TEXTS: [[&'static str; 2]; NUM_LOCALE] = [["전체 화면 모드", "창 모드"]];
/// 애플리케이션 표시 언어에 따른 확인 버튼 텍스트입니다.
const OKAY_TEXTS: [&'static str; NUM_LOCALE] = ["확인"];

/// 화면 모드 목록입니다.
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ScreenMode {
    Fullscreen = 0,
    Window = 1,
}

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성을 설정하는 장면입니다.  
/// 애플리케이션 창의 속성을 설정합니다.
pub struct InitWindowScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 최대 창 크기
    max_window_size: WindowSize,

    /// 창 크기
    window_size: WindowSize,
    /// 화면 모드
    screen_mode: ScreenMode,
    /// 설정이 완료된 여부
    completed: bool,

    // 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl InitWindowScene {
    /// 새로운 `InitWindowScene`을 생성합니다.
    pub fn new(texture_pool: TexturePool) -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale,
            max_window_size: WindowSize::MAX,
            window_size: WindowSize::MAX,
            screen_mode: ScreenMode::Fullscreen,
            completed: false,
            texture_pool,
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
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let mut config = UserConfig::get();
        config.window_size = self.window_size;
        config.is_fullscreen = match self.screen_mode {
            ScreenMode::Fullscreen => true,
            ScreenMode::Window => false,
        };
    }

    fn on_window_resized(&mut self, window: &Window, _app: &dyn AppHandle) {
        // 최대 윈도우 크기를 설정합니다.
        self.max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor.size()))
            .flatten()
            .unwrap_or(WindowSize::MAX);
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        let event_loop_proxy = app.event_loop_proxy();
        let event = AppEvent::ResizeRequest(self.window_size);
        event_loop_proxy.send_event(event).unwrap();
        let event = AppEvent::FullScreenRequest(match self.screen_mode {
            ScreenMode::Fullscreen => true,
            ScreenMode::Window => false,
        });
        event_loop_proxy.send_event(event).unwrap();

        if self.completed {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = InitFinishScene::new(self.texture_pool.clone());
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
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
        let font_id = egui::FontId::new(48.0 * scale, family);
        let title_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 해상도 텍스트
        let text = SIZE_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let resolution_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 전체화면 텍스트
        let text = FULLSCREEN_TEXT[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let fullscreen_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 확인 버튼 텍스트
        let text = OKAY_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 확인 버튼
        let width = 240.0 * scale;
        let height = width * 0.25;
        let okay_btn = egui::Button::new(okay_text)
            .fill(egui::Color32::DARK_GRAY)
            .corner_radius(1.5)
            .min_size((width, height).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::WHITE));

        // 해상도 옵션 텍스트
        let text = self.window_size.to_string();
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let window_size_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 해상도 목록
        let width = 320.0 * scale;
        let height = 48.0 * scale;
        let resolution_lists = egui::ComboBox::from_id_salt("Resolution")
            .selected_text(window_size_text)
            .width(width)
            .height(height);

        // 화면 모드 텍스트
        let mode = self.screen_mode as usize;
        let text = SCREEN_MODE_TEXTS[locale][mode];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(32.0 * scale, family);
        let screen_mode_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 화면 모드 목록
        let width = 320.0 * scale;
        let height = 48.0 * scale;
        let screen_mode_lists = egui::ComboBox::from_id_salt("ScreenMode")
            .selected_text(screen_mode_text)
            .width(width)
            .height(height);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.set_width(960.0 * scale);
                    ui.add_space(16.0 * scale);
                    ui.label(title_text);
                    ui.separator();
                    ui.add_space(16.0 * scale);

                    ui.add_enabled_ui(!self.completed, |ui| {
                        ui.columns(2, |cols| {
                            let ui = &mut cols[0];
                            ui.label(resolution_text);

                            let ui = &mut cols[1];
                            resolution_lists.show_ui(ui, |ui| {
                                ui.set_min_width(320.0 * scale);
                                ui.set_max_width(320.0 * scale);

                                let mut max_window_size = Some(self.max_window_size);
                                while let Some(window_size) = max_window_size {
                                    let text = window_size.to_string();
                                    let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                                    let font_id = egui::FontId::new(18.0 * scale, family);
                                    let text = egui::RichText::new(text)
                                        .font(font_id)
                                        .color(egui::Color32::WHITE);

                                    ui.selectable_value(&mut self.window_size, window_size, text);
                                    max_window_size = window_size.downgrade();
                                }
                            });
                        });
                        ui.add_space(4.0 * scale);
                        ui.columns(2, |cols| {
                            let ui = &mut cols[0];
                            ui.label(fullscreen_text);

                            let ui = &mut cols[1];
                            screen_mode_lists.show_ui(ui, |ui| {
                                ui.set_min_width(320.0 * scale);
                                ui.set_max_width(320.0 * scale);

                                let text = SCREEN_MODE_TEXTS[locale][0];
                                let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                                let font_id = egui::FontId::new(18.0 * scale, family);
                                let text = egui::RichText::new(text)
                                    .font(font_id)
                                    .color(egui::Color32::WHITE);
                                ui.selectable_value(
                                    &mut self.screen_mode,
                                    ScreenMode::Fullscreen,
                                    text,
                                );

                                let text = SCREEN_MODE_TEXTS[locale][1];
                                let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                                let font_id = egui::FontId::new(18.0 * scale, family);
                                let text = egui::RichText::new(text)
                                    .font(font_id)
                                    .color(egui::Color32::WHITE);
                                ui.selectable_value(
                                    &mut self.screen_mode,
                                    ScreenMode::Window,
                                    text,
                                );
                            });
                        });

                        ui.add_space(16.0 * scale);
                        if ui.add(okay_btn).clicked() {
                            self.completed = true;
                        }
                    });
                });
            });
    }
}
