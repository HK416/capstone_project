use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};
use mod_network::components::Float4;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::component::{MaterialKind, MaterialResource};

/// 점령 지역 재질 데이터입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureZoneMaterialData {
    pub uri: String,
    pub color0: Float4,
    pub color1: Float4,
}

/// 점령 지역 재질 데이터 유니폼 버퍼 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CaptureZoneMaterialDataLayout {
    pub color0: [f32; 4],
    pub color1: [f32; 4],
    pub timer: f32,
    pub _padding: [u8; 12],
}

impl Default for CaptureZoneMaterialDataLayout {
    fn default() -> Self {
        Self {
            color0: [0.0; 4],
            color1: [0.0; 4],
            timer: 0.0,
            _padding: [0; 12],
        }
    }
}

/// 점령 지역 재질 데이터 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureZoneMaterialUniform(Arc<wgpu::Buffer>);

impl CaptureZoneMaterialUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<CaptureZoneMaterialDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [`wgpu::BufferUsages`]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);

    /// [wgpu::BindGroupLayoutEntry]를 반환합니다.
    pub fn bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: unsafe { Some(NonZeroU64::new_unchecked(Self::SIZE)) },
            },
            count: None,
        }
    }

    /// 새로운 유니폼 버퍼를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        data: CaptureZoneMaterialDataLayout,
    ) -> Self {
        // 유니폼 버퍼를 생성합니다.
        Self(Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
                contents: bytemuck::bytes_of(&data),
                usage: Self::USAGES,
            },
        )))
    }

    /// 유니폼 버퍼를 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(
        &self,
        _device: &wgpu::Device,
        _encoder: &mut wgpu::CommandEncoder,
        _staging_buffers: &mut Vec<wgpu::Buffer>,
        data: CaptureZoneMaterialDataLayout,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    {
                        let mut view = capturable.slice(..).get_mapped_range_mut();
                        let layout: &mut CaptureZoneMaterialDataLayout =
                            bytemuck::from_bytes_mut(&mut view);
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
        data: CaptureZoneMaterialDataLayout,
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

static_assertions::const_assert_ne!(CaptureZoneMaterialUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    CaptureZoneMaterialUniform::SIZE as usize,
    core::mem::size_of::<CaptureZoneMaterialDataLayout>()
);

/// 점령 지역 재질 쉐이더 리소스입니다.
pub struct CaptureZoneMaterialResource;

impl CaptureZoneMaterialResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CaptureZoneMaterialResource)"),
                entries: &[
                    // 0번 바인딩: 점령 지역 재질 데이터 유니폼 버퍼
                    CaptureZoneMaterialUniform::bind_group_layout_entry(0),
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        capture_zone_uniform: &CaptureZoneMaterialUniform,
    ) -> MaterialResource {
        MaterialResource {
            kind: MaterialKind::CaptureZone,
            bind_group: Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: capture_zone_uniform.as_entire_binding(),
                }],
            })),
        }
    }
}
