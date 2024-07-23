
use super::App;
use super::delegate::AppDelegate;
use super::delegate::DefaultDelegate;
use super::flag::AppFlags;
use super::dpi::Dpi;
use crate::scene::GameScene;

use core::num::NonZeroUsize;
use framework::concurrency;
use winit::window::Icon;



/// 애플리케이션을 생성하는 빌더 입니다.
#[derive(Debug)]
pub struct AppBuilder {
    /// 게임 시작 장면입니다.
    pub start_scene: Box<dyn GameScene>,

    /// 애플리케이션 `delegate` 입니다.
    /// 
    /// ※ 기본 값은 [`DefaultDelegate`] 입니다.
    /// 
    pub delegate: Box<dyn AppDelegate>,

    /// 애플리케이션 창 타이틀 문자열 입니다.
    /// 
    /// ※ 기본 값은 `"Hello, World!"` 입니다.
    /// 
    pub title: String,

    /// 애플리케이션 창 아이콘 이미지 데이터 입니다.
    /// 
    /// ※ 기본 값은 `None` 입니다.
    /// 
    pub icon: Option<Icon>,

    /// 애플리케이션 창의 크기 입니다.
    /// 
    /// ※ 기본 값은 `None` 입니다.
    /// 
    pub dpi: Option<Dpi>,

    /// 애플리케이션 창의 전체화면 여부 입니다.
    /// 
    /// ※ 기본 값은 `true` 입니다.
    /// 
    pub fullscreen: bool,

    /// 애플리케이션에서 사용 가능한 최대 스레드의 갯수 입니다.
    /// 
    /// ※ 기본 값은 시스템의 물리적 코어의 갯수 입니다.
    /// 
    pub num_threads: usize,

    /// 애플리케이션 생성에 사용되는 플래그 옵션입니다.
    /// 
    /// ※ 기본 값은 `AppFlags::default()` 입니다.
    /// 
    pub flags: AppFlags,
}

impl AppBuilder {
    /// 애플리케이션 빌더를 생성합니다.
    #[must_use]
    #[inline(always)]
    pub fn new(start_scene: Box<dyn GameScene>) -> Self {
        Self { 
            start_scene,
            delegate: Box::new(DefaultDelegate), 
            title: "Hello, World!".to_string(), 
            icon: None, 
            dpi: None, 
            fullscreen: true, 
            num_threads: *concurrency::MAX_CORE_NUM, 
            flags: AppFlags::default() 
        }
    }
}

#[allow(unused)]
impl AppBuilder {
    /// 애플리케이션 `delegate`를 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_delegate(mut self, delegate: Box<dyn AppDelegate>) -> Self {
        self.delegate = delegate;
        self
    }

    /// 애플리케이션 창 타이틀 문자열을 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = title.into();
        self
    }

    /// 애플리케이션 창 아이콘 이미지 데이터를 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 애플리케이션 창의 크기를 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_dpi(mut self, dpi: Dpi) -> Self {
        self.dpi = Some(dpi);
        self
    } 

    /// 애플리케이션 창의 전체화면 여부를 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// 애플리케이션에서 사용가능한 최대 스레드의 갯수를 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_num_threads(mut self, num_threads: NonZeroUsize) -> Self {
        self.num_threads = num_threads.get();
        self
    }

    /// 애플리케이션 생성 플래그 옵션을 설정합니다.
    #[inline]
    #[must_use]
    pub fn set_flags(mut self, flag: AppFlags) -> Self {
        self.flags = flag;
        self
    }

    /// 애플리케이션 생성 플래그 옵션을 추가합니다.
    #[inline]
    #[must_use]
    pub fn add_flags(mut self, flag: AppFlags) -> Self {
        self.flags |= flag;
        self
    }
}

impl AppBuilder {
    /// 애플리케이션을 빌드하고 실행합니다.
    #[inline(always)]
    pub fn build_and_run(self) {
        App::run(self)
    }
}
