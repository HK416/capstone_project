mod enter;
mod in_game;

use std::{error::Error, fmt};

use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{CharacterKind, ClientId};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    config::{InvalidConfig, UserConfig},
    USER_CONFIG,
};

pub use {self::enter::*, self::in_game::*};

/// ## Testbed Title Scene
pub struct TestbedTitleScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,

    /// 선택된 전체 화면 여부
    fullscreen: bool,
    /// 선택된 윈도우 창 크기
    window_size: WindowSize,
    /// 선택된 캐릭터 종류
    character_kind: CharacterKind,

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedTitleScene {
    /// 새로운 `TestbedTitleScene`을 생성합니다.
    pub fn new(user_config: Box<UserConfig>, client_id: ClientId) -> Self {
        assert_ne!(client_id, ClientId::NULL, "invalid client id");
        Self {
            user_config: Some(user_config),
            client_id,
            fullscreen: false,
            window_size: WindowSize::MAX,
            character_kind: CharacterKind::default(),
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 구성이 변경된 경우 `true`를 반환합니다.
    fn is_configuration_changed(&self) -> bool {
        self.user_config.as_ref().is_some_and(|user_config| {
            user_config.fullscreen != self.fullscreen || user_config.window_size != self.window_size
        })
    }

    /// 사용자 구성을 저장합니다.
    fn store_configuration(
        &mut self,
        asset_manager: &AssetManager,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 사용자 구성 설정 데이터를 변경합니다.
        let user_config = self
            .user_config
            .as_mut()
            .expect("user configuration must exist");
        user_config.fullscreen = self.fullscreen;
        user_config.window_size = self.window_size;

        // 파일 데이터를 가져옵니다.
        let data = serde_json::to_vec_pretty(&user_config)
            .map_err(|e| InvalidConfig(e))
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        // 사용자 구성 설정 파일에 데이터를 저장합니다.
        asset_manager
            .store(USER_CONFIG, &data)
            .map(|_| ())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    /// 사용자 인터페이스 콜백 함수입니다.
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        let client_id: u32 = self.client_id.into();
        let scale_factor = window.scale_factor() as f32;
        let (width, _): (f32, f32) = window.inner_size().into();

        let title_text = egui::RichText::new("Hello2Halo (개발자 모드)")
            .color(egui::Color32::WHITE)
            .size(24.0);
        let fullscreen_option_text = egui::RichText::new("전체 화면")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let window_size_option_text = egui::RichText::new("창 크기")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let character_option_text = egui::RichText::new("캐릭터 선택")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let save_button_text = egui::RichText::new("설정 저장")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let join_button_text = egui::RichText::new("게임 월드 입장")
            .color(egui::Color32::WHITE)
            .size(18.0);
        let client_id_text = egui::RichText::new(format!("클라이언트 ID: {}", client_id))
            .color(egui::Color32::WHITE)
            .size(12.0);

        self.fullscreen = app.is_fullscreen();
        self.window_size = app.window_size();
        let mut pressed_save_button = false;
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
                            ui.label(fullscreen_option_text);
                            ui.checkbox(&mut self.fullscreen, "");
                            ui.end_row();

                            ui.label(window_size_option_text);
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
                        ui.add_enabled_ui(self.is_configuration_changed(), |ui| {
                            if ui.button(save_button_text).clicked() {
                                pressed_save_button = true;
                            }
                        });
                    });

                    let ui = &mut cols[1];
                    egui::Grid::new("testbed")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(character_option_text);
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{}", self.character_kind.to_string()))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.character_kind,
                                        CharacterKind::ArisOriginal,
                                        CharacterKind::ArisOriginal.to_string(),
                                    );
                                });
                        });

                    ui.separator();
                    ui.label(client_id_text);
                    ui.horizontal(|ui| {
                        if ui.button(join_button_text).clicked() {
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

        if change_scene && self.user_config.is_some() {
            if let Some(user_config) = self.user_config.take() {
                let next_scene =
                    EnterStageScene::new(user_config, self.client_id, self.character_kind);
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                proxy.send_event(event).unwrap();
            }
        }

        if pressed_save_button {
            self.store_configuration(app.asset_manager())?;
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
        //! UI를 띄웁니다.
        //!
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
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
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
