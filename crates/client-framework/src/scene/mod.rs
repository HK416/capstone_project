pub mod control_flow;
pub mod manager;

use crate::app::Application;
use crate::error::AppError;

use std::fmt;
use hecs::World;



/// 게임 장면의 인터페이스 `trait` 입니다.
pub trait GameScene : fmt::Debug {
    /// 게임 장면에 진입할 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_enter(&mut self, world: &mut World, app: &dyn Application) -> Result<(), AppError> {
        log::info!("Entering the {:?} scene.", self);
        Ok(())
    }

    /// 게임 장면에 빠져나올 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_exit(&mut self, world: &mut World, app: &dyn Application) -> Result<(), AppError> {
        log::info!("Exiting the {:?} scene.", self);
        Ok(())
    }

    /// 애플리케이션이 일시정지 될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_pause(&mut self, world: &mut World, app: &dyn Application) -> Result<(), AppError> {
        log::info!("Pausing the {:?} scene.", self);
        Ok(())
    }

    /// 애플리케이션이 재개될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resume(&mut self, world: &mut World, app: &dyn Application) -> Result<(), AppError> {
        log::info!("Resuming the {:?} scene.", self);
        Ok(())
    }

    /// 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        world: &mut World, 
        app: &dyn Application, 
        elapsed_time_sec: f32
    ) -> Result<(), AppError> {
        Ok(())
    }

    /// 일정한 경과 시간으로 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_fixed_update(
        &mut self, 
        world: &mut World, 
        app: &dyn Application, 
        elapsed_time_sec: f32
    ) -> Result<(), AppError> {
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
