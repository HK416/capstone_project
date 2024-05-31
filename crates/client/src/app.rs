//! 클라이언트 애플리케이션의 게임 루프와 관련된 코드를 작성합니다.
//!  

use super::error::AppError;
use super::error::show_error_msg;

use std::panic;
use std::process;
use std::sync::Arc;
use std::marker::PhantomData;
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
    /// ※ 창을 한개만 사용한다고 가정함
    /// 
    window: Option<Arc<Window>>,

    /// 사용자 정의 이벤트의 PhantomData
    _phantom: PhantomData<T>
}

impl<T: 'static> App<T> {
    /// 애플리케이션 이벤트 루프를 실행합니다.
    pub fn run(event_loop: EventLoop<T>) -> Result<(), AppError> {
        let mut app = App {
            window: None,
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
    fn regist_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> { 
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes()
            .with_title("Hello to Halo")
            .with_visible(true)
            .with_resizable(false);

        let window = event_loop
            .create_window(attributes)
            .map_err(|e| AppError::from(e))?;

        let window_id = window.id();
        log::info!("Created new window (ID: {:?})", window_id);
        self.window = Some(window.into());

        Ok(())
    }
}

impl<T: 'static> ApplicationHandler<T> for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("resumed 호출됨!");
        
        if let Err(e) = self.regist_window(event_loop) {
            show_error_msg("Window Creation Failed", &e.to_string(), None);
            return event_loop.exit();
        }

        // `panic!` 호출시 처리를 설정합니다.
        // ※ winit에서 `resumed`를 호출하기 전까지 App<T>의 윈도우가 삭제되지 않음
        let window = self.window.clone();
        panic::set_hook(Box::new(move |info| {
            if let Some(location) = info.location() {
                log::debug!("Calling panic at - File:{}, Line:{}, Column:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }

            if let Some(text) = info.payload().downcast_ref::<&str>() {
                log::error!("{}", text.to_string());
                show_error_msg("Runtime Error", text, window.as_deref());
            }

            process::exit(-1);
        }))
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // 등록된 윈도우를 가져오고 없을 경우 함수 실행을 생략한다.
        let window = match &self.window {
            Some(window) if window.id() == window_id => {
                window
            },
            _ => return,
        };

        match event {
            WindowEvent::Resized(size) => {
                
            },
            WindowEvent::RedrawRequested => {
                
            },
            WindowEvent::CloseRequested => {
                return event_loop.exit();
            },
            _ => { /* empty */ }
        };
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            // 등록된 윈도우가 존재할 경우 윈도우를 갱신한다.
            window.request_redraw();
        } else {
            // 등록된 윈도우가 없는 경우 애플리케이션을 종료한다.
            event_loop.exit();
        }
    }
}
