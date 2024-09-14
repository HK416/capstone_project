use std::fmt;

use serde::{Deserialize, Serialize};
use winit::{
    dpi::PhysicalSize, 
    event_loop::ActiveEventLoop
};



/// 사용 가능한 창의 크기 목록입니다.
#[repr(C)]
#[derive(Deserialize, Serialize)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowSize {
    #[default]
    W864H486,
    W960H540, 
    W1024H576,
    W1152H648,
    W1280H720, 
    W1366H768, 
    W1600H900, 
    W1920H1080,
    W2048H1152,
    W2560H1440,
    W2880H1620,
    W3200H1800,
    W3840H2160,
}

impl WindowSize {
    /// 사용 가능한 최소 창 크기입니다.
    pub const MIN: Self = Self::W864H486;

    /// 사용 가능한 최대 창 크기입니다.
    pub const MAX: Self = Self::W3840H2160;

    /// 현재 시스템에서 사용가능한 최대 창 크기를 반환합니다.
    /// 
    /// 출력 장치가 없거나, 출력 장치의 창 크기가 호환되지 않는 경우 `None`을 반환합니다.
    /// 
    pub fn find_maximize_size(event_loop: &ActiveEventLoop) -> Option<WindowSize> {
        // 현재 주 모니터의 정보를 가져옵니다.
        let monitor = event_loop.primary_monitor()?;
        let px_size = monitor.size();

        // 가장 큰 창 크기부터 모니터의 물리적 창 크기보다 작은지 확인합니다.
        let mut target = Some(WindowSize::MAX);
        while let Some(dpi) = target {
            let dpi_size = dpi.size();
            if dpi_size.width <= px_size.width && dpi_size.height <= px_size.height {
                return target;
            }
            target = dpi.downgrade();
        }
        return None;
    }
}

impl WindowSize {
    /// 한 단계 낮은 창 크기를 반환합니다.
    /// 한 단계 낮은 창 크기가 존재하지 않는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn downgrade(self) -> Option<Self> {
        match self {
            WindowSize::W864H486 => None, 
            WindowSize::W960H540 => Some(WindowSize::W864H486), 
            WindowSize::W1024H576 => Some(WindowSize::W960H540), 
            WindowSize::W1152H648 => Some(WindowSize::W1024H576), 
            WindowSize::W1280H720 => Some(WindowSize::W1152H648), 
            WindowSize::W1366H768 => Some(WindowSize::W1280H720), 
            WindowSize::W1600H900 => Some(WindowSize::W1366H768), 
            WindowSize::W1920H1080 => Some(WindowSize::W1600H900), 
            WindowSize::W2048H1152 => Some(WindowSize::W1920H1080), 
            WindowSize::W2560H1440 => Some(WindowSize::W2048H1152), 
            WindowSize::W2880H1620 => Some(WindowSize::W2560H1440), 
            WindowSize::W3200H1800 => Some(WindowSize::W2880H1620), 
            WindowSize::W3840H2160 => Some(WindowSize::W3200H1800), 
        }
    }

    /// 한 단계 높은 창 크기를 반환합니다.
    /// 한 단계 높은 창 크기가 존재하지 않는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn upgrade(self) -> Option<Self> {
        match self {
            WindowSize::W864H486 => Some(WindowSize::W960H540), 
            WindowSize::W960H540 => Some(WindowSize::W1024H576), 
            WindowSize::W1024H576 => Some(WindowSize::W1152H648), 
            WindowSize::W1152H648 => Some(WindowSize::W1280H720), 
            WindowSize::W1280H720 => Some(WindowSize::W1366H768), 
            WindowSize::W1366H768 => Some(WindowSize::W1600H900), 
            WindowSize::W1600H900 => Some(WindowSize::W1920H1080), 
            WindowSize::W1920H1080 => Some(WindowSize::W2048H1152), 
            WindowSize::W2048H1152 => Some(WindowSize::W2560H1440), 
            WindowSize::W2560H1440 => Some(WindowSize::W2880H1620), 
            WindowSize::W2880H1620 => Some(WindowSize::W3200H1800), 
            WindowSize::W3200H1800 => Some(WindowSize::W3840H2160), 
            WindowSize::W3840H2160 => None, 
        }
    }

    /// [PhysicalSize](winit::dpi::PhysicalSize)를 반환합니다.
    #[inline]
    #[must_use]
    pub fn size(self) -> PhysicalSize<u32> {
        match self {
            WindowSize::W864H486 => (864, 486), 
            WindowSize::W960H540 => (960, 540), 
            WindowSize::W1024H576 => (1024, 576), 
            WindowSize::W1152H648 => (1152, 648), 
            WindowSize::W1280H720 => (1280, 720), 
            WindowSize::W1366H768 => (1366, 768), 
            WindowSize::W1600H900 => (1600, 900), 
            WindowSize::W1920H1080 => (1920, 1080), 
            WindowSize::W2048H1152 => (2048, 1152), 
            WindowSize::W2560H1440 => (2560, 1440), 
            WindowSize::W2880H1620 => (2880, 1620), 
            WindowSize::W3200H1800 => (3200, 1800), 
            WindowSize::W3840H2160 => (3840, 2160), 
        }.into()
    }
}

impl fmt::Debug for WindowSize {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(Dpi))
            .field(&match self {
                WindowSize::W864H486 => "864x486", 
                WindowSize::W960H540 => "960x540", 
                WindowSize::W1024H576 => "1024x576", 
                WindowSize::W1152H648 => "1152x648", 
                WindowSize::W1280H720 => "1280x720", 
                WindowSize::W1366H768 => "1366x768", 
                WindowSize::W1600H900 => "1600x900", 
                WindowSize::W1920H1080 => "1920x1080", 
                WindowSize::W2048H1152 => "2048x1152", 
                WindowSize::W2560H1440 => "2560x1440", 
                WindowSize::W2880H1620 => "2880x1620", 
                WindowSize::W3200H1800 => "3200x1800", 
                WindowSize::W3840H2160 => "3840x2160", 
            })
            .finish()
    }
}
