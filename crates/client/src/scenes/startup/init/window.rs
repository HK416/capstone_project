use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::BASE_WIDTH,
};

use super::InitFinishScene;

/// 애플리케이션 표시 언어에 따른 안내 텍스트입니다.
const INFO_TEXTS: [&'static str; NUM_LOCALE] = ["창 크기 설정"];
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

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl InitWindowScene {
    /// 새로운 `InitWindowScene`을 생성합니다.
    pub fn new() -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale,
            max_window_size: WindowSize::MAX,
            window_size: WindowSize::MAX,
            is_fullscreen: true,
            completed: false,
            egui_clip_primitives: Vec::default(),
            egui_free_texture_ids: Vec::default(),
        }
    }

    /// UI 콜백 함수
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let head_font_id = egui::FontId::new(64.0 * scale, head_font_family);
        let main_font_id = egui::FontId::new(32.0 * scale, main_font_family);
        let font_color = egui::Color32::WHITE;

        // 텍스트
        let text = INFO_TEXTS[self.locale as usize];
        let info_text = egui::RichText::new(text)
            .font(head_font_id.clone())
            .color(font_color.clone());
        let text = SIZE_TEXTS[self.locale as usize];
        let size_text = egui::RichText::new(text).font(main_font_id.clone());
        let text = FULLSCREEN_TEXT[self.locale as usize];
        let fullscreen_text = egui::RichText::new(text).font(main_font_id.clone());
        let text = OKAY_TEXTS[self.locale as usize];
        let okay_text = egui::RichText::new(text)
            .font(main_font_id.clone())
            .color(font_color.clone());

        // 콤보 박스 속성
        let current_size = egui::RichText::new(self.window_size.to_string())
            .font(main_font_id.clone())
            .color(font_color.clone());

        // 콤보 박스
        let combobox = egui::ComboBox::from_label("").selected_text(current_size);

        // 버튼
        let okay_btn = egui::Button::new(okay_text);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(app.egui_ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(info_text);
                    ui.separator();

                    egui::Grid::new("SubLayout").num_columns(2).show(ui, |ui| {
                        ui.label(size_text);

                        ui.add_enabled_ui(!self.is_fullscreen, |ui| {
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
                        });
                        ui.end_row();

                        ui.label(fullscreen_text);
                        ui.checkbox(&mut self.is_fullscreen, "");
                        ui.end_row();
                    });

                    ui.separator();
                    ui.vertical_centered(|ui| {
                        if ui.add(okay_btn).clicked() {
                            self.completed = true;
                        }
                    });
                });
            });

        Ok(())
    }
}

impl GameScene for InitWindowScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
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

        Ok(())
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let mut config = UserConfig::get();
        config.window_size = self.window_size;
        config.is_fullscreen = self.is_fullscreen;
        Ok(())
    }

    fn on_window_resized(
        &mut self,
        window: &Window,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 최대 윈도우 크기를 설정합니다.
        self.max_window_size = window
            .current_monitor()
            .map(|monitor| WindowSize::find_maximize_size(monitor))
            .flatten()
            .unwrap_or(WindowSize::MAX);
        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let event_loop_proxy = app.event_loop_proxy();
        let event = AppEvent::ResizeRequest(self.window_size);
        event_loop_proxy.send_event(event).unwrap();
        let event = AppEvent::FullScreenRequest(self.is_fullscreen);
        event_loop_proxy.send_event(event).unwrap();

        if self.completed {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = Box::new(InitFinishScene::new());
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            event_loop_proxy.send_event(event).unwrap();
        }

        Ok(())
    }

    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        commands.push(encoder.finish());
        queue.submit(commands);

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("RenderPass(UI({}))", stringify!(InitLocaleScene))),
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
                })
                .forget_lifetime();

            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }

    fn on_finish_draw(
        &mut self,
        _window: &Window,
        egui_renderer: &mut UiRenderer,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}
