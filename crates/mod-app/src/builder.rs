use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process;

use mod_error::alert_error;
use mod_error::set_panic_hooker;
use mod_parallelism::NUM_SYSTEM_CORE;
use mod_scene::GameScene;
use mod_util::AppDpi;
use mod_util::AppEvent;
use mod_util::AppFlags;
use winit::event_loop::EventLoop;
use winit::window::Icon;

use crate::Application;
use crate::parse_command_line_args;



/// 애플리케이션 빌더입니다.
#[derive(Debug)]
pub struct AppBuilder {
    /// 게임 시작 장면입니다.
    pub(crate) start_scene: Box<dyn GameScene>, 

    /// 현재 실행 디렉토리 경로입니다.
    pub(crate) current_dir: Option<PathBuf>, 

    /// 애플리케이션 창 타이틀입니다.
    pub(crate) title: Option<String>, 

    /// 애플리케이션 창 아이콘 이미지 데이터입니다.
    pub(crate) icon: Option<Icon>, 

    /// 애플리케이션 창 해상도입니다.
    pub(crate) dpi: Option<AppDpi>, 

    /// 애플리케이션 창의 전체화면 여부입니다.
    pub(crate) fullscreen: bool, 

    /// 애플리케이션에서 사용 가능한 최대 스레드의 갯수입니다
    pub(crate) num_threads: usize, 

    /// 애플리케이션 플래그입니다.
    pub(crate) flags: AppFlags, 
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
            dpi: None, 
            fullscreen: true, 
            num_threads: *NUM_SYSTEM_CORE, 
            flags: AppFlags::empty() 
        }
    }

    /// 애플리케이션 실행 디렉토리 경로를 설정합니다.
    #[inline]
    #[must_use]
    pub(crate) fn with_current_path<T: Into<PathBuf>>(mut self, current_dir: T) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    /// 애플리케이션 창 타이틀을 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 애플리케이션 창 아이콘을 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 애플리케이션 창 해상도를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_dpi(mut self, dpi: AppDpi) -> Self {
        self.dpi = Some(dpi);
        self
    }

    /// 애플리케이션 창의 전체화면 여부를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// 애플리케이션에서 사용 가능한 최대 스레드 갯수를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_num_threads(mut self, num_threads: NonZeroUsize) -> Self {
        self.num_threads = num_threads.get();
        self
    }

    /// 애플리케이션 플래그를 추가합니다.
    #[inline]
    #[must_use]
    pub fn with_flags(mut self, flag: AppFlags) -> Self {
        self.flags |= flag;
        self
    }
}

impl AppBuilder {
    /// 애플리케이션을 빌드하고 실행합니다.
    #[cfg(target_pointer_width = "64")]
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn build_and_run(self) {
        // 후커를 설정합니다.
        set_panic_hooker(None);

        // 명령줄 인자를 구문 분석 합니다.
        let result = parse_command_line_args(self);
        let builder = match result {
            Ok(builder) => builder, 
            Err(e) => {
                alert_error("Command parsing failed", e.to_string(), None);
                process::exit(-1);
            }, 
        };

        // 이벤트 루프를 생성합니다.
        let result = EventLoop::with_user_event().build();
        let event_loop: EventLoop<AppEvent> = match result {
            Ok(event_loop) => event_loop, 
            Err(e) => { 
                alert_error("Event loop creation failed", e.to_string(), None);
                process::exit(-1);
            }, 
        };

        // 애플리케이션을 생성합니다.
        let result = pollster::block_on(Application::new(builder));
        let mut app = match result {
            Ok(app) => app, 
            Err(e) => {
                alert_error("Application creation failed", e.to_string(), None);
                process::exit(-1);
            },
        };

        // 애플리케이션을 실행합니다.
        if let Err(e) = event_loop.run_app(&mut app) {
            alert_error("Application launching failed", e.to_string(), None);
            process::exit(-1);
        }
    }
}
