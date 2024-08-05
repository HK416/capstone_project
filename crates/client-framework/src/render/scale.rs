/// 렌더링 품질을 정합니다.
/// 
/// ※ 스왑체인 텍스처의 크기와 깊이 버퍼 텍스처의 크기에는 영향을 끼치지 않습니다.
/// 
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderScale {
    P50, 
    P75, 
    P100, 
    P125, 
    P150, 
}

impl Into<f32> for RenderScale {
    #[inline]
    fn into(self) -> f32 {
        match self {
            RenderScale::P50 => 0.5, 
            RenderScale::P75 => 0.75, 
            RenderScale::P100 => 1.0, 
            RenderScale::P125 => 1.25, 
            RenderScale::P150 => 1.5
        }
    }
}
