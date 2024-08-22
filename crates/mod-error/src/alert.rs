use winit::window::Window;



/// 오류 메시지를 Dialog에 출력합니다.
/// 
/// ※ 이 함수는 메인 스레드에서 실행되어야 하며, 호출시 스레드를 정지시킵니다.
/// 
/// # Panics
/// 현재 스레드가 메인 스레드가 아닌 경우 [`panic!`]을 호출합니다.
/// 
pub fn alert_error<T: AsRef<str>, M: AsRef<str>>(
    title: T, text: M, owner: Option<&Window>, 
) {
    use mod_parallelism::is_main_thread;

    assert!(is_main_thread(), "This function must be called from the main thread.");
    impl_alert_error(title, text, owner);
}



/// `Windows`, `macOS`에서 에러 메시지를 출력하는 Dialog 구현입니다.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn impl_alert_error<T: AsRef<str>, M: AsRef<str>>(
    title: T, text: M, owner: Option<&Window>
) {
    use native_dialog::MessageDialog;
    use native_dialog::MessageType;

    let mut dialog = MessageDialog::new()
        .set_title(title.as_ref())
        .set_text(text.as_ref())
        .set_type(MessageType::Error);

    if let Some(window) = owner {
        dialog = dialog.set_owner(window);
    }

    dialog.show_alert().unwrap();
}
