use std::fmt;
use std::sync::Arc;
use std::path::Path;
use winit::window::Window;
use framework::timer::GameTimer;

use crate::app::App;
use crate::app::AppFlags;
use crate::app::AppLocale;



/// 애플리케이션 인터페이스 `trait` 입니다.
pub trait Handler : fmt::Debug {
    /// 애플리케이션에서 사용 가능한 최대 스레드 갯수를 반환합니다.
    fn get_num_threads(&self) -> usize;

    /// 애플리케이션 실행 디렉토리 경로를 빌려옵니다.
    fn ref_current_dir(&self) -> &Path;

    /// 애플리케이션 생성 플래그를 반환합니다.
    fn get_flags(&self) -> AppFlags;

    /// 애플리케이션 표시 언어를 빌려옵니다.
    fn ref_locale(&self) -> Option<&AppLocale>;

    /// 애플리케이션 타이머를 빌려옵니다.
    fn ref_timer(&self) -> &GameTimer;

    /// `wgpu` 렌더러의 인스턴스를 빌려옵니다.
    fn ref_render_instance(&self) -> &Arc<wgpu::Instance>;

    /// `wgpu` 렌더러의 장치 어뎁터를 빌려옵니다.
    fn ref_render_adapter(&self) -> &Arc<wgpu::Adapter>;

    /// `wgpu` 렌더러의 논리적 장치를 빌려옵니다.
    fn ref_render_device(&self) -> &Arc<wgpu::Device>;

    /// `wgpu` 렌더러의 명령 대기열을 빌려옵니다.
    fn ref_render_queue(&self) -> &Arc<wgpu::Queue>;

    /// 애플리케이션 창과 `wgpu` 렌더러의 표면을 빌려옵니다.
    fn ref_window_and_render_surface(&self) -> Option<(&Arc<Window>, &Arc<wgpu::Surface>)>;
}


impl Handler for App {
    #[inline]
    #[must_use]
    fn get_num_threads(&self) -> usize {
        self.num_threads
    }

    #[inline]
    #[must_use]
    fn ref_current_dir(&self) -> &Path {
        &self.current_dir
    }

    #[inline]
    #[must_use]
    fn get_flags(&self) -> AppFlags {
        self.flags
    }

    #[inline]
    #[must_use]
    fn ref_locale(&self) -> Option<&AppLocale> {
        self.locale.as_ref()
    }

    #[inline]
    #[must_use]
    fn ref_timer(&self) -> &GameTimer {
        &self.timer
    }

    #[inline]
    #[must_use]
    fn ref_render_instance(&self) -> &Arc<wgpu::Instance> {
        &self.instance
    }

    #[inline]
    #[must_use]
    fn ref_render_adapter(&self) -> &Arc<wgpu::Adapter> {
        &self.adapter
    }

    #[inline]
    #[must_use]
    fn ref_render_device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    #[inline]
    #[must_use]
    fn ref_render_queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    #[inline]
    #[must_use]
    fn ref_window_and_render_surface(&self) -> Option<(&Arc<Window>, &Arc<wgpu::Surface>)> {
        self.window.as_ref().zip(self.surface.as_ref())
    }
}
