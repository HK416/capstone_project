use std::{error::Error, fmt};

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::components::ClientId;
use mod_render::UiRenderer;
use winit::window::Window;

use crate::config::UserConfig;

/// ## IntroScene
/// 1. 게임 로고와 Blue Archive 2차 저작물 안내사항을 표시합니다.
///
/// 2. 게임 서버와 연결을 시도합니다.
///
/// 3. 클라이언트 에셋 유효성을 검사합니다. (추후)
///
pub struct IntroScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,

    /// 클라이언트 식별자입니다.
    client_id: ClientId,
}

impl IntroScene {
    /// 새로운 인트로 게임 장면을 생성합니다.
    pub fn new(user_config: Box<UserConfig>) -> Self {
        Self {
            user_config: Some(user_config),
            client_id: ClientId::NULL,
        }
    }
}

impl GameScene for IntroScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        window.set_visible(true);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 게임 인트로 화면을 보여줍니다.
        //! 현재는 임시로 검은색 화면을 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(IntroScene)"),
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

impl fmt::Debug for IntroScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(IntroScene))
    }
}
