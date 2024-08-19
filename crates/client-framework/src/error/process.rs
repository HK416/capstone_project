use std::panic;
use std::sync::Arc;
use winit::window::Window;
use native_dialog::MessageDialog;
use native_dialog::MessageType;
use framework::concurrency::MAIN_THREAD_ID;



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
    assert_eq!(std::thread::current().id(), *MAIN_THREAD_ID);
    impl_show_error_msg(title.as_ref(), text.as_ref(), owner_window)
}



/// `Windows`, `macOS`에서 에러 메시지를 출력하는 대화 상자 구현입니다.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn impl_show_error_msg(title: &str, text: &str, owner_window: Option<&Window>) {
    let mut dialog = MessageDialog::new()
        .set_title(title)
        .set_text(text)
        .set_type(MessageType::Error);

    if let Some(window) = owner_window {
        dialog = dialog.set_owner(window);
    }

    dialog.show_alert().unwrap();
}



/// 오류가 발생한 경우 에러 메시지를 화면에 표시합니다.
/// 에러 메시지를 화면에 표시한 후 애플리케이션 프로세서를 종료시킵니다.
macro_rules! success {
    ($title:expr, $result:expr, $window:expr) => {
        match $result {
            Ok(item) => item, 
            Err(err) => {
                crate::error::show_error_msg($title, err.to_string(), $window);
                std::process::exit(-1)
            }, 
        }
    };
}

pub(crate) use success;



/// `panic!` 호출시 처리할 함수를 설정합니다.
pub fn set_panic_hooker(window: Option<Arc<Window>>) {
    panic::set_hook(Box::new(move |info| {
        log::error!("{}", info);
        show_error_msg("Runtime Error", info.to_string(), window.as_deref());
        std::process::exit(-1);
    }));
}
