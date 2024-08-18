use std::ops;
use std::mem;
use std::sync::Arc;

use crate::render::light::spot::SpotLightDataLayout;



/// 3차원 spot 조명의 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct SpotLightBuffer(wgpu::Buffer);

impl SpotLightBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<SpotLightDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE);
}

impl SpotLightBuffer {
    /// 유니폼 버퍼를 생성합니다.
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

impl ops::Deref for SpotLightBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for SpotLightBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
