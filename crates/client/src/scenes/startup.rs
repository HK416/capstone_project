use std::{collections::VecDeque, error::Error, fmt, net::ToSocketAddrs, thread::JoinHandle};

use mod_app::{app::AppHandle, etc::AppEvent, net::IpAddress, scene::{GameScene, GameSceneFlow}};
use mod_network::{InitPacket, PacketType, RawPacket};
use winit::window::Window;

use super::TestBedScene;


type Task = JoinHandle<Result<(), Box<dyn Error + Send>>>;



/// 게임을 초기화 하는 장면입니다.
/// 게임 모델을 불러오거나 게임 서버와 연결을 하는 작업을 수행합니다.
/// 
pub struct StartupScene {
    /// 서버의 주소입니다.
    address: IpAddress, 

    /// 현재 실행 중인 작업 목록입니다.
    running_task: VecDeque<Task>, 
}

impl StartupScene {
    /// 새로운 `StartupScene`을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let addr = "localhost:7878".to_socket_addrs().unwrap().next().unwrap();
        let addr = IpAddress::Tcp(addr);

        Self { 
            address: addr, 
            running_task: VecDeque::with_capacity(8),
        }
    }
}

impl GameScene for StartupScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let addr = self.address.clone();
        let network = app.network().clone();
        self.running_task.push_back(std::thread::spawn(move || {
            network.connect(&addr)
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
            Ok(())
        }));

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_received_packet(
        &mut self, 
        raw_packet: RawPacket, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // Init Packet에서 데이터를 수집하고 다음 게임 장면으로 전환합니다.
        if raw_packet.packet_type() == PacketType::INIT {
            let packet = InitPacket::from_raw(raw_packet);
            app.event_loop_proxy().send_event(
                AppEvent::SetGameSceneFlow(GameSceneFlow::Change(
                    Box::new(TestBedScene::new(
                        self.address, 
                        packet.client_id, 
                        packet.world
                    ))
                ))
            ).unwrap();
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 처리할 작업이 없는 경우 함수 실행을 생략합니다.
        if self.running_task.is_empty() {
            return Ok(())
        }

        let mut temp = VecDeque::with_capacity(8);
        while let Some(task) = self.running_task.pop_front() {
            // 작업이 끝난 경우 스레드를 `join` 합니다.
            if task.is_finished() {
                task.join().unwrap()?;
            } else {
                temp.push_back(task);
            }
        }
        self.running_task.append(&mut temp);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        app: &dyn AppHandle
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
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), 
                            store: wgpu::StoreOp::Store
                        }, 
                        view: render_target_view, 
                        resolve_target: None
                    }),
                ], 
                depth_stencil_attachment: None, 
                timestamp_writes: None, 
                occlusion_query_set: None
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
