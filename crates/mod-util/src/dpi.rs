use std::fmt;

use serde::Deserialize;
use serde::Serialize;
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;



/// 클라이언트 창의 해상도 목록입니다.
#[repr(C)]
#[derive(Deserialize, Serialize)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppDpi {
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

impl AppDpi {
    /// 사용 가능한 최소 해상도입니다.
    pub const MIN: Self = Self::W864H486;

    /// 사용 가능한 최대 해상도입니다.
    pub const MAX: Self = Self::W3840H2160;

    /// 현재 시스템에서 사용가능한 최대 해상도를 반환합니다.
    /// 
    /// 출력 장치가 없거나, 출력 장치의 해상도가 호환되지 않는 경우 `None`을 반환합니다.
    /// 
    pub fn find_maximize_dpi(event_loop: &ActiveEventLoop) -> Option<AppDpi> {
        // 현재 주 모니터의 정보를 가져옵니다.
        let monitor = event_loop.primary_monitor()?;
        let px_size = monitor.size();

        // 가장 큰 해상도부터 모니터의 물리적 해상도보다 작은지 확인합니다.
        let mut target = Some(AppDpi::MAX);
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

impl AppDpi {
    /// 한 단계 낮은 해상도를 반환합니다.
    /// 한 단계 낮은 해상도가 존재하지 않는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn downgrade(self) -> Option<Self> {
        match self {
            AppDpi::W864H486 => None, 
            AppDpi::W960H540 => Some(AppDpi::W864H486), 
            AppDpi::W1024H576 => Some(AppDpi::W960H540), 
            AppDpi::W1152H648 => Some(AppDpi::W1024H576), 
            AppDpi::W1280H720 => Some(AppDpi::W1152H648), 
            AppDpi::W1366H768 => Some(AppDpi::W1280H720), 
            AppDpi::W1600H900 => Some(AppDpi::W1366H768), 
            AppDpi::W1920H1080 => Some(AppDpi::W1600H900), 
            AppDpi::W2048H1152 => Some(AppDpi::W1920H1080), 
            AppDpi::W2560H1440 => Some(AppDpi::W2048H1152), 
            AppDpi::W2880H1620 => Some(AppDpi::W2560H1440), 
            AppDpi::W3200H1800 => Some(AppDpi::W2880H1620), 
            AppDpi::W3840H2160 => Some(AppDpi::W3200H1800), 
        }
    }

    /// 한 단계 높은 해상도를 반환합니다.
    /// 한 단계 높은 해상도가 존재하지 않는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn upgrade(self) -> Option<Self> {
        match self {
            AppDpi::W864H486 => Some(AppDpi::W960H540), 
            AppDpi::W960H540 => Some(AppDpi::W1024H576), 
            AppDpi::W1024H576 => Some(AppDpi::W1152H648), 
            AppDpi::W1152H648 => Some(AppDpi::W1280H720), 
            AppDpi::W1280H720 => Some(AppDpi::W1366H768), 
            AppDpi::W1366H768 => Some(AppDpi::W1600H900), 
            AppDpi::W1600H900 => Some(AppDpi::W1920H1080), 
            AppDpi::W1920H1080 => Some(AppDpi::W2048H1152), 
            AppDpi::W2048H1152 => Some(AppDpi::W2560H1440), 
            AppDpi::W2560H1440 => Some(AppDpi::W2880H1620), 
            AppDpi::W2880H1620 => Some(AppDpi::W3200H1800), 
            AppDpi::W3200H1800 => Some(AppDpi::W3840H2160), 
            AppDpi::W3840H2160 => None, 
        }
    }

    /// [PhysicalSize](winit::dpi::PhysicalSize)를 반환합니다.
    #[inline]
    #[must_use]
    pub fn size(self) -> PhysicalSize<u32> {
        match self {
            AppDpi::W864H486 => (864, 486), 
            AppDpi::W960H540 => (960, 540), 
            AppDpi::W1024H576 => (1024, 576), 
            AppDpi::W1152H648 => (1152, 648), 
            AppDpi::W1280H720 => (1280, 720), 
            AppDpi::W1366H768 => (1366, 768), 
            AppDpi::W1600H900 => (1600, 900), 
            AppDpi::W1920H1080 => (1920, 1080), 
            AppDpi::W2048H1152 => (2048, 1152), 
            AppDpi::W2560H1440 => (2560, 1440), 
            AppDpi::W2880H1620 => (2880, 1620), 
            AppDpi::W3200H1800 => (3200, 1800), 
            AppDpi::W3840H2160 => (3840, 2160), 
        }.into()
    }
}

impl fmt::Debug for AppDpi {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(Dpi))
            .field(&match self {
                AppDpi::W864H486 => "864x486", 
                AppDpi::W960H540 => "960x540", 
                AppDpi::W1024H576 => "1024x576", 
                AppDpi::W1152H648 => "1152x648", 
                AppDpi::W1280H720 => "1280x720", 
                AppDpi::W1366H768 => "1366x768", 
                AppDpi::W1600H900 => "1600x900", 
                AppDpi::W1920H1080 => "1920x1080", 
                AppDpi::W2048H1152 => "2048x1152", 
                AppDpi::W2560H1440 => "2560x1440", 
                AppDpi::W2880H1620 => "2880x1620", 
                AppDpi::W3200H1800 => "3200x1800", 
                AppDpi::W3840H2160 => "3840x2160", 
            })
            .finish()
    }
}
