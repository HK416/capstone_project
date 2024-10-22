use std::{mem, num::NonZeroU64, sync::{Arc, OnceLock}};

use bytemuck::{Pod, Zeroable};

use crate::render::light::GlobalLightUniform;

use super::light::LocalLightUniform;



/// 카메라 데이터 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraDataLayout {
    /// 투영 변환 행렬과 뷰 변환 행렬이 곱해진 행렬 데이터입니다.
    pub proj_view: [f32; 16], 

    /// 카메라의 월드 좌표상 위치입니다.
    pub position: [f32; 3], 
    pub _padding0: [u8; 4], 

    /// 카메라의 월드 좌표상 바라보는 방향입니다.
    pub direction: [f32; 3], 
    pub _padding1: [u8; 4] 
}

impl Default for CameraDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            proj_view: gmm::Float4x4::IDENTITY.into(), 
            position: gmm::Float3::ZERO.into(), 
            _padding0: [0; 4], 
            direction: gmm::Float3::ZERO.into(), 
            _padding1: [0; 4] 
        }
    }
}





/// 카메라 데이터 유니폼 버퍼
#[derive(Debug, Clone)]
pub struct CameraUniform {
    inner: Arc<wgpu::Buffer>
}

impl CameraUniform {
    /// 유니폼 버퍼의 크기
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<CameraDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl CameraUniform {
    /// 초기화되지 않은 새로운 카메라 유니폼 버퍼를 생성합니다.
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

    /// 카메라 유니폼 버퍼 데이터를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: CameraDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut CameraDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

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

    /// 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

static_assertions::const_assert_ne!(CameraUniform::SIZE, 0);
static_assertions::const_assert_eq!(CameraUniform::SIZE as usize, mem::size_of::<CameraDataLayout>());





/// 카메라 데이터 쉐이더 리소스
#[derive(Debug)]
pub struct CameraResource {
    /// 카메라 데이터 유니폼 버퍼입니다.
    camera_uniform: CameraUniform, 

    /// 지역 조명 유니폼 버퍼입니다.
    local_light_uniform: LocalLightUniform, 

    /// 카메라 데이터의 [wgpu::BindGroup]입니다.
    bind_group: wgpu::BindGroup
}

impl CameraResource {
    /// 카메라 데이터 쉐이더 리소스의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
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
                                } 
                            }, 
                            count: None 
                        }, 
                        // 1번 바인딩: 전역 조명 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(GlobalLightUniform::SIZE))
                                } 
                            }, 
                            count: None 
                        }, 
                        // 2번 바인딩: 지역 조명 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(LocalLightUniform::SIZE))
                                } 
                            }, 
                            count: None 
                        }
                    ]
                }
            )
        })
    }
}

impl CameraResource {
    /// 새로운 카메라 데이터 쉐이더 리소스를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> Self {
        let name = name.unwrap_or("Unknown");
        let camera_uniform = CameraUniform::new(
            Some(&format!("CameraUniform({})", &name)), 
            device
        );
        let local_light_uniform = LocalLightUniform::new(
            Some(&format!("LocalLightUniform({})", &name)), 
            device
        );
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup(CameraResource({}))", &name)), 
                layout: &Self::bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: camera_uniform.buffer().as_entire_binding() 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: GlobalLightUniform::get_or_init(device).buffer().as_entire_binding() 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: local_light_uniform.buffer().as_entire_binding() 
                    } 
                ]
            }
        );

        Self { 
            camera_uniform, 
            local_light_uniform, 
            bind_group 
        }
    }

    /// 카메라 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn uniform(&self) -> &CameraUniform {
        &self.camera_uniform
    }

    /// 카메라 데이터의 [wgpu::BindGroup]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}





/// 삼인칭 시점 카메라의 데이터입니다.
#[derive(Debug, Clone)]
pub struct ThirdPersonCamera {
    pub target: crate::component::WorldID, 

    // 거리
    pub distance: f32, 

    /// 극선
    pub polar: f32,

    /// 방위각 
    pub azimuthal: f32, 
}
