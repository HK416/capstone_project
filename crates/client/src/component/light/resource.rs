#![allow(dead_code)]
//! 조명 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::{Arc, OnceLock};

use super::{
    GlobalLightUniform, LightTransformUniform, LocalLightSetUniform, MAX_LIGHTS, SHADOW_FORMAT,
};

/// 그림자 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowResource {
    pub view: wgpu::TextureView,
    pub uniform: LightTransformUniform,
    pub bind_group: wgpu::BindGroup,
}

impl ShadowResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(ShadowResource)"),
                entries: &[
                    // 0번 바인딩: 조명 변환 행렬 데이터 유니폼 버퍼
                    LightTransformUniform::bind_group_layout_entry(wgpu::ShaderStages::VERTEX, 0),
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(label: Option<&str>, device: &wgpu::Device, view: wgpu::TextureView) -> Self {
        let uniform = LightTransformUniform::uninit(label, device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: Self::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        Self {
            view,
            uniform,
            bind_group,
        }
    }
}

/// 조명 집합 쉐이더 리소스입니다.
pub struct LightSetResource {
    pub global_light_uniform: GlobalLightUniform,
    global_light_shadow: Arc<ShadowResource>,

    pub local_light_uniform: LocalLightSetUniform,
    local_light_shadows: Vec<Arc<ShadowResource>>,

    bind_group: wgpu::BindGroup,
}

impl LightSetResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(LightSetResource)"),
                entries: &[
                    // 0번 바인딩: 전역 조명 데이터 집합 유니폼 버퍼
                    GlobalLightUniform::bind_group_layout_entry(wgpu::ShaderStages::FRAGMENT, 0),
                    // 1번 바인딩: 지역 조명 데이터 집합 유니폼 버퍼
                    LocalLightSetUniform::bind_group_layout_entry(wgpu::ShaderStages::FRAGMENT, 1),
                    // 2번 바인딩: 전역 조명 그림자 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 3번 바인딩: 로컬 조명 그림자 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 4번 바인딩: 조명 그림자 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
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
        global_light_texture_size: u32,
        local_light_texture_size: u32,
    ) -> Self {
        // 전역 조명과 관련된 쉐이더 리소스를 생성합니다.
        let global_light_uniform = GlobalLightUniform::uninit(label, device);
        let global_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("GlobalLightDepth({})", label.unwrap_or("Unknown"))),
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            size: wgpu::Extent3d {
                width: global_light_texture_size,
                height: global_light_texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let global_light_shadow = Arc::new(ShadowResource::new(
            Some(&format!("GlobalLight({})", label.unwrap_or("Unknown"))),
            device,
            global_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        ));

        // 지역 조명과 관련된 쉐이더 리소스를 생성합니다.
        let local_light_uniform = LocalLightSetUniform::uninit(label, device);
        let local_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("LocalLightDepth({})", label.unwrap_or("Unknown"))),
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            size: wgpu::Extent3d {
                width: local_light_texture_size,
                height: local_light_texture_size,
                depth_or_array_layers: MAX_LIGHTS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let local_light_shadows = (0..MAX_LIGHTS)
            .map(|i| {
                Arc::new(ShadowResource::new(
                    Some(&format!("LocalLight({})", label.unwrap_or("Unknown"))),
                    device,
                    local_texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: i as u32,
                        array_layer_count: Some(1),
                        ..Default::default()
                    }),
                ))
            })
            .collect();

        // 그림자 맵 샘플러를 생성합니다.
        // `*_filter`가 Linear인 경우 하드웨어 PCF를 수행합니다.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler(Shadow)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: global_light_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: local_light_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &global_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&local_texture.create_view(
                        &wgpu::TextureViewDescriptor {
                            dimension: Some(wgpu::TextureViewDimension::D2Array),
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            global_light_uniform,
            global_light_shadow,
            local_light_uniform,
            local_light_shadows,
            bind_group,
        }
    }

    /// 주어진 인덱스에 해당하는 지역 조명 그림자 쉐이더 리소스를 반환합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 범위를 벗어나는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn get_local(&self, index: usize) -> Arc<ShadowResource> {
        self.local_light_shadows
            .get(index)
            .cloned()
            .expect("index out of range!")
    }

    /// 전역 조명 그림자 쉐이더 리소스를 반환합니다.
    pub fn get_global(&self) -> Arc<ShadowResource> {
        self.global_light_shadow.clone()
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
