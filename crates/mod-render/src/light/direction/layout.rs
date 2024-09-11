use std::mem;
use bytemuck::Pod;
use bytemuck::Zeroable;



/// 3차원 방향 조명 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DirectionLightDataLayout {
    /// 조명의 색깔입니다.
    pub color: gmm::Float4, 

    /// 조명의 월드 좌표상 방향입니다.
    pub direction: gmm::Float3,
    pub _padding0: [u8; mem::size_of::<f32>()], 
}

impl Default for DirectionLightDataLayout {
    #[inline]
    fn default() -> Self {
        let direction: gmm::Vector = gmm::Float3::new(0.5, -1.0, 1.0).into();
        let direction: gmm::Float3 = direction.vec3_normalize()
            .map(|v| v.into())
            .unwrap_or_default();

        Self {
            color: gmm::Float4::ONE, 
            direction, 
            _padding0: [0; mem::size_of::<f32>()],
        }
    }
}
