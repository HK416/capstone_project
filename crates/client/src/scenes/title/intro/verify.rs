use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use rodio::Sink;
use winit::window::Window;

use crate::{
    asset::{SoundDataPool, TexturePool, TextureViewPool, UI_NOTICE},
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
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 클라이언트 데이터가 유효한지 여부
    is_validate: bool,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl GameIntroVerifyScene {
    /// 새로운 `GameIntroVerifyScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            background_volume,
            effect_volume,
            voice_volume,
            is_validate: false,
            texture_pool,
            texture_view_pool,
            sound_data_pool,
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

        // 효과음을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(UI_NOTICE)
            .expect("UI_Notice sound must be preloaded!");
        let source = decoded.as_source();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.effect_volume as f32 / 255.0);
        sink.append(source);
        sink.play();
        sink.detach();
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 클라이언트가 유효한 경우 다음 게임 장면으로 전환합니다.
        if self.is_validate {
            let next_scene = GameLoginTitleScene::new(
                self.locale,
                self.background_volume,
                self.effect_volume,
                self.voice_volume,
                self.texture_pool.clone(),
                self.texture_view_pool.clone(),
                self.sound_data_pool.clone(),
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
