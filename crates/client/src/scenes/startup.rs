use std::{
    collections::VecDeque, 
    error::Error, 
    fmt, 
    io::{self, BufReader, Read}, 
    net::{SocketAddr, TcpStream}, 
    sync::{Arc, Mutex}, 
    thread::JoinHandle
};

use mod_app::{
    app::AppHandle, 
    etc::AppEvent, 
    scene::{GameScene, GameSceneFlow}
};
use mod_network::{InitPacket, PacketParser, PacketType, RawPacket};
use winit::{event_loop::EventLoopProxy, window::Window};

use super::TestBedScene;


type Task = JoinHandle<Result<(), Box<dyn Error + Send>>>;



/// 게임을 초기화 하는 장면입니다.
/// 게임 모델을 불러오거나 게임 서버와 연결을 하는 작업을 수행합니다.
/// 
pub struct StartupScene {
    /// Tcp 소켓 입니다.
    stream: Arc<Mutex<Option<Arc<TcpStream>>>>, 

    /// 현재 실행 중인 작업 목록입니다.
    running_task: VecDeque<Task>, 
}

impl StartupScene {
    /// 새로운 `StartupScene`을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { 
            stream: Arc::new(Mutex::new(None)), 
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
        let addr = app.address().clone();
        let proxy_cloned = app.event_loop_proxy().clone();
        let future = self.stream.clone();
        self.running_task.push_back(std::thread::spawn(move || {
            let stream = connect_and_run(proxy_cloned, addr)?;
            let mut future_guard = future.lock().unwrap();
            *future_guard = Some(stream);
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
            let stream = {
                let mut guard = self.stream.lock().unwrap();
                guard.take().unwrap()
            };
            
            app.event_loop_proxy().send_event(
                AppEvent::SetGameSceneFlow(GameSceneFlow::Change(
                    Box::new(TestBedScene::new(
                        stream, 
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



/// 주어진 주소로 서버를 연결하고 네트워크 패킷 수신 루프를 실행합니다.
fn connect_and_run(
    proxy: Arc<EventLoopProxy<AppEvent>>, 
    addr: SocketAddr
) -> Result<Arc<TcpStream>, Box<dyn Error + Send>> {
    // 서버에 연결합니다.
    let stream: Arc<TcpStream> = connect_to_server(addr)
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?
        .into();

    // 별도의 스레드에서
    // 네트워크 패킷 수신 루프를 실행합니다.
    let stream_cloned = stream.clone();
    std::thread::spawn(move || network_loop(proxy, stream_cloned));

    Ok(stream)
}

/// 주어진 주소로 네트워크를 연결합니다.
fn connect_to_server(addr: SocketAddr) -> Result<TcpStream, io::Error> {
    // `std::net::TcpStream`의 connect 함수를 사용하여 네트워크에 연결합니다.
    // 네트워크 연결에 실패한 경우 `std::io::Error`를 반환합니다.
    // 

    let stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;
    stream.set_nonblocking(true)?;
    return Ok(stream);
}

/// 네트워크 패킷 수신 루프입니다.
fn network_loop(
    proxy: Arc<EventLoopProxy<AppEvent>>, 
    stream: Arc<TcpStream>
) {
    // 수신한 데이터를 `mod_network::PacketParser`를 통해 구문 분석한 후
    // EventLoopProxy를 통해 이벤트 루프로 패킷 수신 이벤트를 보냅니다.
    //
    // 만약 수신 중 오류가 발생할 경우 오류 메시지를 이벤트 루프로 보내고
    // 패킷 수신 루프를 빠져나옵니다.
    //
    let mut parser = PacketParser::new(); 
    let mut server_stream = BufReader::new(stream.as_ref());
    
    'recv: loop {
        // tcp stream 에서 읽어들이기 시도

        let mut buffer = [0; 1024]; 
        match server_stream.read(&mut buffer){
            Ok(0) => if proxy.send_event(AppEvent::ClosedSocket).is_err() {
                break 'recv;
            },
            Ok(n) =>{
                parser.push(&buffer[..n]);
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue 'recv;
            },
            Err(e) => if proxy.send_event(AppEvent::NetworkIOError(e)).is_err() {
                break 'recv;
            }
        }

        while let Some(raw_packet) = parser.pop() {
            if proxy.send_event(AppEvent::PacketReceived(raw_packet)).is_err() {
                break 'recv;
            }
        }
    }
}
