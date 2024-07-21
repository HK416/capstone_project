use bitflags::bitflags;



/// 32bit 크기의 애플리케이션 생성 플래그 옵션 입니다.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppFlags(u32);

bitflags! {
    impl AppFlags: u32 {
        /// 현재 프레임 레이트를 창의 상단 좌측에 표시할 것인지 나타냅니다.
        const SHOW_FRAME_RATE = 0x01;

        /// 디버깅 레이어 기능을 활성화 할 것인지 나타냅니다.
        const ENABLE_DEBUG_LAYER = 0x02;
    }
}

impl Default for AppFlags {
    #[must_use]
    #[inline(always)]
    fn default() -> Self {
        Self(0)
    }
}

impl core::fmt::Debug for AppFlags {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(AppFlags))
            .field("Show Frame Rate", &self.contains(Self::SHOW_FRAME_RATE))
            .field("Enable Debug Layer", &self.contains(Self::ENABLE_DEBUG_LAYER))
            .finish()
    }
}
