//! Win32 API를 이용하여 확장 함수를 구현합니다.
//!

use windows::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::{
        Input::KeyboardAndMouse::GetActiveWindow,
        WindowsAndMessaging::{ClipCursor, GetClientRect},
    },
};
use winit::window::Window;

use super::AppWindowExt;

impl AppWindowExt for Window {
    #[allow(unused_must_use)]
    fn confine_cursor_to_window(&self, confine: bool) {
        unsafe {
            let hwnd = GetActiveWindow();
            let mut rect = RECT::default();
            if confine && GetClientRect(hwnd, &mut rect).is_ok() {
                let mut top_left = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                let mut bottom_right = POINT {
                    x: rect.right,
                    y: rect.bottom,
                };

                ClientToScreen(hwnd, &mut top_left);
                ClientToScreen(hwnd, &mut bottom_right);

                rect = RECT {
                    left: top_left.x,
                    top: top_left.y,
                    right: bottom_right.x,
                    bottom: bottom_right.y,
                };
                ClipCursor(Some(&rect));
            } else {
                ClipCursor(None);
            }
        }
    }
}
