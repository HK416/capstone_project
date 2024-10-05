use winit::window::Window;

#[cfg(target_os = "windows")]
pub mod windows;



/// 애플리케이션 창의 확장 함수입니다.
pub trait AppWindowExt {
    /// `true`를 전달할 경우 마우스 커서를 보여줍니다.
    /// 
    /// `false`를 전달할 경우 마우스 커서를 숨깁니다.
    /// 
    fn show_cursor(&self, show: bool);
}

impl AppWindowExt for Window {
    #[inline]
    fn show_cursor(&self, show: bool) {
        #[cfg(target_os = "windows")] {
            return windows::show_cursor(show);
        }

        #[allow(unreachable_code)] {
            return self.set_cursor_visible(show);
        }
    }
}
