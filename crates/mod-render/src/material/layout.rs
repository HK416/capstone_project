use bytemuck::Pod;
use bytemuck::Zeroable;



/// 3차원 메쉬의 재질에 사용되는 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MaterialDataLayout {
    /// `Diffuse` 텍스처에 곱해지는 색상입니다.
    /// 
    /// 재질의 기본 색상을 제어합니다.
    /// 
    pub diffuse: gmm::Float4, 
    
    /// `Specular` 텍스처에 곱해지는 색상입니다.
    /// 
    /// 재질의 반사 강도를 제어합니다.
    /// 
    pub specular: gmm::Float4, 
    
    /// `Emissive` 텍스처에 곱해지는 색상입니다.
    /// 
    /// 재질의 발광 강도를 제어합니다.
    /// 
    pub emissive: gmm::Float4, 
}

impl Default for MaterialDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            diffuse: gmm::Float4::ONE, 
            specular: gmm::Float4::ONE, 
            emissive: gmm::Float4::ONE 
        }
    }
}
