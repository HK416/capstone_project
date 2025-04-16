use winit::window::Window;

#[cfg(target_os = "windows")]
pub mod windows;

/// `winit::window::Window`의 확장 기능입니다.
pub trait AppWindowExt {
    #[cfg(target_os = "windows")]
    /// 커서를 애플리케이션 창 좌표로 제한합니다.
    fn confine_cursor_to_window(&self, confine: bool);
}
