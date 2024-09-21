use std::{mem, sync::{Arc, OnceLock}};

use bytemuck::{Pod, Zeroable};



/// 전역 조명의 데이터 레이아웃입니다.
/// 
/// 전역 조명에는 Directional Light가 포함됩니다.
/// 
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlobalLightDataLayout {
    /// 전역 조명의 색깔입니다.
    pub color: gmm::Float4, 

    /// 전역 조명이 바라보는 월드 좌표상 방향입니다.
    pub direction: gmm::Float3, 
    pub _padding0: [u8; 4], 
}

impl Default for GlobalLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            color: gmm::Float4::ONE, 
            direction: gmm::Float3::NEG_Y, 
            _padding0: [0; 4] 
        }
    }
}



/// 전역 조명의 유니폼 버퍼입니다.
/// 
/// 전역 조명의 유니폼 버퍼는 애플리케이션에서 오직 한개만 존재합니다.
/// 
#[derive(Debug, Clone)]
pub struct GlobalLightUniform {
    /// 전역 조명의 유니폼 버퍼
    buffer: Arc<wgpu::Buffer>, 
}

impl GlobalLightUniform {
    /// 전역 조명 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<GlobalLightDataLayout>() as wgpu::BufferAddress;

    /// 전역 조명 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl GlobalLightUniform {
    /// 전역 조명 유니폼 버퍼를 가져옵니다.
    #[must_use]
    pub fn get(device: &Arc<wgpu::Device>) -> &'static Self {
        static THIS: OnceLock<GlobalLightUniform> = OnceLock::new();
        THIS.get_or_init(|| {
            Self { 
                buffer: device.create_buffer(
                    &wgpu::BufferDescriptor {
                        label: Some("GlobalLightUniform"), 
                        mapped_at_creation: false, 
                        size: Self::SIZE, 
                        usage: Self::USAGES
                    }
                ).into()
            }
        })
    }

    /// 전역 조명 유니폼 버퍼의 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: GlobalLightDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut GlobalLightDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (GlobalLightUniform :: {})", e);
                }
            }
        });
    }
}
