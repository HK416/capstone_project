use std::ops;
use std::mem;
use std::sync::Arc;
use std::sync::OnceLock;

use super::DirectionLightDataLayout;



/// 3차원 방향 조명의 유니폼 버퍼입니다.
#[derive(Debug)]
pub struct DirectionLightBuffer(Arc<wgpu::Buffer>);

impl DirectionLightBuffer {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<DirectionLightDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGE: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE);
}

impl DirectionLightBuffer {
    /// 유니폼 버퍼를 가져옵니다.
    #[must_use]
    pub fn get(device: &wgpu::Device) -> &'static Self {
        static BUFFER: OnceLock<DirectionLightBuffer> = OnceLock::new();
        BUFFER.get_or_init(|| {
            Self(device.create_buffer(
                &wgpu::BufferDescriptor {
                    label: Some("Uniform(DirectionalLight)"), 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGE
                }
            ).into())
        })
    }

    /// 유니폼 버퍼를 갱신합니다.
    pub fn update(&self, data: DirectionLightDataLayout) {
        let capturable = (*self).clone();
        self.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            if result.is_ok() {
                let mut view = capturable.slice(..).get_mapped_range_mut();
                let layout: &mut DirectionLightDataLayout = bytemuck::from_bytes_mut(&mut view);
                *layout = data;
                drop(view);
                capturable.unmap();
            } else {
                log::warn!("Failed to write uniform buffer (name: DirectionalLight)");
            }
        });
    }
}

impl ops::Deref for DirectionLightBuffer {
    type Target = Arc<wgpu::Buffer>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
