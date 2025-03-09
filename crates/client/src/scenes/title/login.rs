use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::UiRenderer;
use winit::window::Window;

pub struct GameLoginScene {}

impl GameLoginScene {}

impl GameScene for GameLoginScene {
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
