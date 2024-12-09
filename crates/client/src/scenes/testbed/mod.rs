use std::{error::Error, fmt};

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    component::StudentKind,
    config::{InvalidConfig, UserConfig},
};

/// ## Testbed Title Scene
pub struct TestbedTitleScene {
    config: UserConfig,

    fullscreen: bool,
    window_size: WindowSize,
    student_kind: StudentKind,

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedTitleScene {
    pub fn new(config: UserConfig) -> Self {
        Self {
            config,
            fullscreen: false,
            window_size: WindowSize::MAX,
            student_kind: StudentKind::ArisOriginal,
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }
}

impl TestbedTitleScene {
    /// 사용자 구성이 변경된 경우 `true`를 반환합니다.
    fn config_changed(&self) -> bool {
        self.fullscreen != self.config.fullscreen || self.window_size != self.config.window_size
    }

    /// 사용자 구성을 저장합니다.
    fn config_store(&mut self, app: &dyn AppHandle) -> Result<(), Box<dyn Error + Send>> {
        self.config.fullscreen = self.fullscreen;
        self.config.window_size = self.window_size;

        let data = serde_json::ser::to_vec_pretty(&self.config)
            .map_err(|e| InvalidConfig(e))
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        let asset_manager = app.asset_manager();
        asset_manager
            .store("user_config", &data)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        Ok(())
    }

    /// 사용자 인터페이스 콜백 함수입니다.
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        // 픽셀 크기에 따른 폰트 크기 계산
        // Point Size(폰트 크기) = Pixel Size / Scale Factor
        //
        let scale_factor = window.scale_factor() as f32;
        let (width, height): (f32, f32) = window.inner_size().into();

        let title_text = egui::RichText::new("Hello to Halo (개발자모드)")
            .color(egui::Color32::WHITE)
            .size((height * 0.075) / scale_factor);
        let fullscreen_text = egui::RichText::new("전체 화면")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let size_text = egui::RichText::new("창 크기")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let save_text = egui::RichText::new("설정 저장")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let student_text = egui::RichText::new("학생 선택")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let enter_text = egui::RichText::new("테스트 필드 입장")
            .color(egui::Color32::WHITE)
            .size(18.0);

        self.fullscreen = app.is_fullscreen();
        self.window_size = app.window_size();
        let mut store_config = false;
        let mut change_scene = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.vertical_centered(|ui| {
                    ui.label(title_text);
                });
                ui.separator();

                ui.columns(2, |cols| {
                    let ui = &mut cols[0];
                    egui::Grid::new("configurations")
                        .num_columns(2)
                        .spacing([(width * 0.02) / scale_factor, 4.0])
                        .show(ui, |ui| {
                            ui.label(fullscreen_text);
                            ui.checkbox(&mut self.fullscreen, "");
                            ui.end_row();

                            ui.label(size_text);
                            ui.add_enabled_ui(!self.fullscreen, |ui| {
                                egui::ComboBox::from_label("")
                                    .selected_text(format!("{}", self.window_size.to_string()))
                                    .show_ui(ui, |ui| {
                                        if let Some(monitor) = window.current_monitor() {
                                            let mut max_window_size =
                                                WindowSize::find_maximize_size(monitor);

                                            while let Some(window_size) = max_window_size {
                                                ui.selectable_value(
                                                    &mut self.window_size,
                                                    window_size,
                                                    window_size.to_string(),
                                                );
                                                max_window_size = window_size.downgrade();
                                            }
                                        }
                                    });
                            });
                            ui.end_row();
                        });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(self.config_changed(), |ui| {
                            if ui.button(save_text).clicked() {
                                store_config = true;
                            }
                        });
                    });

                    let ui = &mut cols[1];
                    egui::Grid::new("testbed")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(student_text);
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{}", self.student_kind.to_string()))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.student_kind,
                                        StudentKind::ArisOriginal,
                                        StudentKind::ArisOriginal.to_string(),
                                    );
                                });
                        });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button(enter_text).clicked() {
                            change_scene = true;
                        }
                    });
                });
            });

        let proxy = app.event_loop_proxy();
        if self.fullscreen != app.is_fullscreen() {
            proxy
                .send_event(AppEvent::FullScreenRequest(self.fullscreen))
                .unwrap();
        }

        if self.window_size != app.window_size() {
            proxy
                .send_event(AppEvent::ResizeRequest(self.window_size))
                .unwrap();
        }

        if change_scene {
            proxy
                .send_event(AppEvent::SetGameSceneFlow(GameSceneFlow::Change(Box::new(
                    TestbedEnterScene::new(self.student_kind),
                ))))
                .unwrap();
        }

        if store_config {
            self.config_store(app)?;
        }

        Ok(())
    }
}

impl GameScene for TestbedTitleScene {
    fn on_enter(
        &mut self,
        window: &Window,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        window.set_visible(true);
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
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TestbadTitleScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

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

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl fmt::Debug for TestbedTitleScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestbedTitleScene))
    }
}

mod enter;
mod in_game;

pub use self::enter::*;
pub use self::in_game::*;
