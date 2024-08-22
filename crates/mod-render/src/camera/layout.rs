use std::mem;
use bytemuck::Pod;
use bytemuck::Zeroable;



/// 3차원 카메라에 사용되는 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraDataLayout {
    /// 투영 변환 행렬과 카메라 뷰 변환 행렬을 곱한 행렬입니다.
    pub proj_view: gmm::Float4x4, 

    /// 카메라의 월드 좌표상 위치입니다.
    pub position: gmm::Float3, 
    pub _padding0: [u8; mem::size_of::<f32>()], 

    /// 카메라의 월드 좌표상 바라보는 방향입니다.
    pub direction: gmm::Float3, 
    pub _padding1: [u8; mem::size_of::<f32>()], 
}

impl Default for CameraDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            proj_view: gmm::Float4x4::IDENTITY, 
            position: gmm::Float3::ZERO, 
            _padding0: [0; mem::size_of::<f32>()], 
            direction: gmm::Float3::NEG_Z, 
            _padding1: [0; mem::size_of::<f32>()], 
        }
    }
}
