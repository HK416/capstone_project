use std::fmt;

use bitflags::bitflags;



/// 32bit 크기의 애플리케이션 플래그입니다.
#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppFlags(u16);

bitflags! {
    impl AppFlags: u16 {
        /// 현재 프레임 레이트를 화면에 표시합니다.
        const SHOW_FRAME_RATE = 0x01;

        /// 디버깅 레이어 기능을 활성화 합니다.
        const ENABLE_DEBUG_LAYER = 0x02;

        /// 수직 동기화 기능을 비활성화 합니다.
        const DISABLE_VSYNC = 0x04;
    }
}

impl fmt::Debug for AppFlags {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(AppFlags))
            .field("Show Frame Rate", &self.contains(AppFlags::SHOW_FRAME_RATE))
            .field("Enable Debug Layer", &self.contains(AppFlags::ENABLE_DEBUG_LAYER))
            .field("Disable Vsync", &self.contains(AppFlags::DISABLE_VSYNC))
            .finish()
    }
}
