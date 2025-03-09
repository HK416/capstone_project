mod intro;
mod login;

use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::UiRenderer;
use winit::window::Window;

pub use self::{intro::*, login::*};

/// 게임 타이틀 화면을 보여주는 장면입니다.
pub struct GameTitleScene {}

impl GameTitleScene {}

impl GameScene for GameTitleScene {
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
