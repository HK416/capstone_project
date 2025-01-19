use std::path::PathBuf;

use rfd::{MessageButtons, MessageDialog, MessageLevel};
use winit::window::Window;

/// 에러 메시지 대화상자를 생성하고, 화면에 띄웁니다.
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn alert_error(
    title_text: impl Into<String>,
    message_text: impl Into<String>,
    parent: Option<&Window>,
) {
    let mut dialog = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title_text)
        .set_description(message_text)
        .set_buttons(MessageButtons::Ok);

    if let Some(parent_window) = parent {
        dialog = dialog.set_parent(parent_window);
    }

    dialog.show();
}

/// 주어진 경로를 찾을 수 없는 경우 발생하는 오류입니다.
#[derive(Debug, thiserror::Error)]
#[error("The given path could not be found (PATH:{0})")]
pub struct PathNotFound(pub PathBuf);
