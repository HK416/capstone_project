use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::{
    asset::{TexturePool, TextureViewPool},
    config::Locale,
    scenes::{
        FatalErrorSceneLayer, GameLoginTitleScene, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
};

/// 게임 인트로 화면을 보여주는 장면입니다.  
/// 클라이언트 데이터 무결성 검사를 진행합니다. (현재 이 기능은 작동하지 않습니다)
pub struct GameIntroVerifyScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 클라이언트 데이터가 유효한지 여부
    is_validate: bool,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl GameIntroVerifyScene {
    /// 새로운 `GameIntroVerifyScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
    ) -> Self {
        Self {
            locale,
            is_validate: false,
            texture_pool,
            texture_view_pool,
        }
    }
}

impl GameScene for GameIntroVerifyScene {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, _app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // TODO: 현재 클라이언트 데이터 무결성 검사를 실행하고 있지 않습니다.
        log::warn!("현재 클라이언트 데이터 무결성 검사를 실행하고 있지 않습니다.");
        self.is_validate = true;
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 클라이언트가 유효한 경우 다음 게임 장면으로 전환합니다.
        if self.is_validate {
            let next_scene = GameLoginTitleScene::new(
                self.locale,
                self.texture_pool.clone(),
                self.texture_view_pool.clone(),
            );
            let scene_flow = GameSceneFlow::Reset(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn ui_callback(&mut self, _window: &Window, _app: &dyn AppHandle) {
        /* empty */
    }
}
