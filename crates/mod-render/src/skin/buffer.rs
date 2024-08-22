use std::ops;
use std::mem;
use std::sync::Arc;

use super::BoneDataLayout;
use super::BoneMatrixDataLayout;



/// 뼈 데이터 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct BoneBuffer(wgpu::Buffer);

impl BoneBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<BoneDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::COPY_DST);
}

impl BoneBuffer {
    /// 초기화 되지 않은 새로운 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: Self::SIZE, 
                usage: Self::USAGE, 
            }
        )).into()
    }

    /// 주어진 데이터로 초기화된 새로운 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn from_data(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: BoneDataLayout
    ) -> Self {
        let buffer = Self::new(label, device);
        queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&data));
        return buffer;
    }
}

impl ops::Deref for BoneBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for BoneBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}



/// 뼈 애니메이션 변환 데이터 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct BoneMatrixBuffer(wgpu::Buffer);

impl BoneMatrixBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<BoneMatrixDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE);
}

impl BoneMatrixBuffer {
    /// 새로운 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Arc<Self> {
        Self(device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: Self::SIZE, 
                usage: Self::USAGE, 
            }
        )).into()
    }
}

impl ops::Deref for BoneMatrixBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for BoneMatrixBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}



/// 뼈 바인드 포즈의 역행렬 데이터 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct BindMatrixBuffer(wgpu::Buffer);

impl BindMatrixBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<BoneMatrixDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::COPY_DST);
}

impl BindMatrixBuffer {
    /// 새로운 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: Self::SIZE, 
                usage: Self::USAGE, 
            }
        )).into()
    }

    /// 주어진 데이터로 초기화된 새로운 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn from_data<I>(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: I
    ) -> Self 
    where 
        I: IntoIterator<Item = gmm::Float4x4>, 
        I::IntoIter: ExactSizeIterator, 
    {
        let buffer = Self::new(label, device);
        let data = BoneMatrixDataLayout::new(data);
        queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&data));
        return buffer;
    }
}

impl ops::Deref for BindMatrixBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for BindMatrixBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
