//! 시스템 API를 이용하여 확장 함수를 구현합니다.
//!

use winit::window::Window;

use super::AppWindowExt;

impl AppWindowExt for Window {
    #[allow(unused_must_use)]
    #[cfg(target_os = "windows")]
    fn confine_cursor_to_window(&self, confine: bool) {
        use windows::Win32::{
            Foundation::{HWND, POINT, RECT},
            Graphics::Gdi::ClientToScreen,
            UI::WindowsAndMessaging::{ClipCursor, GetClientRect},
        };
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        unsafe {
            let handle = self.window_handle().unwrap();
            let hwnd = match handle.as_raw() {
                RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get() as *mut _),
                _ => panic!("no supported platform!"),
            };
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

    #[cfg(not(target_os = "windows"))]
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
