/// 뷰포트 사각형 영역입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Viewport {
    /// 새로운 뷰포트 영역을 생성합니다.
    pub const fn new(x: f32, y: f32, width: f32, height: f32, z_near: f32, z_far: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            z_near,
            z_far,
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            z_near: 0.0,
            z_far: 1.0,
        }
    }
}
