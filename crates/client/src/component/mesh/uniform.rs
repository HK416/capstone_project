#![allow(dead_code)]
//! 메쉬의 유니폼 버퍼와 관련된 코드를 관리합니다.
//!

use std::{num::NonZeroU64, ops::RangeBounds, sync::Arc};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// 변환 행렬 유니폼 버퍼의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TransformDataLayout {
    pub trans: [f32; 16],
}

impl Default for TransformDataLayout {
    fn default() -> Self {
        Self { trans: [0.0; 16] }
    }
}

/// 변환 행렬 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformUniform(Arc<wgpu::Buffer>);

impl TransformUniform {
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<TransformDataLayout>() as wgpu::BufferAddress;

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
    pub fn new(label: Option<&str>, device: &wgpu::Device, data: TransformDataLayout) -> Self {
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
        data: TransformDataLayout,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    {
                        let mut view = capturable.slice(..).get_mapped_range_mut();
                        let layout: &mut TransformDataLayout = bytemuck::from_bytes_mut(&mut view);
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
        data: TransformDataLayout,
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

static_assertions::const_assert_ne!(TransformUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    TransformUniform::SIZE as usize,
    core::mem::size_of::<TransformDataLayout>()
);

/// 스키닝된 메쉬 데이터 유니폼 버퍼의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SkinningDataLayout {
    pub quality: u32,
    pub num_bones: u32,
    pub _padding0: [u8; 8],
}

impl Default for SkinningDataLayout {
    fn default() -> Self {
        Self {
            quality: 0,
            num_bones: 0,
            _padding0: [0; 8],
        }
    }
}

/// 최대 뼈 노드의 개수입니다.
pub const MAX_BONES: usize = 256;

/// 바인드 포즈 뼈 변환 행렬 유니폼 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindposeUniform(Arc<wgpu::Buffer>);

impl BindposeUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        (core::mem::size_of::<[f32; 16]>() * MAX_BONES) as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages =
        wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST);

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
    ///
    /// # Panics
    /// 주어진 `data`의 개수가 `MAX_BONES`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vec<[f32; 16]>,
    ) -> Self {
        assert!(
            data.len() <= MAX_BONES,
            "the number of bindpose transforms is greater than the maximum number of bone nodes!"
        );
        unsafe { Self::new_unchecked(label, device, encoder, staging_buffers, data) }
    }

    /// 새로운 유니폼 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어진 `data`의 개수가 `MAX_BONES`보다 클 경우 정의되지 않은 동작을 수행합니다.
    ///
    pub unsafe fn new_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vec<[f32; 16]>,
    ) -> Self {
        // 스테이징(업로드) 버퍼를 생성합니다.
        let contents = bytemuck::cast_slice(&data);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Staging(Uniform({}))", label.unwrap_or("Unknown"))),
            contents,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 유니폼 버퍼를 생성합니다.
        let buffer = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        }));

        // 버퍼 데이터를 복사합니다.
        encoder.copy_buffer_to_buffer(&staging, 0, &buffer, 0, copy_size);
        staging_buffers.push(staging);

        Self(buffer)
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

/// 뼈 변환 행렬 유니폼 버퍼입니다.
pub struct BoneTransformUniform(Arc<wgpu::Buffer>);

impl BoneTransformUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        (core::mem::size_of::<[f32; 16]>() * MAX_BONES) as wgpu::BufferAddress;

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
    ///
    /// # Panics
    /// 주어진 뼈 변환 행렬의 개수가 `MAX_BONES`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(label: Option<&str>, device: &wgpu::Device, data: Vec<[f32; 16]>) -> Self {
        assert!(
            data.len() <= MAX_BONES,
            "the number of given bone transform is larger than the {}",
            MAX_BONES
        );

        const SIZE: usize = core::mem::size_of::<[f32; 16]>() * MAX_BONES;
        let mut contents = vec![0u8; SIZE];
        let data: &[u8] = bytemuck::cast_slice(&data);
        let count = data.len();
        let src = data.as_ptr();
        let dst = contents.as_mut_ptr();
        unsafe { core::ptr::copy_nonoverlapping(src, dst, count) };

        Self(Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform({})", label.unwrap_or("Unknown"))),
                contents: &contents,
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
    ///
    /// # Panics
    /// 주어진 뼈 변환 행렬의 개수가 `MAX_BONES`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(
        &self,
        _device: &wgpu::Device,
        _encoder: &mut wgpu::CommandEncoder,
        _staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vec<[f32; 16]>,
    ) {
        assert!(
            data.len() <= MAX_BONES,
            "the number of given bone transform is larger than the {}",
            MAX_BONES
        );

        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    {
                        let mut view = capturable.slice(..).get_mapped_range_mut();
                        let count = data.len() * core::mem::size_of::<[f32; 16]>();
                        let src = data.as_ptr() as *const u8;
                        let dst = view.as_mut_ptr();
                        // Safe: `data`의 크기는 유니폼 버퍼의 크기보다 작습니다.
                        unsafe { core::ptr::copy_nonoverlapping(src, dst, count) };
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
    /// 주어진 뼈 변환 행렬의 개수가 `MAX_BONES`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn update(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vec<[f32; 16]>,
    ) {
        assert!(
            data.len() <= MAX_BONES,
            "the number of given bone transform is larger than the {}",
            MAX_BONES
        );

        // 스테이징 버퍼를 생성합니다.
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging(Uniform)"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 버퍼의 내용을 복사합니다.
        encoder.copy_buffer_to_buffer(&self.0, 0, &buffer, 0, Self::SIZE);
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
