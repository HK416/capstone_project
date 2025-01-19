use std::fmt;

use bitflags::bitflags;

/// ## Application Running Flags
#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppFlags(u16);

bitflags! {
    impl AppFlags: u16 {
        /// 현재 프레임 레이트를 화면에 표시합니다.
        const SHOW_FRAME_RATE = 0x01;

        /// 수직 동기화 기능을 비활성화 합니다.
        const DISABLE_VSYNC = 0x02;
    }
}

impl fmt::Debug for AppFlags {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(AppFlags))
            .field("Show Frame Rate", &self.contains(AppFlags::SHOW_FRAME_RATE))
            .field("Disable Vsync", &self.contains(AppFlags::DISABLE_VSYNC))
            .finish()
    }
}
