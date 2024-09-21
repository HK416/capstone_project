use std::{mem, ops, sync::Arc};

use bytemuck::{Pod, Zeroable};



/// 카메라의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraDataLayout {
    /// 투영 변환 행렬과 뷰 변환 행렬이 곱해진 행렬의 데이터입니다.
    pub proj_view: gmm::Float4x4, 

    /// 카메라의 월드 좌표상 위치입니다.
    pub position: gmm::Float3, 
    pub _padding0: [u8; 4], 

    /// 카메라의 월드 좌표상 바라보는 방향입니다.
    pub direction: gmm::Float3, 
    pub _padding1: [u8; 4], 
}

impl Default for CameraDataLayout {
    #[inline]
    fn default() -> Self {
        Self {
            proj_view: gmm::Float4x4::IDENTITY, 
            position: gmm::Float3::ZERO, 
            _padding0: [0; 4], 
            direction: gmm::Float3::ZERO, 
            _padding1: [0; 4]
        }
    }
}



/// 카메라의 유니폼 버퍼입니다.
#[derive(Debug, Clone)]
pub struct CameraUniform {
    buffer: Arc<wgpu::Buffer>
}

impl CameraUniform {
    /// 카메라 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<CameraDataLayout>() as wgpu::BufferAddress;

    /// 카메라 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl CameraUniform {
    /// 초기화되지 않은 새로운 카메라 유니폼 버퍼를 생성합니다.
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


    /// 카메라 유니폼 버퍼의 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: CameraDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut CameraDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (CameraDataLayout :: {})", e);
                }
            }
        });
    }
}

impl ops::Deref for CameraUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

static_assertions::const_assert_eq!(CameraUniform::SIZE as usize, mem::size_of::<CameraDataLayout>());
