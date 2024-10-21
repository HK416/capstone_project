use std::{mem, sync::{Arc, OnceLock}};

use bytemuck::{Pod, Zeroable};

/// 최대 지역 조명의 개수입니다.
pub const MAX_LOCAL_LIGHTS: usize = 16;



/// 전역 조명 데이터 레이아웃
/// 
/// 전역 조명에는 `Directional Light`가 포함됩니다.
/// 
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlobalLightDataLayout {
    /// 전역 조명의 색깔입니다.
    pub color: [f32; 4], 

    /// 전역 조명이 바라보는 월드 좌표상 방향입니다.
    pub direction: [f32; 3], 
    pub _padding0: [u8; 4] 
}

impl Default for GlobalLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            color: gmm::Float4::ONE.into(), 
            direction: gmm::Float3::ZERO.into(), 
            _padding0: [0; 4] 
        }
    }
}





/// 전역 조명 데이터 유니폼 버퍼
/// 
/// 전역 조명 데이터 유니폼 버퍼는 애플리케이션에서 오직 한개만 존재합니다.
/// 
#[derive(Debug, Clone)]
pub struct GlobalLightUniform {
    inner: Arc<wgpu::Buffer> 
}

impl GlobalLightUniform {
    /// 유니폼 버퍼의 크기
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<GlobalLightDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl GlobalLightUniform {
    /// 전역 조명 데이터 유니폼 버퍼를 가져옵니다.
    /// 
    /// 전역 조명 데이터 유니폼 버퍼가 생성되어 있지 않은 경우 
    /// 전역 조명 데이터 유니폼 버퍼를 생성합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get_or_init(device: &wgpu::Device) -> &'static Self {
        static INSTANCE: OnceLock<GlobalLightUniform> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            Self { 
                inner: device.create_buffer(
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

    /// 전역 조명 유니폼 버퍼의 데이터를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: GlobalLightDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut GlobalLightDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (UNIFORM:{})", e);
                }
            }
        });

        // 제출된 작업이 끝날 때 까지 대기합니다.
        let index = queue.submit([]);
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(index));
    }

    /// 전역 조명 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}





/// 지역 조명 데이터 레이아웃
/// 
/// 지역 조명에는 `Point Light`, `Spot Light`가 포함됩니다.
/// 
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LocalLightDataLayout {
    /// 지역 조명의 색깔입니다.
    pub color: [f32; 4], 

    /// 지역 조명의 월드 좌표상 위치입니다.
    pub position: [f32; 3], 

    /// 지역 조명이 영향을 미치는 월드 좌표상 범위입니다.
    pub range: f32, 

    /// 지역 조명이 바라보는 월드 좌표상 방향입니다.
    pub direction: [f32; 3], 

    /// 지역 조명이 퍼지는 라디안 각도입니다.
    pub angle: f32, 
}

impl Default for LocalLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            color: gmm::Float4::ONE.into(), 
            position: gmm::Float3::ZERO.into(), 
            range: 0.0, 
            direction: gmm::Float3::ZERO.into(), 
            angle: 0.0 
        }
    }
}





/// 지역 조명 집합 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LocalLightSetLayout {
    /// 지역 조명 집합입니다.
    pub lights: [LocalLightDataLayout; MAX_LOCAL_LIGHTS], 

    /// 지역 조명의 개수입니다.
    pub num_lights: u32, 
    pub _padding0: [u8; 12] 
}

impl Default for LocalLightSetLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            lights: [LocalLightDataLayout::default(); MAX_LOCAL_LIGHTS], 
            num_lights: 0, 
            _padding0: [0; 12] 
        }
    }
}





/// 지역 조명 데이터 유니폼 버퍼
/// 
/// 지역 조명 유니폼 버퍼는 각 카메라 마다 한개씩 가지고 있습니다.
/// 각 카메라가 범위에 들어오는 지역 조명을 관리합니다.
/// 
#[derive(Debug, Clone)]
pub struct LocalLightUniform {
    inner: Arc<wgpu::Buffer> 
}

impl LocalLightUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<LocalLightSetLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl LocalLightUniform {
    /// 초기화되지 않은 새로운 지역 조명 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self { 
            inner: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES 
                }
            ).into() 
        }
    }

    /// 지역 조명 유니폼 버퍼 데이터를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: LocalLightSetLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut LocalLightSetLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (UNIFORM:{})", e);
                }
            }
        });

        // 제출된 작업이 끝날 때 까지 대기합니다.
        let index = queue.submit([]);
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(index));
    }

    /// 지역 조명 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}
