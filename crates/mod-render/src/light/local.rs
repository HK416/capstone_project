use std::{mem, sync::Arc};

use bytemuck::{Pod, Zeroable};

/// 최대 지역 조명의 개수입니다.
pub const MAX_LIGHTS: usize = 32;



/// 지역 조명의 데이터 레이아웃입니다.
/// 
/// 지역 조명에는 Point Light, Spot Light가 포함됩니다.
/// 
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LocalLightDataLayout {
    /// 지역 조명의 색깔입니다.
    pub color: gmm::Float4, 

    /// 지역 조명의 월드 좌표상 위치입니다.
    pub position: gmm::Float3, 

    /// 지역 조명이 영향을 미치는 월드 좌표상 범위입니다.
    pub range: f32, 

    /// 지역 조명이 바라보는 월드 좌표상 방향입니다.
    pub direction: gmm::Float3, 

    /// 지역 조명이 퍼지는 라디안 각도입니다.
    pub angle: f32,
}

impl Default for LocalLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self {
            color: gmm::Float4::ONE, 
            position: gmm::Float3::ZERO, 
            range: 0.0, 
            direction: gmm::Float3::ZERO, 
            angle: 0.0, 
        }
    }
}



/// 지역 조명 배열의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LocalLightArrayDataLayout {
    /// 지역 조명 배열입니다.
    pub lights: [LocalLightDataLayout; MAX_LIGHTS], 

    /// 지역 조명의 개수입니다.
    pub num_lights: u32, 
    pub _padding0: [u8; 12], 
}

impl Default for LocalLightArrayDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            lights: [LocalLightDataLayout::default(); MAX_LIGHTS], 
            num_lights: 0, 
            _padding0: [0; 12] 
        }
    }
}



/// 지역 조명의 유니폼 버퍼입니다.
/// 
/// 지역 조명의 유니폼 버퍼는 카메라 마다 하나씩 가지고 있으며, 각 카메라가 관리합니다.
/// 
#[derive(Debug, Clone)]
pub struct LocalLightUniform {
    buffer: Arc<wgpu::Buffer>
}

impl LocalLightUniform {
    /// 지역 조명 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<LocalLightArrayDataLayout>() as wgpu::BufferAddress;

    /// 지역 조명 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl LocalLightUniform {
    /// 초기화되지 않은 새로운 지역 조명 유니폼 버퍼를 생성합니다.
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

    /// 지역 조명 유니폼 버퍼의 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: LocalLightArrayDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut LocalLightArrayDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (LocalLightUniform :: {})", e);
                }
            }
        });
    }
}
