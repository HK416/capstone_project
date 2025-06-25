#![allow(dead_code)]
//! 캐릭터 헤일로 재질과 관련된 코드를 관리합니다.
//!

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};

use crate::component::{MaterialKind, MaterialResource};

/// 캐릭터 헤일로 재질 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HaloMaterialData {
    pub uri: String,
    pub main_color: String,
}

/// 캐릭터 몸체 재질을 쉐이더 리소스입니다.
pub struct HaloMaterialResource;

impl HaloMaterialResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CharacterHaloMaterialResource)"),
                entries: &[
                    // 0번 바인딩: 메인 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: 메인 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
        main_color_view: &wgpu::TextureView,
        main_color_sampler: &wgpu::Sampler,
    ) -> MaterialResource {
        MaterialResource {
            kind: MaterialKind::CharacterHalo,
            bind_group: Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(main_color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(main_color_sampler),
                    },
                ],
            })),
        }
    }
}
