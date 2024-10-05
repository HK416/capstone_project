use std::sync::Arc;

use rfd::{MessageButtons, MessageDialog, MessageLevel};
use winit::window::Window;



/// 에러 메시지 대화상자를 생성하고, 화면에 띄웁니다.
/// 
/// 이 함수는 일부 플랫폼에서만 사용 가능합니다.
/// - Windows
/// - macOS
/// 
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn alert_error(
    title_text: impl Into<String>, 
    message_text: impl Into<String>, 
    parent: Option<&Window>
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



/// 애플리케이션에서 [`panic!`]을 호출했을 때 처리할 함수를 설정합니다.
pub fn set_panic_hooker(parent: Option<Arc<Window>>){
    std::panic::set_hook(Box::new(move |info| {
        log::error!("{:?}", info);
        alert_error("Runtime error", "A fatal error has occurred and the application will be terminated.", parent.as_deref());
        std::process::exit(-1);
    }))
}
