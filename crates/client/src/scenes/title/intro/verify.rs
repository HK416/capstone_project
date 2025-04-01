use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::{TexturePool, TextureViewPool};
use winit::window::Window;

use crate::{
    asset::GAME_LOGO_URI,
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, GameLoginTitleScene},
};

/// 게임 인트로 화면을 보여주는 장면입니다.  
/// 클라이언트 데이터 무결성 검사를 진행합니다. (현재 이 기능은 작동하지 않습니다)
pub struct GameIntroVerifyScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 클라이언트 데이터가 유효한지 여부
    is_validate: bool,

    /// 게임 로고 텍스처의 텍스처 식별자입니다.
    game_logo_texture_id: egui::load::SizedTexture,
}

impl GameIntroVerifyScene {
    /// 새로운 `GameIntroVerifyScene`을 생성합니다.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            is_validate: false,
            game_logo_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
        }
    }
}

impl GameScene for GameIntroVerifyScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
        // TODO: 현재 클라이언트 데이터 무결성 검사를 실행하고 있지 않습니다.
        log::warn!("현재 클라이언트 데이터 무결성 검사를 실행하고 있지 않습니다.");
        self.is_validate = true;

        // 게임 로고 텍스처를 가져옵니다.
        let texture =
            TexturePool::get(GAME_LOGO_URI).expect("Game_Logo texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 게임 로고 텍스처의 텍스처 뷰를 생성합니다.
        let texture =
            TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let mut egui_renderer = app.egui_renderer_mut();
        let texture_id = egui_renderer.register_native_texture(
            app.render_device(),
            &texture,
            wgpu::FilterMode::Linear,
        );

        // 등록된 텍스처 정보를 저장합니다.
        self.game_logo_texture_id = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    fn on_exit(&mut self, _window: Option<&Window>, app: &dyn AppHandle) {
        // 등록된 텍스처를 해제합니다.
        let mut egui_renderer = app.egui_renderer_mut();
        egui_renderer.free_texture(&self.game_logo_texture_id.id);
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
        let title = ERR_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결이 끊겼습니다!"];
                ERR_MSG_TEXTS[i]
            }
            NetworkError::IO(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] =
                    ["패킷을 읽는 도중 오류가 발생했습니다!"];
                ERR_MSG_TEXTS[i]
            }
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::SetGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 클라이언트가 유효한 경우 다음 게임 장면으로 전환합니다.
        if self.is_validate {
            let next_scene = Box::new(GameLoginTitleScene::new(self.locale));
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_draw(
        &self,
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
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;

        let ratio = self.game_logo_texture_id.size.x / self.game_logo_texture_id.size.y;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let img_width = width * 0.3;
        let img_height = img_width / ratio;

        let rect = egui::Rect {
            min: egui::pos2(
                (center_x - 0.5 * img_width) / scale_factor,
                (center_y - 0.5 * img_height) / scale_factor,
            ),
            max: egui::pos2(
                (center_x + 0.5 * img_width) / scale_factor,
                (center_y + 0.5 * img_height) / scale_factor,
            ),
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                egui::Image::new(self.game_logo_texture_id).paint_at(ui, rect);
            });
    }
}
