mod enter;

use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::components::{LoginToken, UserId};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::config::Locale;

pub use self::enter::*;

pub struct MainLobbyScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 클라이언트의 사용자 식별자입니다.
    user_id: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,
}

impl MainLobbyScene {
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

impl GameScene for MainLobbyScene {}
