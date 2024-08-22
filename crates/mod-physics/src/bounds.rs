use serde::Serialize;
use serde::Deserialize;



/// 3차원 축 정렬 경계 상자 입니다.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    /// 경계 상자의 중심 좌표입니다.
    pub center: gmm::Float3, 

    /// 경계 상자의 크기입니다.
    pub extents: gmm::Float3, 
}

impl Default for BoundingBox {
    #[inline]
    fn default() -> Self {
        Self { 
            center: gmm::Float3::ZERO, 
            extents: gmm::Float3::ZERO 
        }
    }
}
