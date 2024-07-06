use std::fmt;
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;



/// 클라이언트 애플리케이션에서 사용하는 해상도 목록입니다.
#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dpi {
    #[default]
    W640H360,
    W800H450,
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

impl Dpi {
    /// 현재 해상도에서 한 단계 낮은 해상도를 반환합니다.
    /// 
    /// 한 단계 낮은 해상도가 없는 경우 `None`을 반환합니다.
    /// 
    #[inline]
    #[must_use]
    #[allow(unused)]
    pub fn downgrade(self) -> Option<Self> {
        match self {
            Self::W640H360 => None, 
            Self::W800H450 => Some(Self::W640H360), 
            Self::W864H486 => Some(Self::W800H450), 
            Self::W960H540 => Some(Self::W864H486), 
            Self::W1024H576 => Some(Self::W960H540), 
            Self::W1152H648 => Some(Self::W1024H576), 
            Self::W1280H720 => Some(Self::W1152H648), 
            Self::W1366H768 => Some(Self::W1280H720), 
            Self::W1600H900 => Some(Self::W1366H768), 
            Self::W1920H1080 => Some(Self::W1600H900), 
            Self::W2048H1152 => Some(Self::W1920H1080), 
            Self::W2560H1440 => Some(Self::W2048H1152), 
            Self::W2880H1620 => Some(Self::W2560H1440), 
            Self::W3200H1800 => Some(Self::W2880H1620), 
            Self::W3840H2160 => Some(Self::W3200H1800), 
        }
    }

    /// 현재 해상도에서 한 단계 높은 해상도를 반환합니다.
    /// 
    /// 한 단계 높은 해상도가 없는 경우 `None`을 반환합니다.
    /// 
    #[inline]
    #[must_use]
    #[allow(unused)]
    pub fn upgrade(self) -> Option<Self> {
        match self {
            Self::W640H360 => Some(Self::W800H450), 
            Self::W800H450 => Some(Self::W864H486), 
            Self::W864H486 => Some(Self::W960H540), 
            Self::W960H540 => Some(Self::W1024H576), 
            Self::W1024H576 => Some(Self::W1152H648), 
            Self::W1152H648 => Some(Self::W1280H720), 
            Self::W1280H720 => Some(Self::W1366H768), 
            Self::W1366H768 => Some(Self::W1600H900), 
            Self::W1600H900 => Some(Self::W1920H1080), 
            Self::W1920H1080 => Some(Self::W2048H1152), 
            Self::W2048H1152 => Some(Self::W2560H1440), 
            Self::W2560H1440 => Some(Self::W2880H1620), 
            Self::W2880H1620 => Some(Self::W3200H1800), 
            Self::W3200H1800 => Some(Self::W3840H2160), 
            Self::W3840H2160 => None, 
        }
    }
}

impl Into<PhysicalSize<u32>> for Dpi {
    #[inline]
    fn into(self) -> PhysicalSize<u32> {
        match self {
            Self::W640H360 => (640, 360), 
            Self::W800H450 => (800, 450), 
            Self::W864H486 => (864, 486), 
            Self::W960H540 => (960, 540), 
            Self::W1024H576 => (1024, 576), 
            Self::W1152H648 => (1152, 648), 
            Self::W1280H720 => (1280, 720), 
            Self::W1366H768 => (1366, 768), 
            Self::W1600H900 => (1600, 900), 
            Self::W1920H1080 => (1920, 1080), 
            Self::W2048H1152 => (2048, 1152), 
            Self::W2560H1440 => (2560, 1440), 
            Self::W2880H1620 => (2880, 1620), 
            Self::W3200H1800 => (3200, 1800), 
            Self::W3840H2160 => (3840, 2160), 
        }.into()
    }
}

impl ToString for Dpi {
    #[inline]
    fn to_string(&self) -> String {
        String::from(match self {
            Self::W640H360 => "640x360", 
            Self::W800H450 => "800x450", 
            Self::W864H486 => "864x486", 
            Self::W960H540 => "960x540", 
            Self::W1024H576 => "1024x576", 
            Self::W1152H648 => "1152x648", 
            Self::W1280H720 => "1280x720", 
            Self::W1366H768 => "1366x768", 
            Self::W1600H900 => "1600x900", 
            Self::W1920H1080 => "1920x1080", 
            Self::W2048H1152 => "2048x1152", 
            Self::W2560H1440 => "2560x1440", 
            Self::W2880H1620 => "2880x1620", 
            Self::W3200H1800 => "3200x1800", 
            Self::W3840H2160 => "3840x2160", 
        })
    }
}

impl fmt::Debug for Dpi {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(Dpi))
            .field(&self.to_string())
            .finish()
    }
}



/// 현재 시스템에서 사용가능한 최대 해상도를 찾습니다.
/// 
/// 최대 해상도를 찾을 수 없는 경우 
/// (예를 들어 출력 장치가 없거나, 크기가 너무 작은 경우) `None`을 반환합니다.
/// 
pub fn find_maximize_dpi(event_loop: &ActiveEventLoop) -> Option<Dpi> {
    // 현재 주 모니터의 정보를 가져옵니다.
    let monitor = event_loop.primary_monitor()?;
    let monitor_size = monitor.size();

    // 가장 큰 해상도부터 모니터의 물리적 해상도보다 작은지 확인합니다.
    let mut target = Some(Dpi::W3840H2160);
    while let Some(dpi) = target {
        let target_size: PhysicalSize<u32> = dpi.into();
        if target_size.width <= monitor_size.width 
        && target_size.height <= monitor_size.height {
            return target;
        }
        target = dpi.downgrade();
    }   
    return None;
}
