use super::Application;
use crate::error::AppError;

use std::fmt;
use winit::event_loop::ActiveEventLoop;



/// 애플리케이션을 제어하는 `delegate` 입니다.
pub trait AppDelegate : fmt::Debug {
    /// 애플리케이션이 시작될 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_application_launching(
        &mut self, 
        app: &dyn Application,
        event_loop: &ActiveEventLoop
    ) -> Result<(), AppError> {
        log::info!("Application Launching!");
        Ok(())
    }

    /// 애플리케이션이 종료될 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_application_finish(
        &mut self,
        app: &dyn Application, 
        event_loop: &ActiveEventLoop
    ) -> Result<(), AppError> {
        log::info!("Application finish!");
        Ok(())
    }
}



/// 기본 애플리케이션 `delegate` 입니다.
#[derive(Debug)]
pub struct DefaultDelegate;

impl AppDelegate for DefaultDelegate { }
