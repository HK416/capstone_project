//! 클라이언트 애플리케이션에서 발생할 수 있는 오류와 관련된 코드를 작성합니다.
//! 

use thiserror::Error;
use winit::error::OsError;
use winit::error::EventLoopError;



/// 클라이언트 애플리케이션에서 발생할 수 있는 에러 목록입니다.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("A system error occurred for the following reasons:{0}")]
    System(#[from] OsError),

    #[error("An event loop error occurred for the following reasons:{0}")]
    EventLoop(#[from] EventLoopError),
}
