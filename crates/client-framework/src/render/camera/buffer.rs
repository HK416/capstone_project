use std::mem;
use std::ops;
use std::sync::Arc;

use crate::render::camera::CameraDataLayout;



/// 카메라 오브젝트의 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct CameraObjectBuffer(wgpu::Buffer);

impl CameraObjectBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<CameraDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl CameraObjectBuffer {
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

impl ops::Deref for CameraObjectBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0 
    }
}

impl ops::DerefMut for CameraObjectBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
