use std::{
    error::Error,
    fmt,
    io::{Cursor, ErrorKind},
};

use mod_app::{
    app::AppHandle,
    etc::{AppEvent, NoSuitableWndSize, WindowSize},
    scene::{GameScene, GameSceneFlow},
};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::config::{InvalidConfig, UserConfig};

use super::DraftScene;

/// ## Startup Scene
/// 게임을 실행하면 제일 먼저 진입하는 장면입니다.
///
/// `UserConfig` 파일을 읽고, 애플리케이션 창을 조정합니다.  
/// 시스템에서 파일을 찾을 수 없는 경우 초기 설정 장면으로 전환합니다.
///
pub struct StartupScene {}

impl StartupScene {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameScene for StartupScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let asset_manager = app.asset_manager();
        let result = asset_manager.get_or_init("user_config");
        let config = match result {
            Ok(cached_asset) => {
                let reader = Cursor::new(cached_asset.as_bytes());
                let config: UserConfig = serde_json::from_reader(reader)
                    .map_err(|e| InvalidConfig(e))
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

                config
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // 최대 윈도우 크기를 가져옵니다.
                let max_window_size = window
                    .primary_monitor()
                    .map(|monitor| WindowSize::find_maximize_size(monitor))
                    .flatten();
                let max_window_size = match max_window_size {
                    Some(size) => size,
                    None => return Err(Box::new(NoSuitableWndSize)),
                };

                // 사용자 구성 파일을 생성합니다.
                let config = UserConfig::new(max_window_size);
                let data = serde_json::ser::to_vec_pretty(&config)
                    .map_err(|e| InvalidConfig(e))
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
                asset_manager
                    .create("user_config", &data)
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

                config
            }
            Err(e) => return Err(Box::new(e)),
        };

        // 애플리케이션 창을 조정합니다.
        let proxy = app.event_loop_proxy();
        proxy.send_event(AppEvent::ResizeRequest(config.window_size)).unwrap();
        proxy.send_event(AppEvent::FullScreenRequest(config.fullscreen)).unwrap();

        // 다음 게임 장면으로 이동합니다.
        if config.locale.is_none() {
            proxy.send_event(AppEvent::SetGameSceneFlow(GameSceneFlow::Change(Box::new(DraftScene { })))).unwrap();
        } else {
            proxy.send_event(AppEvent::SetGameSceneFlow(GameSceneFlow::Change(Box::new(DraftScene { })))).unwrap();
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_stencil_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임을 초기화 하는 동안 검정색 화면을 출력합니다.
        //
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(StartupScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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

        queue.submit([encoder.finish()]);
        Ok(())
    }
}

impl fmt::Debug for StartupScene {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(StartupScene))
    }
}
