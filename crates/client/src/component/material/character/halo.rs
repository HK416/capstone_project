#![allow(dead_code)]
//! 캐릭터 헤일로 재질과 관련된 코드를 관리합니다.
//!

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

/// 캐릭터 헤일로 재질 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HaloMaterialData {
    pub uri: String,
    pub glossiness: f32,
    pub smoothness: f32,
    pub metallic: f32,
    pub emissive: Float4,
    pub main_color: String,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HaloMaterialDataLayout {
    pub glossiness: f32,
    pub smoothness: f32,
    pub metallic: f32,
    pub _padding0: [u8; 4],
    pub emissive: [f32; 4],
}

impl Default for HaloMaterialDataLayout {
    fn default() -> Self {
        Self {
            glossiness: 0.0,
            smoothness: 0.0,
            metallic: 0.0,
            _padding0: [0; 4],
            emissive: [0.0; 4],
        }
    }
}

///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaloMaterialUniform(Arc<wgpu::Buffer>);

impl HaloMaterialUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<HaloMaterialDataLayout>() as wgpu::BufferAddress;

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
    pub fn new(label: Option<&str>, device: &wgpu::Device, data: HaloMaterialDataLayout) -> Self {
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
        data: HaloMaterialDataLayout,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    {
                        let mut view = capturable.slice(..).get_mapped_range_mut();
                        let layout: &mut HaloMaterialDataLayout =
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
        data: HaloMaterialDataLayout,
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

/// 캐릭터 몸체 재질을 쉐이더 리소스입니다.
pub struct HaloMaterialResource;

impl HaloMaterialResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CharacterHaloMaterialResource)"),
                entries: &[
                    // 0번 바인딩: 캐릭터 헤일로 재질 데이터 유니폼 버퍼
                    HaloMaterialUniform::bind_group_layout_entry(wgpu::ShaderStages::FRAGMENT, 0),
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
        character_uniform: &HaloMaterialUniform,
        main_color_view: &wgpu::TextureView,
        main_color_sampler: &wgpu::Sampler,
    ) -> MaterialResource {
        MaterialResource {
            kind: MaterialKind::CharacterHalo,
            bind_group: Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: character_uniform.as_entire_binding(),
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
