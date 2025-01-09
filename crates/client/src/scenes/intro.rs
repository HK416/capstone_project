use std::{error::Error, fmt};

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetManager,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{components::ClientId, ConnectPacket, PacketType, RawPacket};
use mod_render::UiRenderer;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{channel::TaskResultChannel, config::UserConfig, SERVER_ADDR};

use super::TestbedTitleScene;

/// ## IntroScene
/// 1. 게임 로고와 Blue Archive 2차 저작물 안내사항을 표시합니다.
///
/// 2. 게임 서버와 연결을 시도합니다.
///
/// 3. 클라이언트 에셋 유효성을 검사합니다. (추후)
///
/// # Note
/// 현재는 서버에 연결하여 클라이언트 식별자를 가져옵니다.
///
pub struct IntroScene {
    /// 사용자 구성 설정 데이터
    user_config: Option<Box<UserConfig>>,

    /// 클라이언트 식별자
    client_id: ClientId,

    /// 작업 결과 채널
    task_result_channel: TaskResultChannel,

    /// 작업의 개수
    num_task: usize,
}

impl IntroScene {
    /// 새로운 인트로 게임 장면을 생성합니다.
    pub fn new(user_config: Box<UserConfig>) -> Self {
        Self {
            user_config: Some(user_config),
            client_id: ClientId::NULL,
            task_result_channel: TaskResultChannel::new(),
            num_task: 0,
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

        // 게임 서버에 연결을 시도합니다.
        let pool = app.io_threads();
        let channel = self.task_result_channel.clone();
        let net_manager = app.net_manager().clone();
        connect_game_server(pool, channel, net_manager);
        self.num_task += 1;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 기다립니다.
        if let Some(result) = self.task_result_channel.recv() {
            self.num_task -= 1;
            result?;
        }

        // 현재는 클라이언트 식별자를 할당 받은 경우 다음 게임 장면으로 전환합니다.
        if self.num_task == 0 && self.client_id != ClientId::NULL {
            if let Some(user_config) = self.user_config.take() {
                let next_scene = Box::new(TestbedTitleScene::new(user_config, self.client_id));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let proxy = app.event_loop_proxy();
                proxy.send_event(event).unwrap();
            }
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // `Connect` 패킷을 수신하고 클라이언트 식별자를 저장합니다.
        assert_eq!(packet.packet_type(), PacketType::Connect, "invalid packet");
        let connect_packet = ConnectPacket::from_raw(packet);
        self.client_id = connect_packet.client_id;

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

/// 주어진 스레드 풀에서 게임 서버에 연결합니다.
/// 네트워크 연결 결과를 주어진 작업 결과 채널로 전송합니다.
fn connect_game_server(pool: &ThreadPool, channel: TaskResultChannel, net_manager: NetManager) {
    pool.spawn(move || {
        channel.send(net_manager.connect(&SERVER_ADDR));
    });
}
