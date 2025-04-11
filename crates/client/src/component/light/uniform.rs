//! 조명 데이터 유니폼 버퍼와 관련된 코드를 관리합니다.
//! 

use std::{num::NonZeroU64, ops::RangeBounds, sync::Arc};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// 조명 데이터 유니폼 버퍼의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightDataLayout {
    pub proj_view: [f32; 16],
    pub position_w: [f32; 3],
    pub _padding0: [u8; 4],
    pub color: [f32; 3],
    pub _padding1: [u8; 4],
}

impl Default for LightDataLayout {
    fn default() -> Self {
        Self {
            proj_view: [0.0; 16],
            position_w: [0.0; 3],
            _padding0: [0; 4],
            color: [0.0; 3],
            _padding1: [0; 4],
        }
    }
}

/// 조명 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightUniform(Arc<wgpu::Buffer>);

impl LightUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<LightDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);

    /// [wgpu::BindGroupLayoutEntry]를 반환합니다.
    pub fn bind_group_layout_entry(
        visibility: wgpu::ShaderStages,
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: unsafe { Some(NonZeroU64::new_unchecked(Self::SIZE)) },
            },
            count: None,
        }
    }

    /// 새로운 유니폼 버퍼를 생성합니다.
    pub fn new(label: Option<&str>, device: &wgpu::Device, data: LightDataLayout) -> Self {
        Self(Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
                contents: bytemuck::bytes_of(&data),
                usage: Self::USAGES,
            },
        )))
    }

    /// 초기화되지 않은 새로운 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 유니폼 버퍼를 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(
        &self,
        _device: &wgpu::Device,
        _encoder: &mut wgpu::CommandEncoder,
        _staging_buffers: &mut Vec<wgpu::Buffer>,
        data: LightDataLayout,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    {
                        let mut view = capturable.slice(..).get_mapped_range_mut();
                        let layout: &mut LightDataLayout = bytemuck::from_bytes_mut(&mut view);
                        *layout = data;
                    }
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });
    }

    /// 유니폼 버퍼를 갱신합니다.
    ///
    /// # Panics
    /// 주어진 `contents`가 유니폼 버퍼의 크기와 다른 경우 [`panic!`]을 호출합니다.
    ///
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn update(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: LightDataLayout,
    ) {
        // 스테이징 버퍼를 생성합니다.
        let contents = bytemuck::bytes_of(&data);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging(Uniform)"),
            contents,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 버퍼의 내용을 복사합니다.
        encoder.copy_buffer_to_buffer(&self.0, 0, &buffer, 0, copy_size);
        staging_buffers.push(buffer);
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
}

static_assertions::const_assert_ne!(LightUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    LightUniform::SIZE as usize,
    core::mem::size_of::<LightDataLayout>()
);

/// 최대 조명의 개수입니다.
pub const MAX_LIGHTS: usize = 24;

/// 조명 집합 유니폼 버퍼의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightSetDataLayout {
    pub lights: [LightDataLayout; MAX_LIGHTS],
    pub num_lights: u32,
    pub _padding0: [u8; 12],
}

impl Default for LightSetDataLayout {
    fn default() -> Self {
        Self {
            lights: [LightDataLayout::default(); MAX_LIGHTS],
            num_lights: 0,
            _padding0: [0; 12],
        }
    }
}

/// 조명 집합 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightSetUniform(Arc<wgpu::Buffer>);

impl LightSetUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<LightSetDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);

    /// [wgpu::BindGroupLayoutEntry]를 반환합니다.
    pub fn bind_group_layout_entry(
        visibility: wgpu::ShaderStages,
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: unsafe { Some(NonZeroU64::new_unchecked(Self::SIZE)) },
            },
            count: None,
        }
    }

    /// 새로운 유니폼 버퍼를 생성합니다.
    pub fn new(label: Option<&str>, device: &wgpu::Device, data: &LightSetDataLayout) -> Self {
        Self(Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
                contents: bytemuck::bytes_of(data),
                usage: Self::USAGES,
            },
        )))
    }

    /// 초기화되지 않은 새로운 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 유니폼 버퍼를 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(
        &self,
        _device: &wgpu::Device,
        _encoder: &mut wgpu::CommandEncoder,
        _staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Box<LightSetDataLayout>,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    {
                        let mut view = capturable.slice(..).get_mapped_range_mut();
                        let layout: &mut LightSetDataLayout = bytemuck::from_bytes_mut(&mut view);
                        *layout = *data;
                    }
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });
    }

    /// 유니폼 버퍼를 갱신합니다.
    ///
    /// # Panics
    /// 주어진 `contents`가 유니폼 버퍼의 크기와 다른 경우 [`panic!`]을 호출합니다.
    ///
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn update(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: &LightSetDataLayout,
    ) {
        // 스테이징 버퍼를 생성합니다.
        let contents = bytemuck::bytes_of(data);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging(Uniform)"),
            contents,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 버퍼의 내용을 복사합니다.
        encoder.copy_buffer_to_buffer(&self.0, 0, &buffer, 0, copy_size);
        staging_buffers.push(buffer);
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
}

static_assertions::const_assert_ne!(LightSetUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    LightSetUniform::SIZE as usize,
    core::mem::size_of::<LightSetDataLayout>()
);
