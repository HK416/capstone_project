mod locale;
mod window;

use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering as MemOrdering},
        Arc,
    },
};

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::{TexturePool, NOTOSANS_REGULAR, USER_CONFIG},
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::GameIntroNotifyScene,
};

pub use self::{locale::*, window::*};

/// 애플리케이션 표시 언어에 따른 로딩 텍스트입니다.
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["설정 저장 중..."];

/// 시스템에서 클라이언트를 처음 실행했을 때 사용자 구성을 설정하는 장면입니다.  
/// 설정을 저장합니다.
pub struct InitFinishScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 저장의 완료 여부
    completed: Arc<AtomicBool>,

    // 텍스처 풀 객체
    texture_pool: TexturePool,
}

impl InitFinishScene {
    /// 새로운 `InitFinishScene`을 생성합니다.
    pub fn new(texture_pool: TexturePool) -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale,
            completed: Arc::new(AtomicBool::new(false)),
            texture_pool,
        }
    }

    /// 사용자 구성을 저장합니다.
    fn store_user_config(&self, thread_pool: &ThreadPool, path: &Path) {
        let completed = self.completed.clone();
        let mut path = path.to_path_buf();
        path.push(format!("assets/{}", USER_CONFIG));
        thread_pool.spawn(move || {
            let _ = UserConfig::store_from_file(path);
            completed.store(true, MemOrdering::Release);
        });
    }
}

impl GameScene for InitFinishScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        self.store_user_config(app.io_threads(), app.current_dir());
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        if self.completed.load(MemOrdering::Acquire) {
            // 다음 게임 장면으로 전환합니다.
            let next_scene = GameIntroNotifyScene::new(self.locale, self.texture_pool.clone());
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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

    fn ui_callback(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 텍스트
        let text = LOAD_TEXTS[self.locale as usize];
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0, family);
        let loading_text = egui::RichText::new(text)
            .font(font_id.clone())
            .color(egui::Color32::WHITE);

        egui::Area::new(egui::Id::new("Layout_Loading"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [0.0, 0.0])
            .show(app.egui_ctx(), |ui| {
                ui.label(loading_text);
            });
    }
}
