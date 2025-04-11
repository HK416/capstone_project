//! 조명 쉐이더 리소스와 관련된 코드를 관리합니다.
//! 

use std::sync::{Arc, OnceLock};

use crate::component::LightSetUniform;

/// 조명 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightResource(Arc<wgpu::BindGroup>);

impl LightResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(LightResource)"), 
                entries: &[
                    // 0번 바인딩: 조명 집합 데이터 유니폼 버퍼
                    LightSetUniform::bind_group_layout_entry(wgpu::ShaderStages::VERTEX_FRAGMENT, 0),
                    // 1번 바인딩: 그림자 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2번 바인딩: 그림자 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ]
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>, 
        device: &wgpu::Device, 
        light_uniform: &LightSetUniform, 
        light_texture: &wgpu::TextureView,
        light_sampler: &wgpu::Sampler,
    ) -> Self {
        Self(Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))), 
            layout: Self::bind_group_layout(device), 
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_uniform.as_entire_binding(), 
                }, 
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(light_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(light_sampler),
                },
            ],
        })))
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}
