//! Win32 API를 이용하여 확장 함수를 구현합니다.
//! 

use windows::Win32::{
    Foundation::BOOL, 
    UI::WindowsAndMessaging::ShowCursor
};



#[inline]
pub fn show_cursor(show: bool) {
    unsafe { ShowCursor(BOOL::from(show)) };
}
