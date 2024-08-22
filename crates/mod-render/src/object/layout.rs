use bytemuck::Pod;
use bytemuck::Zeroable;



/// 3차원 오브젝트에 사용되는 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GameObjectDataLayout {
    /// 오브젝트의 월드 변환 행렬입니다.
    pub transform: gmm::Float4x4, 
}

impl Default for GameObjectDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            transform: gmm::Float4x4::IDENTITY, 
        }
    }
}
