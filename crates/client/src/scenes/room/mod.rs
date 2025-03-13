//! 커스텀 게임 장면과 관련된 코드를 작성합니다.
//!
mod enter;
mod exit;

use std::error::Error;

use hecs::World;
use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::components::{CustomGamePlayer, LoginToken, UserId, WorldId};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::config::UserConfig;

pub use self::{enter::*, exit::*};

/// 커스텀 게임 대기실 장면입니다.
pub struct CustomGameRoomScene {
    /// 현재 클라이언트의 사용자 식별자입니다.
    user_id: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 엔터티 월드입니다.
    world: World,

    /// 커스텀 게임 대기실의 월드 식별자입니다.
    world_id: WorldId,
    /// 현재 커스텀 게임에 참가한 플레이어 목록입니다.  
    /// 사용자 식별자의 오름차순으로 정렬됩니다.
    players: Vec<CustomGamePlayer>,
}

impl CustomGameRoomScene {
    /// 새로운 `CustomGameRoomScene`을 생성합니다.
    ///
    /// # Panics
    /// `UserId` 또는 `LoginToken`이 NULL인 경우 `panic!`을 호출합니다.
    ///
    pub fn new<I>(world: World, world_id: WorldId, iter: I) -> Self
    where
        I: IntoIterator<Item = CustomGamePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        let config = UserConfig::get();
        let user_id = config.info.uid;
        let token = config.token;
        drop(config);

        assert_ne!(user_id, UserId::NULL, "invalid user identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            user_id,
            token,
            world,
            world_id,
            players: iter.into_iter().collect(),
        }
    }
}

impl GameScene for CustomGameRoomScene {
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
