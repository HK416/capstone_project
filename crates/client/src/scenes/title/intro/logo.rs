use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::CharacterKind;
use mod_render::UiRenderer;
use rodio::Sink;
use winit::window::Window;

use crate::{
    asset::{CV_SOUND_TITLE, GAME_LOGO_URI, SoundDataPool, TexturePool, TextureViewPool},
    config::Locale,
    scenes::BASE_WIDTH,
};

use super::GameIntroConnectScene;

/// 장면 지속 시간(초)
const SCENE_DURATION: f32 = 2.8;

/// 게임 인트로 화면을 보여주는 장면입니다.  
/// 하얀색 화면 중앙에 게임 로고를 표시합니다.
pub struct GameIntroLogoScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    /// 게임 로고 텍스처의 텍스처 식별자입니다.
    game_logo_texture_id: egui::load::SizedTexture,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl GameIntroLogoScene {
    /// 새로운 `GameIntroLogoScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        texture_pool: TexturePool,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            background_volume,
            effect_volume,
            voice_volume,
            elapsed_time_sec: 0.0,
            game_logo_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            texture_pool,
            texture_view_pool: TextureViewPool::new(),
            sound_data_pool,
        }
    }
}

impl GameScene for GameIntroLogoScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        // 게임 로고 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(GAME_LOGO_URI)
            .expect("Game_Logo texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 게임 로고 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id = ui_renderer.register_native_texture(
            app.render_device(),
            &texture,
            wgpu::FilterMode::Linear,
        );

        // 무작위 타이틀 보이스를 출력합니다.
        if let Some(mixer) = app.audio_mixer() {
            let character: CharacterKind = rand::random();
            let i = character as usize;
            let uri = CV_SOUND_TITLE[i];
            let decoded = self
                .sound_data_pool
                .get(uri)
                .expect("Character Title sound must be preloaded!");
            let source = decoded.as_source();
            let sink = Sink::connect_new(mixer);
            sink.set_volume(self.voice_volume as f32 / 255.0);
            sink.append(source);
            sink.play();
            sink.detach();
        }

        // 등록된 텍스처 정보를 저장합니다.
        self.game_logo_texture_id = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        // 등록된 텍스처를 해제합니다.
        ui_renderer.free_texture(&self.game_logo_texture_id.id);
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 게임 장면 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 다음 게임 장면으로 전환합니다.
        if self.elapsed_time_sec >= SCENE_DURATION {
            let next_scene = GameIntroConnectScene::new(
                self.locale,
                self.background_volume,
                self.effect_volume,
                self.voice_volume,
                self.texture_pool.clone(),
                self.texture_view_pool.clone(),
                self.sound_data_pool.clone(),
            );
            let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({:?})", &self)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        let ratio = self.game_logo_texture_id.size.x / self.game_logo_texture_id.size.y;
        let rect = egui::Rect::from_min_max(
            egui::pos2(
                clip_rect.min.x + (1280.0 - 512.0) * 0.5 * scale,
                clip_rect.min.y + (720.0 - 512.0 / ratio) * 0.5 * scale,
            ),
            egui::pos2(
                clip_rect.min.x + (1280.0 + 512.0) * 0.5 * scale,
                clip_rect.min.y + (720.0 + 512.0 / ratio) * 0.5 * scale,
            ),
        );

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                ui.shrink_clip_rect(clip_rect);
                egui::Image::new(self.game_logo_texture_id)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, rect);
            });
    }
}
