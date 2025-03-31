use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};

use crate::GlobalLightUniform;

use super::LocalLightUniform;

/// ## Camera Data Layout
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraDataLayout {
    pub proj_view: [f32; 16],
    pub position_w: [f32; 3],
    pub _padding0: [u8; 4],
    pub direction_w: [f32; 3],
    pub _padding1: [u8; 4],
}

impl Default for CameraDataLayout {
    fn default() -> Self {
        Self {
            proj_view: [0.0; 16],
            position_w: [0.0; 3],
            _padding0: [0; 4],
            direction_w: [0.0; 3],
            _padding1: [0; 4],
        }
    }
}

/// ## Camera Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraUniform(Arc<wgpu::Buffer>);

impl CameraUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<CameraDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl CameraUniform {
    /// 초기화 되지 않은 새로운 카메라 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 카메라 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, _device: &wgpu::Device, _queue: &wgpu::Queue, data: CameraDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut CameraDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        // let index = queue.submit([]);
        // device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 카메라 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
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

        // let index = queue.submit([]);
        // device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
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

static_assertions::const_assert_ne!(CameraUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    CameraUniform::SIZE as usize,
    core::mem::size_of::<CameraDataLayout>()
);

/// ## Camera Shader Resource
#[derive(Debug)]
pub struct CameraResource {
    pub camera_uniform: CameraUniform,
    pub local_light_uniform: LocalLightUniform,
    pub bind_group: wgpu::BindGroup,
}

impl CameraResource {
    /// 카메라 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CameraResource)"),
                entries: &[
                    // 0번 바인딩: 카메라 데이터 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(CameraUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 1번 바인딩: 전역 조명 데이터 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(GlobalLightUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 2번 바인딩: 지역 조명 데이터 유니폼 버퍼 집합
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(LocalLightUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                ],
            })
        })
    }
}

impl CameraResource {
    /// 초기화되지 않은 새로운 카메라 쉐이더 리소스를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        let tag = &format!("Uniform(Camera({}))", label.unwrap_or("Unknown"));
        let camera_uniform = CameraUniform::uninit(Some(&tag), device);
        let global_light_uniform = GlobalLightUniform::get_or_uninit(device);
        let tag = &format!("Uniform(LocalLight({}))", label.unwrap_or("Unknown"));
        let local_light_uniform = LocalLightUniform::uninit(Some(tag), device);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: global_light_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: local_light_uniform.as_entire_binding(),
                },
            ],
        });

        Self {
            camera_uniform,
            local_light_uniform,
            bind_group,
        }
    }
}
