use std::{error::Error, sync::Arc};

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::{NetManager, NetworkError},
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rayon::ThreadPool;
use rodio::Sink;
use winit::window::Window;

use crate::{
    asset::{SoundDataPool, TexturePool, TextureViewPool, NOTOSANS_REGULAR, UI_NOTICE},
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
    SERVER_TCP_ADDR,
};

use super::GameIntroVerifyScene;

/// 애플리케이션 표시 언어에 따른 게임 서버 연결 텍스트
const CONNECT_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결 중"];
/// 애플리케이션 표시 언어에 따른 게임 서버 연결 실패 타이틀 텍스트
const CONNECT_ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
/// 애플리케이션 표시 언어에 따른 게임 서버 연결 실패 메시지 텍스트
const CONNECT_ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결에 실패했습니다!"];

pub struct GameIntroConnectScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 작업 결과를 저장
    task_result: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl GameIntroConnectScene {
    /// 새로운 `GameIntroConnectScene`을 생성합니다.
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
            task_result: Arc::new(Queue::new()),
            texture_pool,
            texture_view_pool,
            sound_data_pool,
        }
    }

    /// 게임 서버와 연결을 시도합니다.
    fn try_connect_game_server(&mut self, thread_pool: &ThreadPool, net_manager: &NetManager) {
        let task_result = self.task_result.clone();
        let net_manager = net_manager.clone();
        thread_pool.spawn(move || {
            let result = net_manager
                .connect(&SERVER_TCP_ADDR)
                .map(|_| ())
                .map_err(|e| {
                    log::error!("failed to connect to game server! (REASON:{e})");
                    Box::new(e) as Box<dyn Error + Send>
                });
            task_result.push(result);
        });
    }
}

impl GameScene for GameIntroConnectScene {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        self.try_connect_game_server(app.io_threads(), app.net_manager());
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_result.pop() {
            // 오류를 확인합니다.
            let scene_flow = match result {
                Ok(_) => {
                    let next_scene = GameIntroVerifyScene::new(
                        self.locale,
                        self.background_volume,
                        self.effect_volume,
                        self.voice_volume,
                        self.texture_pool.clone(),
                        self.texture_view_pool.clone(),
                        self.sound_data_pool.clone(),
                    );
                    GameSceneFlow::Change(Box::new(next_scene))
                }
                Err(_) => {
                    let i = self.locale as usize;
                    let next_scene = FatalErrorSceneLayer::new(
                        self.locale,
                        self.background_volume,
                        self.effect_volume,
                        self.voice_volume,
                        CONNECT_ERR_TITLE_TEXTS[i],
                        CONNECT_ERR_MSG_TEXTS[i],
                        self.sound_data_pool.clone(),
                    );
                    GameSceneFlow::Push(Box::new(next_scene))
                }
            };

            // 다음 게임 장면으로 전환합니다.
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
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            title,
            message,
            self.sound_data_pool.clone(),
        );
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

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let locale = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 텍스트
        let text = CONNECT_TEXTS[locale];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(18.0 * scale, family);
        let connect_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::BLACK);
        let connect_label = egui::Label::new(connect_text)
            .sense(egui::Sense::empty())
            .selectable(false);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, (360.0 - 18.0) * scale])
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                ui.vertical_centered(|ui| {
                    ui.add(connect_label);
                })
            });
    }
}
