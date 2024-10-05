use winit::error::{EventLoopError, OsError};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 시스템에서 오류가 발생한 경우 발생하는 오류입니다.
    #[error("A system error occurred for the following reasons: {0}")]
    System(#[from] OsError), 

    /// 이벤트 루프에서 오류가 발생한 경우 발생하는 오류입니다.
    #[error("An event loop error occurred for the following reasons: {0}")]
    EventLoop(#[from] EventLoopError), 

    /// 애플리케이션에서 사용 가능한 최대 해상도를 찾지 못한 경우 발생하는 오류입니다.
    #[error("No suitable resolution found!")]
    NoSuitableResolution, 
}
