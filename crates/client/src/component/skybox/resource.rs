//! 스카이박스 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::OnceLock;

use crate::component::SkyboxUniform;

/// 스카이박스 쉐이더 리소스입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct SkyboxResource(wgpu::BindGroup);

impl SkyboxResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(SkyboxResource)"),
                entries: &[
                    // 0번 바인딩: 스카이박스 유니폼 버퍼
                    SkyboxUniform::bind_group_layout_entry(wgpu::ShaderStages::VERTEX_FRAGMENT, 0),
                    // 1번 바인딩: 스카이박스 큐브맵 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2번 바인딩: 스카이박스 큐브맵 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        uniform: &SkyboxUniform,
        texture: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        Self(bind_group)
    }
}
