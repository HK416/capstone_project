use std::{error::Error, fmt, net::ToSocketAddrs, sync::Arc};

use mod_app::{app::AppHandle, etc::AppEvent, net::IpAddress, scene::{GameScene, GameSceneFlow}};
use mod_network::{InitPacket, PacketType, Player, RawPacket};
use mod_parallelism::collections::Queue;
use winit::window::Window;

use super::TestBedScene;



/// 게임을 초기화 하는 장면입니다.
/// 게임 모델을 불러오거나 게임 서버와 연결을 하는 작업을 수행합니다.
/// 
pub struct StartupScene {
    /// 서버의 주소입니다.
    address: IpAddress, 

    client_id: u32, 
    world: Vec<Player>, 

    /// 작업의 갯수입니다.
    num_tasks: usize, 

    /// 실행이 완료된 작업 목록입니다.
    results: Arc<Queue<Result<(), Box<dyn Error + Send>>>>, 
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
            client_id: 0, 
            world: Vec::new(), 
            num_tasks: 0, 
            results: Arc::new(Queue::new()), 
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
        self.num_tasks = 4;
        let io_threads = app.io_threads();

        // 네트워크 연결
        let addr = self.address.clone();
        let network = app.network().clone();
        let cloned_results = self.results.clone();
        io_threads.spawn(move || {
            let result = network.connect(&addr)
                .map(|_| ())
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            cloned_results.push(result);
        });

        // `Aris_Original` 에셋 로드
        let bundle = app.bundle().clone();
        let cloned_results = self.results.clone();
        io_threads.spawn(move || {
            let result = bundle.load("characters/aris_original/Aris_Original_Mesh.ron")
                .map(|_| ())
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            cloned_results.push(result);
        });


        // `Sphere` 에셋 로드
        let bundle = app.bundle().clone();
        let cloned_results = self.results.clone();
        io_threads.spawn(move || {
            let result = bundle.load("shape/sphere/Sphere.ron")
                .map(|_| ())
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
            cloned_results.push(result);
        });


        // 폰트 추가
        let bundle = app.bundle().clone();
        let egui_ctx = app.egui_ctx().clone();
        let cloned_results = self.results.clone();
        io_threads.spawn(move || {
            // 폰트 파일을 로드합니다.
            let font_data = match bundle.load("font/NEXON Lv2 Gothic.ttf") {
                Ok(cache) => cache.as_bytes().to_vec(), 
                Err(e) => return cloned_results.push(Err(Box::new(e))),
            };
                
            // 폰트 데이터를 추가합니다.
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "NEXON Lv2 Gothic".to_owned(), 
                egui::FontData::from_owned(font_data)
            );
            
            fonts.families.entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "NEXON Lv2 Gothic".to_owned());

            fonts.families.entry(egui::FontFamily::Monospace)
                .or_default()
                .push("NEXON Lv2 Gothic".to_owned());

            egui_ctx.set_fonts(fonts);
            cloned_results.push(Ok(()));
        });


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
            self.client_id = packet.client_id;
            self.world = packet.world;
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
        if self.num_tasks == 0 {
            return Ok(())
        }

        if let Some(result) = self.results.pop() {
            self.num_tasks -= 1;
            result?;
        }

        if self.num_tasks == 0 {
            let mut players = Vec::new();
            players.append(&mut self.world);
            app.event_loop_proxy().send_event(
                AppEvent::SetGameSceneFlow(GameSceneFlow::Change(
                    Box::new(TestBedScene::new(
                        self.address, 
                        self.client_id, 
                        players
                    ))
                ))
            ).unwrap();
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        window: &Window, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        egui_renderer: &egui_wgpu::Renderer, 
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
