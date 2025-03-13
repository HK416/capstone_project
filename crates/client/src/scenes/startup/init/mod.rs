mod locale;
mod window;

use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering as MemOrdering},
        Arc,
    },
};

use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::{ScreenDescriptor, UiRenderer};
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_REGULAR, USER_CONFIG},
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::GameIntroNotifyScene,
};

pub use self::{locale::*, window::*};

/// 애플리케이션 표시 언어에 따른 로딩 텍스트입니다.
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["설정 저장 중..."];

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성을 설정하는 장면입니다.  
/// 설정을 저장합니다.
pub struct InitFinishScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 저장의 완료 여부
    completed: Arc<AtomicBool>,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl InitFinishScene {
    /// 새로운 `InitFinishScene`을 생성합니다.
    pub fn new() -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale,
            completed: Arc::new(AtomicBool::new(false)),
            egui_clip_primitives: Vec::default(),
            egui_free_texture_ids: Vec::default(),
        }
    }

    /// UI 콜백 함수
    fn ui_callback(&mut self, _window: &Window, egui_ctx: &egui::Context) {
        // 폰트 속성
        let font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0, font_family);
        let font_color = egui::Color32::WHITE;

        // 텍스트
        let text = LOAD_TEXTS[self.locale as usize];
        let loading_text = egui::RichText::new(text)
            .font(font_id.clone())
            .color(font_color.clone());

        egui::Area::new(egui::Id::new("Layout_0"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [0.0, 0.0])
            .show(egui_ctx, |ui| {
                ui.label(loading_text);
            });
    }

    /// 사용자 구성을 저장합니다.
    fn store_user_config(&self, thread_pool: &ThreadPool, asset_manager: &AssetManager) {
        let asset_manager = asset_manager.clone();
        let completed = self.completed.clone();
        thread_pool.spawn(move || {
            let mut path = asset_manager.get_root_dir().to_path_buf();
            path.push(USER_CONFIG);

            let _ = UserConfig::store_from_file(path);
            completed.store(true, MemOrdering::Release);
        });
    }
}

impl GameScene for InitFinishScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.store_user_config(app.io_threads(), app.asset_manager());
        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        if self.completed.load(MemOrdering::Acquire) {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = Box::new(GameIntroNotifyScene::new());
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
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
        self.ui_callback(window, egui_ctx);
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
                label: Some(&format!("RenderPass({})", stringify!(InitLocaleScene))),
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
