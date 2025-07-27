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

/// 지형 방어막 재질 데이터입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageBarrierMaterialData {
    pub uri: String,
    pub tint: Float4,
    pub pattern: String,
}

impl StageBarrierMaterialData {
    pub fn as_layout(&self) -> StageBarrierMaterialDataLayout {
        StageBarrierMaterialDataLayout {
            tint: self.tint.into(),
        }
    }
}

/// 지형 방어막 재질 데이터 유니폼 버퍼의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StageBarrierMaterialDataLayout {
    pub tint: [f32; 4],
}

impl Default for StageBarrierMaterialDataLayout {
    fn default() -> Self {
        Self {
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// 지형 방어막 재질 데이터 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageBarrierMaterialUniform(Arc<wgpu::Buffer>);

impl StageBarrierMaterialUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<StageBarrierMaterialDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [`wgpu::BufferUsages`]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::COPY_DST)
        .union(wgpu::BufferUsages::MAP_WRITE);

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

    /// 새로우 유니폼 버퍼를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        data: StageBarrierMaterialDataLayout,
    ) -> Self {
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
        data: &StageBarrierMaterialDataLayout,
    ) {
        let contents = bytemuck::bytes_of(data).to_vec();
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    view.copy_from_slice(&contents);
                    capturable.unmap();
                }
                Err(e) => {
                    log::error!("failed to update uniform buffer! (REASON:{})", e);
                }
            });
    }

    /// 유니폼 버퍼를 갱신합니다.
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn update(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: &StageBarrierMaterialDataLayout,
    ) {
        let contents = bytemuck::bytes_of(data);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging(Material(StageBarrier)"),
            contents,
            usage: wgpu::BufferUsages::COPY_DST,
        });

        encoder.copy_buffer_to_buffer(&buffer, 0, &self.0, 0, copy_size);
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

static_assertions::const_assert_eq!(16, align_of::<StageBarrierMaterialDataLayout>());
static_assertions::const_assert_ne!(StageBarrierMaterialUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    StageBarrierMaterialUniform::SIZE as usize,
    core::mem::size_of::<StageBarrierMaterialDataLayout>()
);

/// 방어막 재질 쉐이더 리소스입니다.
pub struct StageBarrierMaterialResource;

impl StageBarrierMaterialResource {
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(StageBarrierMaterial)"),
                entries: &[
                    // 0번 바인딩: 유니폽 버퍼
                    StageBarrierMaterialUniform::bind_group_layout_entry(
                        wgpu::ShaderStages::FRAGMENT,
                        0,
                    ),
                    // 1번 바인딩: 메인 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2번 바인딩: 메인 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        material_uniform: &StageBarrierMaterialUniform,
        main_color_view: &wgpu::TextureView,
        main_color_sampler: &wgpu::Sampler,
    ) -> MaterialResource {
        MaterialResource {
            kind: MaterialKind::StageBarrier,
            bind_group: Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: material_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(main_color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(main_color_sampler),
                    },
                ],
            })),
        }
    }
}
