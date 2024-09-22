use std::{mem, ops, sync::Arc};

use bytemuck::{Pod, Zeroable};



/// 재질의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MaterialDataLayout {
    /// 재질의 매끄러운 정도입니다. (0.0 ~ 1.0)
    pub glossiness: f32, 

    /// 재질의 부드러운 정도입니다. (0.0 ~ 1.0)
    pub smoothness: f32, 

    /// 재질의 금속성 정도 (0.0 ~ 1.0)
    pub metallic: f32, 
    pub _padding0: [u8; 4], 

    /// 재질의 `Diffuse` 색상입니다.
    pub diffuse: gmm::Float4, 

    /// 재질의 `Specular` 색상입니다.
    pub specular: gmm::Float4, 

    /// 재질의 `Emissive` 색상입니다.
    pub emissive: gmm::Float4, 
}

impl Default for MaterialDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            glossiness: 0.5, 
            smoothness: 0.5, 
            metallic: 0.25, 
            _padding0: [0; 4], 
            diffuse: gmm::Float4::fill(0.85), 
            specular: gmm::Float4::fill(1.0), 
            emissive: gmm::Float4::fill(1.0) 
        }
    }
}



/// 재질의 유니폼 버퍼입니다.
#[derive(Debug, Clone)]
pub struct MaterialUniform {
    buffer: Arc<wgpu::Buffer>
}

impl MaterialUniform {
    /// 재질 유니폼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<MaterialDataLayout>() as wgpu::BufferAddress;

    /// 재질 유니폼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl MaterialUniform {
    /// 초기화되지 않은 재질의 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &Arc<wgpu::Device>) -> Self {
        Self {
            buffer: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES
                }
            ).into()
        }
    }

    /// 재질 유니폼 버퍼 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: MaterialDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut MaterialDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (MaterialUniform :: {})", e);
                }
            }
        });
    }
}

impl ops::Deref for MaterialUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

static_assertions::const_assert_eq!(MaterialUniform::SIZE as usize, mem::size_of::<MaterialDataLayout>());
