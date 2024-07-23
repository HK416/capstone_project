pub mod control_flow;
pub mod manager;

use crate::app::Application;
use crate::error::AppError;

use hecs::World;
use winit::window::Window;



/// 게임 장면의 인터페이스 `trait` 입니다.
pub trait GameScene : core::fmt::Debug {
    /// 게임 장면에 진입할 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        log::info!("게임 장면({:?})에 진입함...", self);
        Ok(())
    }

    /// 게임 장면에 빠져나올 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        log::info!("게임 장면({:?})에 빠져나옴...", self);
        Ok(())
    }

    /// 애플리케이션 창의 크기가 변경되었을 경우 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resized(
        &mut self, 
        window: &Window,
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        Ok(())
    }

    /// 애플리케이션 창의 종료 버튼이 눌렸을 때 호출되는 콜백 함수입니다.
    /// 
    /// 애플리케이션 종료를 원할 경우 `true`를 반환해야 합니다.
    /// 
    #[inline]
    #[allow(unused_variables)]
    fn on_close(&mut self, app: &dyn Application) -> Result<bool, AppError> {
        Ok(true)
    }

    /// 애플리케이션이 일시정지 될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_pause(
        &mut self, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        log::info!("게임 장면({:?})이 일시 정지 됨...", self);
        Ok(())
    }

    /// 애플리케이션이 재개될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resume(
        &mut self, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        log::info!("게임 장면({:?})이 재개 됨...", self);
        Ok(())
    }

    /// 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Application 
    ) -> Result<(), AppError> {
        Ok(())
    }

    /// 일정한 경과 시간으로 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_fixed_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        Ok(())
    }

    /// 게임 장면이 그려져야 할 떄 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &World, 
        app: &dyn Application
    ) -> Result<(), AppError> {
        log::warn!("게임 장면이 그려지고 있지 않습니다!");
        Ok(())
    }

    /// 게임 장면이 투명한지 여부를 반환합니다.
    /// 
    /// 게임 장면이 투명할 경우 하위 게임 장면도 그려집니다.
    /// 
    #[inline]
    fn transparents(&self) -> bool {
        false
    }
}
