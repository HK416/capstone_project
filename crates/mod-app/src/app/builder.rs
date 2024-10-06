use std::{
    net::{SocketAddr, ToSocketAddrs}, 
    path::PathBuf
};

use mod_parallelism::NUM_SYSTEM_CORE;
use winit::window::Icon;

use crate::{
    etc::{AppFlags, WindowSize}, 
    scene::GameScene
};

use super::application::Application;



/// 애플리케이션을 생성하는 빌더입니다.
#[derive(Debug)]
pub struct AppBuilder {
    /// 애플리케이션에서 처음 진입하는 시작 게임 장면입니다.
    pub(crate) start_scene: Box<dyn GameScene>, 

    /// 애플리케이션의 현재 실행 디렉토리 경로입니다.
    pub(crate) current_dir: Option<PathBuf>, 

    /// 애플리케이션 창 제목 텍스트입니다.
    pub(crate) title: Option<String>, 

    /// 애플리케이션 창 아이콘입니다.
    pub(crate) icon: Option<Icon>, 

    /// 애플리케이션 창 해상도입니다.
    pub(crate) size: Option<WindowSize>, 

    /// 애플리케이션 창의 전체화면 여부입니다.
    pub(crate) fullscreen: bool, 

    /// 애플리케이션에서 사용 가능한 최대 스레드의 수입니다.
    pub(crate) num_threads: usize, 

    /// 애플리케이션 생성 플래그입니다.
    pub(crate) flags: AppFlags, 

    /// 애플리케이션 서버의 주소입니다.
    pub(crate) address: SocketAddr, 
}

impl AppBuilder {
    /// 새로운 애플리케이션 빌더를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(start_scene: Box<dyn GameScene>) -> Self {
        Self { 
            start_scene, 
            current_dir: None, 
            title: None, 
            icon: None, 
            size: None, 
            fullscreen: true, 
            num_threads: *NUM_SYSTEM_CORE, 
            flags: AppFlags::empty(), 
            address: "localhost:7878".to_socket_addrs().unwrap().next().unwrap(), 
        }
    }

    /// 애플리케이션 실행 디렉토리 경로를 설정합니다.
    #[inline]
    #[must_use]
    pub(crate) fn with_current_path<P: Into<PathBuf>>(mut self, current_dir: P) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    /// 애플리케이션 창 제목 텍스트를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_window_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 애플리케이션 창 아이콘을 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_window_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 애플리케이션 창 크기를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_window_size(mut self, size: WindowSize) -> Self {
        self.size = Some(size);
        self
    }

    /// 애플리케이션 창의 전체화면 여부를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// 애플리케이션에서 사용 가능한 최대 스레드 수를 설정합니다.
    #[inline]
    #[must_use]
    pub(crate) fn with_num_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// 애플리케이션 플래그 옵션을 추가합니다.
    #[inline]
    #[must_use]
    pub(crate) fn with_flags(mut self, flags: AppFlags) -> Self {
        self.flags = self.flags | flags;
        self
    }

    /// 애플리케이션 서버의 주소를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_server_address(mut self, address: SocketAddr) -> Self {
        self.address = address;
        self
    }
}

impl AppBuilder {
    #[cfg(target_pointer_width = "64")]
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn build_and_run(self) {
        use pollster::FutureExt;
        use winit::event_loop::{ControlFlow, EventLoop};

        use crate::{
            app::command::parse_command_line_args, 
            exception::{alert_error, set_panic_hooker}
        };

        // 후커를 설정합니다.
        set_panic_hooker(None);

        // 명령줄 인자를 구문 분석합니다.
        let builder = match parse_command_line_args(self) {
            Ok(builder) => builder, 
            Err(e) => {
                alert_error("Command parsing failed", e.to_string(), None);
                std::process::exit(-1);
            }
        };

        // 이벤트 루프를 생성합니다.
        let event_loop = match EventLoop::with_user_event().build() {
            Ok(event_loop) => event_loop, 
            Err(e) => {
                alert_error("Event loop creation failed", e.to_string(), None);
                std::process::exit(-1);
            }
        };
        event_loop.set_control_flow(ControlFlow::Poll);

        // 이벤트 루프 프록시를 생성합니다.
        let event_loop_proxy = event_loop.create_proxy().into();

        // 애플리케이션을 생성합니다.
        let future = Application::new(event_loop_proxy, builder);
        let mut app = match future.block_on() {
            Ok(app) => app, 
            Err(e) => {
                alert_error("Application creation failed", e.to_string(), None);
                std::process::exit(-1);
            }
        };

        // 애플리케이션을 실행합니다.
        if let Err(e) = event_loop.run_app(&mut app) {
            alert_error("Application launching failed", e.to_string(), None);
            std::process::exit(-1);
        }
    }
}
