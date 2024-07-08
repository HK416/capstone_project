use thiserror::Error;
use winit::error::OsError;
use winit::error::EventLoopError;
use winit::window::Window;



/// 클라이언트 애플리케이션에서 발생할 수 있는 에러 목록입니다.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("A system error occurred for the following reasons: {0}")]
    System(#[from] OsError),

    #[error("An event loop error occurred for the following reasons: {0}")]
    EventLoop(#[from] EventLoopError),

    #[error("The following error occurred while parsing the command line: {0}")]
    CommandLine(String),

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



/// 에러 메시지를 출력하는 대화 상자를 화면에 표시합니다.
/// 
/// ※ 이 함수는 메인 스레드에서 실행되어야 하며, 스레드를 멈춥니다.
/// 
#[inline]
pub fn show_error_msg<T: AsRef<str>, S: AsRef<str>>(
    title: T, 
    text: S, 
    owner_window: Option<&Window>
) {
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
