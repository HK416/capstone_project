//! NSWindow API를 이용한 확장 함수를 구현합니다.
//!

use winit::window::Window;

use super::AppWindowExt;

impl AppWindowExt for Window {
    fn confine_cursor_to_window(&self, confine: bool) {
        use winit::window::CursorGrabMode;
        if confine {
            self.set_cursor_grab(CursorGrabMode::Confined)
                .or_else(|_| self.set_cursor_grab(CursorGrabMode::Locked))
                .unwrap();
        } else {
            self.set_cursor_grab(CursorGrabMode::None).unwrap();
        }
    }
}
