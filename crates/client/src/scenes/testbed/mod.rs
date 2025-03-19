mod enter;
mod in_game;

use std::{error::Error, fmt};

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{components::{CharacterKind, LoginToken, UserId, UserInfo, WorldId}, protocol::{CustomGameJoinRequestPacket, Packet}};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR, USER_CONFIG},
    config::UserConfig, SERVER_TCP_ADDR,
};

pub use {self::enter::*, self::in_game::*};

/// ## Testbed Title Scene
pub struct TestbedTitleScene {
    /// 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 선택된 전체 화면 여부
    is_fullscreen: bool,
    /// 선택된 윈도우 창 크기
    window_size: WindowSize,
    /// 선택된 캐릭터 종류
    character_kind: CharacterKind,

    /// 다음 장면의 진입 여부입니다.
    /// `true`일 경우 다음 장면으로 전환합니다.
    next_scene_transition_request: bool,
    /// 사용자 구성 파일 저장 여부입니다.
    /// `true`일 경우 사용자 구성 파일을 저장합니다.
    configuration_save_request: bool,

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedTitleScene {
    /// 새로운 `TestbedTitleScene`을 생성합니다.
    pub fn new() -> Self {
        let config = UserConfig::get();
        let user_id = config.info.uid;
        let token = config.token;
        drop(config);

        assert_ne!(user_id, UserId::NULL, "invalid user identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            user_id,
            token, 
            is_fullscreen: false,
            window_size: WindowSize::MAX,
            character_kind: CharacterKind::default(),
            next_scene_transition_request: false,
            configuration_save_request: false,
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 인터페이스 콜백 함수입니다.
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();

        let config = UserConfig::get();
        let name = config.info.name;
        let scale_factor = window.scale_factor() as f32;
        let (width, _): (f32, f32) = window.inner_size().into();

        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let head_font_id = egui::FontId::new(24.0, head_font_family);

        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let main_font_id = egui::FontId::new(18.0, main_font_family);

        let is_config_changed =
            config.is_fullscreen != self.is_fullscreen || config.window_size != self.window_size;
        let title_text = egui::RichText::new("Hello2Halo (개발자 모드)")
            .color(egui::Color32::WHITE)
            .font(head_font_id.clone());
        let fullscreen_option_text = egui::RichText::new("전체 화면")
            .color(egui::Color32::WHITE)
            .font(main_font_id.clone());
        let window_size_option_text = egui::RichText::new("창 크기")
            .color(egui::Color32::WHITE)
            .font(main_font_id.clone());
        let character_option_text = egui::RichText::new("캐릭터 선택")
            .color(egui::Color32::WHITE)
            .font(main_font_id.clone());
        let save_button_text = egui::RichText::new("설정 저장")
            .color(egui::Color32::WHITE)
            .font(main_font_id.clone());
        let join_button_text = egui::RichText::new("커스텀 게임 입장")
            .color(egui::Color32::WHITE)
            .font(main_font_id.clone());
        let client_id_text = egui::RichText::new(format!("사용자: {}", name))
            .color(egui::Color32::WHITE)
            .font(main_font_id.clone());

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
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
                            ui.checkbox(&mut self.is_fullscreen, "");
                            ui.end_row();

                            ui.label(window_size_option_text);
                            ui.add_enabled_ui(!self.is_fullscreen, |ui| {
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
                        ui.add_enabled_ui(is_config_changed, |ui| {
                            if ui.button(save_button_text).clicked() {
                                self.configuration_save_request = true;
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
                                    ui.selectable_value(
                                        &mut self.character_kind,
                                        CharacterKind::MomoiOriginal,
                                        CharacterKind::MomoiOriginal.to_string(),
                                    );
                                    ui.selectable_value(
                                        &mut self.character_kind,
                                        CharacterKind::MidoriOriginal,
                                        CharacterKind::MidoriOriginal.to_string(),
                                    );
                                    ui.selectable_value(
                                        &mut self.character_kind,
                                        CharacterKind::YuukaOriginal,
                                        CharacterKind::YuukaOriginal.to_string(),
                                    );
                                });
                        });

                    ui.separator();
                    ui.label(client_id_text);
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!self.next_scene_transition_request, |ui| {
                            if ui.button(join_button_text).clicked() {
                                self.next_scene_transition_request = true;
                                // 커스텀 게임 입장 요청 패킷을 전송합니다.
                                let packet = CustomGameJoinRequestPacket::new(
                                    WorldId::new(1), 
                                    self.user_id, 
                                    self.token
                                );
                                let net_manager = app.net_manager();
                                let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
                                socket.push_packet(packet.as_raw());
                            } 
                        });
                    });
                });
            });
        Ok(())
    }
}

impl GameScene for TestbedTitleScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 현재 사용자 구성 설정 값을 가져옵니다.
        let config = UserConfig::get();
        self.window_size = config.window_size;
        self.is_fullscreen = config.is_fullscreen;
        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let proxy = app.event_loop_proxy();
        if self.is_fullscreen != app.is_fullscreen() {
            proxy
                .send_event(AppEvent::FullScreenRequest(self.is_fullscreen))
                .unwrap();
        }

        if self.window_size != app.window_size() {
            proxy
                .send_event(AppEvent::ResizeRequest(self.window_size))
                .unwrap();
        }

        if self.configuration_save_request {
            let mut config = UserConfig::get();
            config.window_size = self.window_size;
            config.is_fullscreen = self.is_fullscreen;
            drop(config);

            let mut path = app.asset_manager().get_root_dir().to_path_buf();
            path.push(USER_CONFIG);
            let _ = UserConfig::store_from_file(path)
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

            self.configuration_save_request = false;
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
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(UI(TestbadTitleScene))"),
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
