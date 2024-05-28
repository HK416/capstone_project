//! 클라이언트 애플리케이션의 게임 루프와 관련된 코드를 작성합니다.
//!  

use super::error::AppError;

use std::sync::Arc;
use std::marker::PhantomData;
use hashbrown::HashMap;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
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
    pub fn run(event_loop: EventLoop<T>) -> Result<(), AppError> {
        let mut app = App {
            windows: HashMap::with_capacity(1),
            _phantom: PhantomData,
        };
        
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(&mut app).map_err(|e| {
            AppError::from(e)
        })
    }

    /// 애플리케이션에 새로운 윈도우를 추가합니다.
    /// 
    /// ※ 사용하는 윈도우는 1개이지만, 운영체제에 따라 이 함수를 여러번 호출할 수 있습니다. (예: Android)
    /// 
    fn regist_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Arc<Window>, AppError> { 
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes()
            .with_title("Hello to Halo")
            .with_visible(true)
            .with_resizable(false);

        let window: Arc<Window> = event_loop
            .create_window(attributes)
            .map_err(|e| AppError::from(e))?
            .into();

        self.windows.insert(
            window.id(), 
            AppWindow { 
                window: window.clone(),
            }
        );
        
        log::info!("Created new window (ID: {:?})", window.id());
        Ok(window)
    }
}

impl<T: 'static> ApplicationHandler<T> for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("resumed 호출됨!");
        self.regist_window(event_loop).unwrap(); // FIXME: 현재는 오류가 발생할 경우 프로그램을 중단시키자.
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // 등록된 윈도우를 가져오고 없을 경우 함수 실행을 생략한다.
        let app_window = match self.windows.get_mut(&window_id) {
            Some(app_window) => app_window,
            None => return,
        };

        match event {
            WindowEvent::Resized(size) => {
                app_window.on_resized(size);
            },
            WindowEvent::RedrawRequested => {
                app_window.on_draw();
            },
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
            },
            _ => { /* empty */ }
        };
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 등록된 윈도우가 하나도 없는 경우 애플리케이션을 종료한다.
        if self.windows.is_empty() {
            event_loop.exit();
            return;
        }

        // 등록된 윈도우가 존재할 경우 윈도우를 갱신한다.
        for window in self.windows.values().map(|app_window| &app_window.window) {
            window.request_redraw();
        }
    }
}



/// 클라이언트 애플리케이션의 각 창을 관리합니다.
#[derive(Debug)]
pub struct AppWindow {
    /// `winit`라이브러리의 핸들 입니다.
    window: Arc<Window>,
}

impl AppWindow {
    /// 새로운 창으로 부터 필요한 데이터를 초기화 합니다.
    fn new<T: 'static>(app: &App<T>, window: Window) -> Result<Self, AppError> {
        // TODO: --- 창으로 부터 필요한 데이터를 초기화 ---

        let this = Self {
            window: Arc::new(window),
        };

        Ok(this)
    }

    /// 현재 창의 크기가 변경되었을 때 호출되는 함수입니다.
    fn on_resized(&mut self, size: PhysicalSize<u32>) {
        // TODO: --- 창 크기에 맞춰 스왑체인 재설정 ---
    }

    /// 현재 창을 다시 그려야할 때 호출되는 함수입니다.
    fn on_draw(&mut self) {
        // 다음 프레임을 그릴 준비가 되었음을 알립니다.
        self.window.pre_present_notify();

        // TODO: --- 렌더링 ---
    }
}
