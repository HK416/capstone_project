use std::ops;
use std::mem;
use bytemuck::Pod;
use bytemuck::Zeroable;

/// 뼈의 최대 갯수입니다.
pub const MAX_BONES: usize = 256;



/// 뼈 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BoneDataLayout {
    /// 정점당 연결된 뼈의 갯수입니다.
    pub quality: u32, 
    pub _padding0: [u8; mem::size_of::<u32>() * 3], 
}

impl Default for BoneDataLayout {
    #[inline]
    fn default() -> Self {
        Self {
            quality: 4, 
            _padding0: [0; mem::size_of::<u32>() * 3], 
        }
    }
}



/// 각 뼈의 변환 행렬 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BoneMatrixDataLayout([gmm::Float4x4; MAX_BONES]);

impl BoneMatrixDataLayout {
    /// 반복자로부터 뼈 행렬 데이터 레이아웃을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<I>(items: I) -> Self 
    where 
        I: IntoIterator<Item = gmm::Float4x4>, 
        I::IntoIter: ExactSizeIterator, 
    {
        let mut layout = Self::default();
        for (idx, matrix) in items.into_iter().enumerate() {
            if idx >= MAX_BONES { break; }
            layout[idx] = matrix;
        }
        return layout;
    }
}

impl Default for BoneMatrixDataLayout {
    #[inline]
    fn default() -> Self {
        Self([gmm::Float4x4::IDENTITY; MAX_BONES])
    }
}

impl ops::Deref for BoneMatrixDataLayout {
    type Target = [gmm::Float4x4];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for BoneMatrixDataLayout {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
