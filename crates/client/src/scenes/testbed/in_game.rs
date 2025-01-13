use std::{error::Error, fmt};

use ahash::HashMap;
use hecs::{Entity, World};
use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::components::{ClientId, ObjectId};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::config::UserConfig;

pub struct TestbedInGameScene {
    /// 사용자 설정 구성 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,

    /// 게임 월드
    world: World,
    /// 게임 월드 엔터티 목록
    entities: HashMap<ObjectId, Entity>,
}

impl TestbedInGameScene {
    /// 새로운 `TestbedInGameScene`을 생성합니다.
    ///
    /// # Panics
    /// 주어진 클라이언트 식별자가 유효하지 않는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        user_config: Box<UserConfig>,
        client_id: ClientId,
        world: World,
        entities: HashMap<ObjectId, Entity>,
    ) -> Self {
        assert_ne!(client_id, ClientId::NULL, "invalid client id");
        Self {
            user_config: Some(user_config),
            client_id,
            world,
            entities,
        }
    }
}

impl GameScene for TestbedInGameScene {
    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 검은색 화면에 오른쪽 하단에 상태를 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TestbedInGameScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }
}

impl fmt::Debug for TestbedInGameScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestbedInGameScene))
    }
}
