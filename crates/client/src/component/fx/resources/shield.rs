use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::component::ParticleResource;

/// 방어막 유니폼 버퍼 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FxShieldDataLayout {
    pub color: [f32; 3],
    pub time: f32,
    pub rim_strength: f32,
    pub rim_power: f32,
    pub _padding0: [u8; 8],
}

impl Default for FxShieldDataLayout {
    fn default() -> Self {
        Self {
            color: [0.4862745098, 0.8156862745, 1.0],
            time: 0.0,
            rim_strength: 0.8,
            rim_power: 4.0,
            _padding0: [0; 8],
        }
    }
}

/// 방어막 데이터 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxShieldUniform(Arc<wgpu::Buffer>);

impl FxShieldUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<FxShieldDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
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

    /// 새로운 유니폼 버퍼를 생성합니다.
    pub fn new(device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform(Fx(Shield))"),
                contents: bytemuck::bytes_of(&FxShieldDataLayout::default()),
                usage: Self::USAGES,
            },
        )))
    }

    /// 유니폼 버퍼를 갱신합니다.
    pub fn update(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: &FxShieldDataLayout,
    ) {
        // 스테이징 버퍼를 생성합니다.
        let contents = bytemuck::bytes_of(data);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging(Uniform(Fx(Shield)))"),
            contents,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 버퍼 내용을 복사합니다.
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

/// 방어막 파티클 이펙트 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxShieldResource;

impl FxShieldResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(Fx(Shield))"),
                entries: &[
                    // 0번 바인딩: 유니폼 버퍼
                    FxShieldUniform::bind_group_layout_entry(wgpu::ShaderStages::FRAGMENT, 0),
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(device: &wgpu::Device, uniform_buffer: &FxShieldUniform) -> ParticleResource {
        ParticleResource::new(Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("BindGroup(Fx(Shield))"),
                layout: Self::bind_group_layout(device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            },
        )))
    }
}
