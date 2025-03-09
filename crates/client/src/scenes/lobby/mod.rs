mod enter;
mod exit;

use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::components::{LoginToken, UserId};
use mod_render::UiRenderer;
use winit::window::Window;

pub use self::{enter::*, exit::*};

pub struct MainLobbyScene {
    /// 현재 클라이언트의 사용자 식별자입니다.
    user_id: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,
}

impl MainLobbyScene {}

impl GameScene for MainLobbyScene {
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
