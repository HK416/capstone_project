use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
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
    /// 전체 창 화면 여부
    is_fullscreen: bool,
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
            is_fullscreen: true,
            completed: false,
            texture_pool,
        }
    }
}

impl GameScene for InitWindowScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {
        log::info!("Enter GameScene({:?})", &self);

        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
        window.set_cursor_visible(true);

        // 최대 윈도우 크기를 설정합니다.
        self.max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor))
            .flatten()
            .unwrap_or(WindowSize::MAX);
        self.window_size = app.window_size();
    }

    fn on_exit(&mut self, _window: Option<&Window>, _app: &dyn AppHandle) {
        let mut config = UserConfig::get();
        config.window_size = self.window_size;
        config.is_fullscreen = self.is_fullscreen;
    }

    fn on_window_resized(&mut self, window: &Window, _app: &dyn AppHandle) {
        // 최대 윈도우 크기를 설정합니다.
        self.max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor))
            .flatten()
            .unwrap_or(WindowSize::MAX);
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        let event_loop_proxy = app.event_loop_proxy();
        let event = AppEvent::ResizeRequest(self.window_size);
        event_loop_proxy.send_event(event).unwrap();
        let event = AppEvent::FullScreenRequest(self.is_fullscreen);
        event_loop_proxy.send_event(event).unwrap();

        if self.completed {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = InitFinishScene::new(self.texture_pool.clone());
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::SetGameSceneFlow(scene_flow);
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
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 타이틀 텍스트
        let text = TITLE_TEXTS[self.locale as usize];
        let font_id = egui::FontId::new(48.0 * scale, head_font_family.clone());
        let info_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 해상도 텍스트
        let text = SIZE_TEXTS[self.locale as usize];
        let font_id = egui::FontId::new(32.0 * scale, main_font_family.clone());
        let resolution_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 전체화면 텍스트
        let text = FULLSCREEN_TEXT[self.locale as usize];
        let font_id = egui::FontId::new(32.0 * scale, main_font_family.clone());
        let fullscreen_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 확인 버튼 텍스트
        let text = OKAY_TEXTS[self.locale as usize];
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let okay_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 확인 버튼
        let okay_btn_width = 240.0 * scale;
        let okay_btn_height = okay_btn_width * 0.25;
        let okay_btn = egui::Button::new(okay_text)
            .fill(egui::Color32::DARK_GRAY)
            .min_size((okay_btn_width, okay_btn_height).into())
            .stroke(egui::Stroke::new(1.0 * scale, egui::Color32::WHITE));

        // 콤보 박스 속성
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let current_size = egui::RichText::new(self.window_size.to_string())
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 콤보 박스
        let combobox_width = 320.0 * scale;
        let combobox_height = combobox_width * 0.5;
        let combobox = egui::ComboBox::from_label("")
            .selected_text(current_size)
            .width(combobox_width)
            .height(combobox_height);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.set_width(960.0 * scale);
                    ui.add_space(16.0 * scale);
                    ui.label(info_text);
                    ui.separator();

                    ui.add_space(16.0 * scale);
                    ui.add_enabled_ui(!self.completed, |ui| {
                        ui.columns(2, |cols| {
                            let ui = &mut cols[0];
                            ui.set_width(480.0 * scale);
                            ui.set_height(64.0 * scale);
                            ui.label(resolution_text);
                            ui.label(fullscreen_text);

                            let ui = &mut cols[1];
                            ui.set_width(480.0 * scale);
                            ui.set_height(64.0 * scale);
                            combobox.show_ui(ui, |ui| {
                                let mut max_window_size = Some(self.max_window_size);
                                while let Some(window_size) = max_window_size {
                                    ui.selectable_value(
                                        &mut self.window_size,
                                        window_size,
                                        window_size.to_string(),
                                    );
                                    max_window_size = window_size.downgrade();
                                }
                            });
                            ui.checkbox(&mut self.is_fullscreen, "");
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
