use std::ops;
use bytemuck::Pod;
use bytemuck::Zeroable;



/// 뼈 오프셋의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BoneOffsetsDataLayout([gmm::Float4x4; BoneOffsetsDataLayout::MAX_BONES]);

impl BoneOffsetsDataLayout {
    /// 뼈 오프셋에서 사용하는 최대 뼈의 갯수입니다.
    pub const MAX_BONES: usize = 256;

    /// 새로운 뼈 오프셋 데이터 레이아웃을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ops::Deref for BoneOffsetsDataLayout {
    type Target = [gmm::Float4x4];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for BoneOffsetsDataLayout {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for BoneOffsetsDataLayout {
    #[inline]
    fn default() -> Self {
        Self([gmm::Float4x4::IDENTITY; Self::MAX_BONES])
    }
}
