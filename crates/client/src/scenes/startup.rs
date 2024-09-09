use std::fmt;
use std::error::Error;
use std::io;
use std::io::BufReader;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::thread::JoinHandle;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::io::Read;

use hecs::World;
use mod_error::err_msg;
use mod_error::RuntimeError;
use mod_scene::AppHandle;
use mod_scene::GameScene;
use mod_scene::GameSceneFlow;
use mod_util::AppEvent;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use mod_network::PacketParser;

type Task = JoinHandle<Result<(), Box<dyn Error + Send>>>;



/// 게임을 초기화 하는 장면입니다.
/// 게임 모델을 불러오거나 게임 서버와 연결을 하는 작업을 수행합니다.
/// 
pub struct StartupScene {
    /// Tcp 소켓 입니다.
    stream: Arc<Mutex<Option<Arc<TcpStream>>>>, 

    /// 현재 실행 중인 작업 목록입니다.
    running_task: VecDeque<Task>, 

    /// 총 작업의 갯수 입니다.
    total_task_num: usize, 

    /// 작업이 완료되었는지 나타냅니다.
    finished: bool, 
}

impl StartupScene {
    /// 새로운 `StartupScene`을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { 
            stream: Arc::new(Mutex::new(None)), 
            running_task: VecDeque::with_capacity(8), 
            total_task_num: 0, 
            finished: false, 
        }
    }
}

impl GameScene for StartupScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 수행할 작업을 추가합니다.
        // 현재는 새로운 스레드를 생성하여 
        // 주어진 IP 주소로 서버에 연결하는 작업만 수행합니다.
        // 
        use std::thread;
        let addr = "localhost:7878".to_socket_addrs().unwrap().next().unwrap();
        let proxy_cloned = app.event_loop_proxy().clone();
        let future = self.stream.clone();
        self.running_task.push_back(thread::spawn(move || {
            let stream = connect_and_run(proxy_cloned, addr)?;
            let mut future_guard = future.lock().unwrap();
            *future_guard = Some(stream);
            Ok(())
        }));
        self.total_task_num += 1;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 모든 작업이 완료된 경우 함수 실행을 생략합니다.
        if self.finished {
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

        if temp.is_empty() {
            // 모든 작업이 완료된 경우 다음 게임 장면으로 전환합니다.
            self.finished = true;
            let stream = {
                let mut guard = self.stream.lock().unwrap();
                guard.take().unwrap()
            };
            app.set_scene_flow(GameSceneFlow::Change(
                Box::new(super::TestBedScene::new(stream))
            ));
        } else {
            self.running_task.append(&mut temp);
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        world: &mut World, 
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
        .map_err(|e| err_msg!(e))?
        .into();

    // 별도의 스레드에서
    // 네트워크 패킷 수신 루프를 실행합니다.
    use std::thread;
    let stream_cloned = stream.clone();
    thread::spawn(move || network_loop(proxy, stream_cloned));

    Ok(stream)
}

/// 주어진 주소로 네트워크를 연결합니다.
fn connect_to_server(addr: SocketAddr) -> Result<TcpStream, io::Error> {
    // `std::net::TcpStream`의 connect 함수를 사용하여 네트워크에 연결합니다.
    // 네트워크 연결에 실패한 경우 `std::io::Error`를 반환합니다.
    // 

    TcpStream::connect(addr)
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

    
    let mut buffer = [0; 1024]; 

    let mut parser = PacketParser::new(); 
    let mut server_stream = BufReader::new(stream.as_ref());
    
    loop {
        // tcp stream 에서 읽어들이기 시도

        
        match server_stream.read(&mut buffer){
            Ok(0) => {
                println!("connection closed");
            },
            Ok(n) =>{
                parser.push(&buffer[..n]);
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {

            },
            Err(_) => {

            }
        }




        
    }
}
