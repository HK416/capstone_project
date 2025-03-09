use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::UiRenderer;
use winit::window::Window;

/// 커스텀 게임 대기실에 입장할 때 에셋을 로드하는 장면입니다.
pub struct CustomGameEnterScene {}

impl CustomGameEnterScene {}

impl GameScene for CustomGameEnterScene {
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
