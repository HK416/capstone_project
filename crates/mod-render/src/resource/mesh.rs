use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};

/// ## World Transform Uniform Buffer Data Layout
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

/// ## World Transform Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformUniform(Arc<wgpu::Buffer>);

impl TransformUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<TransformDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl TransformUniform {
    /// 초기화되지 않은 월드 변환 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 월드 변환 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: TransformDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut TransformDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        let index = queue.submit([]);
        device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 월드 변환 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Vec<u8>,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    view.copy_from_slice(&data);

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
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

static_assertions::const_assert_ne!(TransformUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    TransformUniform::SIZE as usize,
    core::mem::size_of::<TransformDataLayout>()
);

/// ## Skinned Mesh Uniform Buffer Data Layout
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

/// ## Skinned Mesh Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinningUniform(Arc<wgpu::Buffer>);

impl SkinningUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<SkinningDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl SkinningUniform {
    /// 초기화되지 않은 새로운 스키닝 메쉬 데이터 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 스키닝 메쉬 데이터 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: SkinningDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut SkinningDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        let index = queue.submit([]);
        device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 스키닝 메쉬 데이터 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Vec<u8>,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    view.copy_from_slice(&data);

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
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

static_assertions::const_assert_ne!(SkinningUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    SkinningUniform::SIZE as usize,
    core::mem::size_of::<SkinningDataLayout>()
);

/// 최대 뼈의 개수입니다.
const MAX_BONES: usize = 256;

/// ## Bone Transform Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoneTransformUniform(Arc<wgpu::Buffer>);

impl BoneTransformUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        (core::mem::size_of::<f32>() * 16 * MAX_BONES) as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl BoneTransformUniform {
    /// 초기화되지 않은 뼈 변환 행렬 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 뼈 변환 행렬 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: Vec<[f32; 16]>) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut [[f32; 16]; MAX_BONES] = bytemuck::from_bytes_mut(&mut view);

                    let mut iter = data.into_iter();
                    for dst in layout.iter_mut() {
                        let src = iter.next().unwrap_or_default();
                        dst.copy_from_slice(&src);
                    }

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
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

/// ## Mesh Shader Resource
#[derive(Debug)]
pub struct MeshResource {
    pub transform_uniform: TransformUniform,
    pub skinning_uniform: SkinningUniform,
    pub bindpose_uniform: BoneTransformUniform,
    pub bone_trans_uniform: BoneTransformUniform,
    pub bind_group: wgpu::BindGroup,
}

impl MeshResource {
    /// 메쉬 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(MeshResource)"),
                entries: &[
                    // 0번 바인딩: 월드 변환 행렬 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(TransformUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 1번 바인딩: 스키닝 데이터 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(SkinningUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 2번 바인딩: 바인드 포즈 변환 행렬 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(BoneTransformUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 3번 바인딩: 뼈 변환 행렬 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(BoneTransformUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                ],
            })
        })
    }
}

impl MeshResource {
    /// 초기화되지 않은 새로운 메쉬 쉐이더 리소스를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        let tag = &format!("Uniform(Transform({}))", label.unwrap_or("Unknown"));
        let transform_uniform = TransformUniform::uninit(Some(tag), device);
        let tag = &format!("Uniform(Skinning({}))", label.unwrap_or("Unknown"));
        let skinning_uniform = SkinningUniform::uninit(Some(tag), device);
        let tag = &format!("Uniform(Bindpose({}))", label.unwrap_or("Unknown"));
        let bindpose_uniform = BoneTransformUniform::uninit(Some(tag), device);
        let tag = &format!("Uniform(BoneTransform({}))", label.unwrap_or("Unknown"));
        let bone_trans_uniform = BoneTransformUniform::uninit(Some(tag), device);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: transform_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: skinning_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bindpose_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bone_trans_uniform.as_entire_binding(),
                },
            ],
        });

        Self {
            transform_uniform,
            skinning_uniform,
            bindpose_uniform,
            bone_trans_uniform,
            bind_group,
        }
    }
}
