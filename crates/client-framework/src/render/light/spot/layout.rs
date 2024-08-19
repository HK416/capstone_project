use std::mem;
use bytemuck::Pod;
use bytemuck::Zeroable;

use crate::render::light::spot::SpotLight;



/// 3차원 spot 조명 데이터 배열 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SpotLightDataLayout {
    /// spot 조명 데이터입니다. (최대 16개)
    pub lights: [SpotLight; SpotLightDataLayout::MAX_LIGHTS], 

    /// 조명의 갯수입니다. (최대 16개)
    pub num_lights: u32, 
    pub _padding0: [u8; mem::size_of::<u32>() * 3], 
}

impl SpotLightDataLayout {
    /// 조명의 최대 갯수입니다.
    pub const MAX_LIGHTS: usize = 16;
}

impl Default for SpotLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self {
            lights: [SpotLight::default(); SpotLightDataLayout::MAX_LIGHTS], 
            num_lights: 0, 
            _padding0: [0; mem::size_of::<u32>() * 3], 
        }
    }
}
