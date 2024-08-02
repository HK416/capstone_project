use std::ops;
use gmm::Float4x4;



/// 투영 변환 행렬 입니다.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Projection(pub(crate) Float4x4);

impl ops::Deref for Projection {
    type Target = Float4x4;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Projection {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
