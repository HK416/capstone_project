mod buffer;
pub use self::buffer::*;

mod layout;
pub use self::layout::*;

mod projection;
pub use self::projection::*;

use std::sync::Arc;
use std::sync::OnceLock;

use crate::render::light::direction::DirectionLightBuffer;
use crate::render::light::point::PointLightBuffer;
use crate::render::light::spot::SpotLightBuffer;



/// 메인 카메라를 식별하는 데이터입니다.
#[derive(Debug)]
pub struct MainCamera;


/// 카메라 컴포넌트 타입입니다.
pub type CameraComponent = Arc<CameraObject>;

/// 3차원 카메라 오브젝트를 나타내는 데이터입니다.
#[derive(Debug)]
pub struct CameraObject {
    name: String, 
    camera_buffer: Arc<CameraObjectBuffer>, 
    #[allow(dead_code)] point_light_buffer: Arc<PointLightBuffer>, // 향후 사용 예정
    #[allow(dead_code)] spot_light_buffer: Arc<SpotLightBuffer>, // 향후 사용 예정
    bind_group: wgpu::BindGroup, 
}

impl CameraObject {
    /// 3차원 오브젝트의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(CameraObject)"), 
                    entries: &[
                        // 0번 바인딩: 카메라 오브젝트 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None, 
                        }, 
                        // 1번 바인딩: 방향 조명 데이터 유니폼
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None, 
                        }, 
                        // 2번 바인딩: 카메라 가시거리 내 점 조명 데이터 유니폼
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None, 
                        }, 
                        // 3번 바인딩: 카메라 가시거리 내 spot 조명 데이터 유니폼
                        wgpu::BindGroupLayoutEntry {
                            binding: 3, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None, 
                        }, 
                    ]
                }
            )
        })
    }
}

impl CameraObject {
    /// 새로운 카메라 오브젝트 데이터를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> CameraComponent {
        // 디버깅 라벨을 생성합니다.
        let name = format!("CameraObject({})", name.unwrap_or("Unknown"));

        // 유니폼 버퍼를 생성합니다.
        let camera_buffer = CameraObjectBuffer::new(Some(&format!("Uniform({})", name)), device);
        let point_light_buffer = PointLightBuffer::new(Some(&format!("Uniform(PointLights({}))", name)), device);
        let spot_light_buffer = SpotLightBuffer::new(Some(&format!("Uniform(SpotLights({}))", name)), device);

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", name)), 
                layout: &Self::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: camera_buffer.as_entire_binding(),
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: DirectionLightBuffer::get(device).as_entire_binding(),
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: point_light_buffer.as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 3, 
                        resource: spot_light_buffer.as_entire_binding(), 
                    }, 
                ], 
            }
        );

        Self { 
            name, 
            camera_buffer, 
            point_light_buffer, 
            spot_light_buffer, 
            bind_group 
        }.into()
    }

    /// 유니폼 버퍼를 갱신합니다.
    pub fn update(&self, queue: &wgpu::Queue, data: CameraDataLayout) {
        let name = self.name.clone();
        let capturable = self.camera_buffer.clone();
        self.camera_buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            if result.is_ok() {
                let mut view = capturable.slice(..).get_mapped_range_mut();
                let layout: &mut CameraDataLayout = bytemuck::from_bytes_mut(&mut view);
                *layout = data;
                drop(view);
                capturable.unmap();
            } else {
                log::warn!("Failed to write uniform buffer! (name: {})", name);
            }
        });
        queue.submit([]);
    }
}

impl CameraObject {
    /// 컴포넌트의 이름을 반환합니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 컴포넌트의 유니폼 버퍼를 반환합니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &CameraObjectBuffer {
        &self.camera_buffer
    }

    /// 컴포넌트의 [wgpu::BindGroup]을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
