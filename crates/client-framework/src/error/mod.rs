use std::num::ParseIntError;
use std::num::ParseFloatError;
use thiserror::Error;
use winit::error::OsError;
use winit::error::EventLoopError;
use winit::window::Window;



/// 화면에 에러 메시지를 띄우고 애플리케이션을 종료하는 에러 처리 매크로 입니다.
/// 
/// ※ 이 매크로는 항상 메인 스레드에서 실행되어야 합니다.
/// 
/// # Panic
/// 다음과 같은 상황에서 이 함수는 [`panic!`]을 호출합니다.
/// - 현재 스레드의 id가 메인 스레드의 id와 불일치할 경우.
/// 
#[macro_export]
macro_rules! handle_error {
    ($t:expr, $e:expr, $w:expr) => {{
        crate::error::show_error_msg($t, $e.to_string(), $w);
        std::process::exit(-1)
    }};
}

/// [`AppError`](crate::error::AppError)를 생성하는 매크로 입니다.
/// 
/// 에러가 발생한 파일이름, 줄, 열을 로그에 출력합니다.
/// 
#[macro_export]
macro_rules! app_error {
    ($e:expr) => {{
        log::debug!("애플리케이션 에러 생성: File: {}, Line:{}, Column:{}", file!(), line!(), column!());
        crate::error::AppError::from($e)
    }};
}



/// 클라이언트 애플리케이션에서 발생할 수 있는 에러 목록 입니다.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("A system error occurred for the following reasons: {0}")]
    System(#[from] OsError),

    #[error("An event loop error occurred for the following reasons: {0}")]
    EventLoop(#[from] EventLoopError),

    #[error("The following error occurred while parsing the command line: {0}")]
    CommandLine(#[from] CommandError),

    #[error("No suitable resolution.")]
    NoSuitableResolution,

    #[error("No suitable adapter.")]
    NoSuitableAdapter,
    
    #[error("Surface runtime error occurred for the following reasons: {0}")]
    SurfaceError(#[from] wgpu::SurfaceError),

    #[error("Surface creation failed for the following reasons: {0}")]
    SurfaceCreationFailed(#[from] wgpu::CreateSurfaceError),

    #[error("Device creation failed for the following reasons: {0}")]
    DeviceCreationFailed(#[from] wgpu::RequestDeviceError),
}



/// 명령줄 인수를 구문 분석할 때 발생할 수 있는 에러 목록 입니다.
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Command line arguments are invalid!")]
    InvalidCommand, 

    #[error("Command line arguments are empty!")]
    EmptyCommand,

    #[error("Not enough command line arguments!")]
    NotEnough,

    #[error("Application execution directory path not found!")]
    RootPathNotFound,

    #[error("Parsing integer failed for the following reasons: {0}")]
    ParsingIntFailure(ParseIntError),
    
    #[error("Parsing float failed for the following reasons: {0}")]
    ParsingFloatFailure(ParseFloatError),
}



/// 에러 메시지를 출력하는 대화 상자를 화면에 표시합니다.
/// 
/// ※ 이 함수는 메인 스레드에서 실행되어야 하며, 스레드를 멈춥니다.
/// 
/// # Panic
/// 다음과 같은 상황에서 이 함수는 [`panic!`]을 호출합니다.
/// - 현재 스레드의 id가 메인 스레드의 id와 불일치할 경우.
/// 
#[inline]
pub fn show_error_msg<T: AsRef<str>, S: AsRef<str>>(
    title: T, 
    text: S, 
    owner_window: Option<&Window>
) {
    assert_eq!(std::thread::current().id(), *framework::MAIN_THREAD_ID);
    impl_show_error_msg(title.as_ref(), text.as_ref(), owner_window)
}

/// `Windows`, `macOS`에서 에러 메시지를 출력하는 대화 상자 구현입니다.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn impl_show_error_msg(title: &str, text: &str, owner_window: Option<&Window>) {
    use native_dialog::MessageDialog;
    use native_dialog::MessageType;

    let mut dialog = MessageDialog::new()
        .set_title(title)
        .set_text(text)
        .set_type(MessageType::Error);

    if let Some(window) = owner_window {
        dialog = dialog.set_owner(window);
    }

    dialog.show_alert().unwrap();
}
