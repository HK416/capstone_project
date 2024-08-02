use std::ops;
use std::hash;
use std::sync::Arc;
use std::sync::OnceLock;
use std::cmp::Ordering;

use crate::components::CameraUniform;
use crate::components::DirLightUniform;
use crate::components::PointLightUniform;
use crate::components::SpotLightUniform;



/// 쉐이더 전반적으로 사용되는 데이터에 대한 쉐이더 변수 묶음 입니다.
#[derive(Debug)]
pub struct GlobalBindGroup(wgpu::BindGroup);

impl GlobalBindGroup {
    /// 바인드 그룹의 레이아웃을 가져옵니다.
    #[must_use]
    pub fn layout(device: &wgpu::Device) -> &wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(GlobalBindGroup)"), 
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::all(), 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::all(), 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::all(), 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                        wgpu::BindGroupLayoutEntry {
                            binding: 3, 
                            visibility: wgpu::ShaderStages::all(), 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                    ]
                }
            )
        })
    }
}

impl GlobalBindGroup {
    #[must_use]
    pub fn new(
        name: Option<&str>, 
        device: &wgpu::Device, 
        camera: &CameraUniform
    ) -> Arc<Self> {
        // 라벨을 생성합니다.
        let label = format!("BindGroup(Global({}))", name.unwrap_or("Unknown"));

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&label), 
                layout: GlobalBindGroup::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::Buffer(
                            camera.as_entire_buffer_binding()
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::Buffer(
                            DirLightUniform::get(device).as_entire_buffer_binding()
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::Buffer(
                            PointLightUniform::get(device).as_entire_buffer_binding()
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::Buffer(
                            SpotLightUniform::get(device).as_entire_buffer_binding()
                        ), 
                    }, 
                ]
            }
        );

        Self(bind_group).into()
    }
}

impl ops::Deref for GlobalBindGroup {
    type Target = wgpu::BindGroup;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for GlobalBindGroup {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for GlobalBindGroup {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for GlobalBindGroup { }

impl PartialEq<Self> for GlobalBindGroup {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for GlobalBindGroup {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
