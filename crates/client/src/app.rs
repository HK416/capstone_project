//! 클라이언트 애플리케이션의 게임 루프와 관련된 코드를 작성합니다.
//!  

use std::sync::Arc;
use std::marker::PhantomData;
use hashbrown::HashMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;
use winit::window::WindowId;



/// 클라이언트 애플리케이션을 관리합니다.
#[derive(Debug)]
pub struct App<T: 'static> {
    /// 생성된 `winit` 창 목록 입니다.
    /// ※ 창을 한개만 사용하지만, 만약에 경우에 대비함.
    /// 
    windows: HashMap<WindowId, AppWindow>,

    /// 사용자 정의 이벤트의 PhantomData
    _phantom: PhantomData<T>
}

impl<T: 'static> App<T> {
    /// 애플리케이션 이벤트 루프를 실행합니다.
    pub fn run(event_loop: EventLoop<T>) {
        let mut app = App {
            windows: HashMap::with_capacity(1),
            _phantom: PhantomData,
        };

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(&mut app);
    }
}

impl<T: 'static> ApplicationHandler<T> for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("resumed 호출됨!");
        event_loop.exit(); // 곧 바로 종료되도록 한다.
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent
    ) {
        todo!()
    }
}



/// 클라이언트 애플리케이션의 각 창을 관리합니다.
#[derive(Debug)]
pub struct AppWindow {
    /// `winit`라이브러리의 핸들 입니다.
    window: Arc<Window>,
}
