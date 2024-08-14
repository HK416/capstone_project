use std::ops;
use std::mem;
use std::sync::Arc;

use crate::render::object::GameObjectDataLayout;



/// 게임 오브젝트의 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct GameObjectBuffer(wgpu::Buffer);

impl GameObjectBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<GameObjectDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE);
}

impl GameObjectBuffer {
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

impl ops::Deref for GameObjectBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for GameObjectBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
