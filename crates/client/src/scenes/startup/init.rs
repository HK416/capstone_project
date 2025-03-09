use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::UiRenderer;
use winit::window::Window;

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성을 설정하는 장면입니다.
pub struct InitConfigScene {}

impl InitConfigScene {
    /// 새로운 `InitConfigScene`을 생성합니다.
    pub fn new() -> Self {
        Self {}
    }
}

impl GameScene for InitConfigScene {
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}
