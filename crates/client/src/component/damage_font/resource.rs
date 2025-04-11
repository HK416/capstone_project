//! 데미지 폰트 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::OnceLock;

use crate::component::DamageFontUniform;

/// 데미지 폰트 쉐이더 리소스입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct DamageFontResource(wgpu::BindGroup);

impl DamageFontResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(DamageFont)"),
                entries: &[
                    // 0번 바인딩: 데미지 폰트 유니폼 버퍼
                    DamageFontUniform::bind_group_layout_entry(
                        wgpu::ShaderStages::VERTEX_FRAGMENT,
                        0,
                    ),
                    // 1번 바인딩: 데미지 폰트 텍스처
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
                    // 2번 바인딩: 데미지 폰트 텍스처 샘플러
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

    /// 새로운 데미지 폰트 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        damage_uniform: &DamageFontUniform,
    ) -> Self {
        Self(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: damage_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        }))
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}
