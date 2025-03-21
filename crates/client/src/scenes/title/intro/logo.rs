use std::error::Error;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::{TexturePool, TextureViewPool};
use winit::window::Window;

use crate::{asset::GAME_LOGO_URI, config::Locale};

use super::GameIntroConnectScene;

/// 장면 지속 시간(초)
const SCENE_DURATION: f32 = 2.8;

/// 게임 인트로 화면을 보여주는 장면입니다.  
/// 하얀색 화면 중앙에 게임 로고를 표시합니다.
pub struct GameIntroLogoScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    /// 게임 로고 텍스처의 텍스처 식별자입니다.
    game_logo_texture_id: egui::load::SizedTexture,
}

impl GameIntroLogoScene {
    /// 새로운 `GameIntroLogoScene`을 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            elapsed_time_sec: 0.0,
            game_logo_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
        }
    }
}

impl GameScene for GameIntroLogoScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 로고 텍스처를 가져옵니다.
        let texture =
            TexturePool::get(GAME_LOGO_URI).expect("Game_Logo texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 게임 로고 텍스처의 텍스처 뷰를 생성합니다.
        let texture =
            TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let mut egui_renderer = app.egui_renderer_mut();
        let texture_id = egui_renderer.register_native_texture(
            app.render_device(),
            &texture,
            wgpu::FilterMode::Linear,
        );

        // 등록된 텍스처 정보를 저장합니다.
        self.game_logo_texture_id = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };

        Ok(())
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 등록된 텍스처를 해제합니다.
        let mut egui_renderer = app.egui_renderer_mut();
        egui_renderer.free_texture(&self.game_logo_texture_id.id);

        Ok(())
    }

    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임 장면 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 다음 게임 장면으로 전환합니다.
        if self.elapsed_time_sec >= SCENE_DURATION {
            let next_scene = Box::new(GameIntroConnectScene::new(self.locale));
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
        Ok(())
    }

    fn on_draw(
        &self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder, 
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        {
            let _rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("RenderPass({})", stringify!(GameIntroNotifyScene))), 
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations { 
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), 
                            store: wgpu::StoreOp::Store 
                        },
                        view: render_target_view, 
                        resolve_target: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
        }
        Ok(())
    }

    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;

        let ratio = self.game_logo_texture_id.size.x / self.game_logo_texture_id.size.y;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let img_width = width * 0.3;
        let img_height = img_width / ratio;
        let rect = egui::Rect {
            min: egui::pos2(
                (center_x - 0.5 * img_width) / scale_factor,
                (center_y - 0.5 * img_height) / scale_factor,
            ),
            max: egui::pos2(
                (center_x + 0.5 * img_width) / scale_factor,
                (center_y + 0.5 * img_height) / scale_factor,
            ),
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                egui::Image::new(self.game_logo_texture_id).paint_at(ui, rect);
            });

        Ok(())
    }
}
