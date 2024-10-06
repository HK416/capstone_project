use std::sync::OnceLock;

use crate::render::{
    camera::CameraUniform, 
    light::{GlobalLightUniform, LocalLightUniform}
};

use super::WorldID;



#[derive(Debug)]
pub struct Camera {
    /// 카메라의 유니폼 버퍼입니다.
    camera_uniform: CameraUniform, 

    /// 카메라의 지역 조명 유니폼 버퍼입니다.
    local_light_uniform: LocalLightUniform, 

    /// 카메라의 [wgpu::BindGroup]입니다.
    bind_group: wgpu::BindGroup, 
}

impl Camera {
    /// 카메라의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[must_use]
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(CameraObject)"), 
                    entries: &[
                        // 0번 바인딩: 카메라 유니폼
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
                        // 1번 바인딩: 카메라 유니폼
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None,
                        }, 
                        // 2번 바인딩: 카메라 유니폼
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
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

    /// 새로운 카메라 요소를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> Self {
        let name = name.unwrap_or("Unknown");

        let camera_uniform = CameraUniform::new(Some(&format!("CameraUniform({})", &name)), device);
        let local_light_uniform = LocalLightUniform::new(Some(&format!("LocalLightUniform({})", &name)), device);
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup(Camera({}))", &name)), 
                layout: &Self::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: camera_uniform.as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: GlobalLightUniform::get(device).as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: local_light_uniform.as_entire_binding(), 
                    }, 
                ], 
            }, 
        );

        Self { 
            camera_uniform, 
            local_light_uniform, 
            bind_group 
        }
    }

    /// 카메라 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn camera_uniform(&self) -> &CameraUniform {
        &self.camera_uniform
    }

    /// 지역 조명 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn local_light_uniform(&self) -> &LocalLightUniform {
        &self.local_light_uniform
    }

    /// 카메라의 [wgpu::BindGroup]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}



/// 삼인칭 시점 카메라의 데이터입니다.
#[derive(Debug, Clone)]
pub struct ThirdPersonCamera {
    pub target: WorldID, 

    // 거리
    pub distance: f32, 

    /// 극선
    pub polar: f32,

    /// 방위각 
    pub azimuthal: f32, 
}
