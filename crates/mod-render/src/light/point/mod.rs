mod buffer;
pub use self::buffer::*; 

mod layout;
pub use self::layout::*; 

use bytemuck::Pod;
use bytemuck::Zeroable;



/// 3차원 점 조명 데이터입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PointLight {
    /// 조명의 색깔입니다.
    pub color: gmm::Float4, 

    /// 조명의 월드 좌표상 위치입니다.
    pub position: gmm::Float3, 
    
    /// 월드 좌표상 조명의 영향을 받는 거리입니다.
    pub range: f32, 
}

impl PointLight {
    /// 점 조명 데이터를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 조명의 색깔을 설정합니다.
    #[inline]
    pub fn with_color<C: Into<gmm::Float4>>(mut self, color: C) -> Self {
        self.color = color.into();
        self
    }

    /// 조명의 월드 좌표상 위치를 설정합니다.
    #[inline]
    pub fn with_position<P: Into<gmm::Float3>>(mut self, position: P) -> Self {
        self.position = position.into();
        self
    }

    /// 조명의 영향을 받는 거리를 설정합니다.
    #[inline]
    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }
}

impl Default for PointLight {
    #[inline]
    fn default() -> Self {
        Self { 
            color: gmm::Float4::ONE, 
            position: gmm::Float3::ZERO, 
            range: 32.0, 
        }
    }
}
