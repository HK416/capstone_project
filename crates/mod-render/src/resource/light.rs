use std::{
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};

/// ## Global Light Uniform Buffer Data Layout
/// 전역 조명에는 `Directional Light`가 있습니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlobalLightDataLayout {
    pub color: [f32; 4],
    pub direction_w: [f32; 3],
    pub _padding0: [u8; 4],
}

impl Default for GlobalLightDataLayout {
    fn default() -> Self {
        Self {
            color: [0.0; 4],
            direction_w: [0.0; 3],
            _padding0: [0; 4],
        }
    }
}

/// ## Global Lights Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalLightUniform(Arc<wgpu::Buffer>);

impl GlobalLightUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<GlobalLightDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl GlobalLightUniform {
    /// 전역 조명 유니폼 버퍼를 가져오거나 초기화 되지 않은 전역 조명 유니폼 버퍼를 생성합니다.
    pub fn get_or_uninit(device: &wgpu::Device) -> &'static Self {
        static THIS: OnceLock<GlobalLightUniform> = OnceLock::new();
        THIS.get_or_init(|| {
            Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Uniform(GlobalLight)"),
                mapped_at_creation: false,
                size: Self::SIZE,
                usage: Self::USAGES,
            })))
        })
    }

    /// 전역 조명 유니폼 버퍼의 내용을 갱신합니다.
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: GlobalLightDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut GlobalLightDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer (REASONS:{})", e);
                }
            });

        let index = queue.submit([]);
        device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 범위에 해당하는 슬라이스된 유니폼 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }

    /// 유니폼 버퍼의 [`wgpu::BindingResource`]를 반환합니다.
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.0.as_entire_binding()
    }

    /// 유니폼 버퍼의 [`wgpu::BufferBinding`]을 반환합니다.
    pub fn as_entire_buffer_binding(&self) -> wgpu::BufferBinding<'_> {
        self.0.as_entire_buffer_binding()
    }
}

static_assertions::const_assert_ne!(GlobalLightUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    GlobalLightUniform::SIZE as usize,
    core::mem::size_of::<GlobalLightDataLayout>()
);

/// ## Local Light Uniform Buffer Data Layout
/// 지역 조명에는 `Point Light`, `Spot Light`가 있습니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LocalLightDataLayout {
    pub color: [f32; 4],
    pub position_w: [f32; 3],
    pub range_w: f32,
    pub direction_w: [f32; 3],
    pub angle_w: f32,
}

impl Default for LocalLightDataLayout {
    fn default() -> Self {
        Self {
            color: [0.0; 4],
            position_w: [0.0; 3],
            range_w: 0.0,
            direction_w: [0.0; 3],
            angle_w: 0.0,
        }
    }
}

/// 최대 지역 조명의 개수입니다.
const MAX_LOCAL_LIGHTS: usize = 32;

/// ## Local Light Uniform Buffer Data Set Layout
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LocalLightSetLayout {
    pub lights: [LocalLightDataLayout; MAX_LOCAL_LIGHTS],
    pub num_lights: u32,
    pub _padding0: [u8; 12],
}

impl Default for LocalLightSetLayout {
    fn default() -> Self {
        Self {
            lights: [LocalLightDataLayout::default(); MAX_LOCAL_LIGHTS],
            num_lights: 0,
            _padding0: [0; 12],
        }
    }
}

/// ## Local Light Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLightUniform(Arc<wgpu::Buffer>);

impl LocalLightUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<LocalLightSetLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl LocalLightUniform {
    /// 새로운 초기화되지 않은 지역 조명 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 지역 조명 유니폼 버퍼의 내용을 갱신합니다.
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: LocalLightSetLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut LocalLightSetLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer (REASONS:{})", e);
                }
            });

        let index = queue.submit([]);
        device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 범위에 해당하는 슬라이스된 유니폼 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }

    /// 유니폼 버퍼의 [`wgpu::BindingResource`]를 반환합니다.
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.0.as_entire_binding()
    }

    /// 유니폼 버퍼의 [`wgpu::BufferBinding`]을 반환합니다.
    pub fn as_entire_buffer_binding(&self) -> wgpu::BufferBinding<'_> {
        self.0.as_entire_buffer_binding()
    }
}

static_assertions::const_assert_ne!(LocalLightUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    LocalLightUniform::SIZE as usize,
    core::mem::size_of::<LocalLightSetLayout>()
);
