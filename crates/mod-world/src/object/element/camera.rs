use std::{any::Any, sync::{Arc, OnceLock}};

use crate::render::{
    camera::CameraUniform, 
    light::{GlobalLightUniform, LocalLightUniform}
};



/// 게임 오브젝트에 연결되는 카메라 요소입니다.
#[derive(Debug)]
pub struct CameraElement {
    /// 카메라 유니폼 버퍼입니다.
    camera_uniform: CameraUniform, 

    /// 지역 조명의 유니폼 버퍼입니다.
    /// 
    /// ※ 차후 사용 예정
    /// 
    local_light_uniform: LocalLightUniform, 

    /// 카메라 유니폼 버퍼의 바인드 그룹입니다.
    bind_group: wgpu::BindGroup, 
}

impl CameraElement {
    pub fn bind_group_layout(device: &Arc<wgpu::Device>) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout"), 
                    entries: &[
                        // 0번 바인딩: 카메라 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None
                        }, 
                        // 1번 바인딩: 전역 조명 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None
                        }, 
                        // 2번 바인딩: 지역 조명 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None
                        }, 
                    ]
                }
            )
        })
    }

    pub fn new(name: Option<&str>, device: &Arc<wgpu::Device>) -> Self {
        let name = name.unwrap_or("Unknown");
        let camera_uniform = CameraUniform::new(Some(&format!("CameraUniform({})", &name)), device);
        let local_light_uniform = LocalLightUniform::new(Some(&format!("LocalLightUniform({})", &name)), device);
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", &name)), 
                layout: &Self::bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: camera_uniform.as_entire_binding(),
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: GlobalLightUniform::get(device).as_entire_binding()
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: local_light_uniform.as_entire_binding()
                    }, 
                ]
            }
        );

        Self { camera_uniform, local_light_uniform, bind_group }
    }

    /// 카메라 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn camera_uniform(&self) -> &CameraUniform {
        &self.camera_uniform
    }

    /// 바인드 그룹 레이아웃을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

static_assertions::assert_impl_all!(CameraElement: crate::object::Element, Any);
