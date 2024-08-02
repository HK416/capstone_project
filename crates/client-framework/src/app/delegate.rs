use std::fmt;
use winit::window::Window;
use winit::event_loop::ActiveEventLoop;

use crate::app::Handler;
use crate::error::ErrorMessage;



/// 애플리케이션을 제어하는 대리자의 `trait` 입니다.
pub trait AppDelegate : fmt::Debug {
    /// 애플리케이션이 시작될 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_launching(
        &mut self, 
        window: &Window, 
        event_loop: &ActiveEventLoop, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        log::info!("애플리케이션 시작 됨.");
        Ok(())
    }

    /// 애플리케이션이 종료될 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_finish(
        &mut self,
        event_loop: &ActiveEventLoop, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        log::info!("애플리케이션 종료 됨.");
        Ok(())
    }

    /// 애플리케이션이 일시 정지될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_paused(
        &mut self, 
        window: &Window, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        log::info!("애플리케이션이 일시 정지 됨.");
        Ok(())
    }

    /// 애플리케이션이 재개될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resumed(
        &mut self, 
        window: &Window, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        log::info!("애플리케이션이 재개 됨.");
        Ok(())
    }
}



/// 기본 애플리케이션 `delegate` 입니다.
#[derive(Debug)]
pub struct DefaultDelegate;

impl AppDelegate for DefaultDelegate { }
