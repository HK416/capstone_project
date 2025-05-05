//! 조명 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::OnceLock;

use super::{
    LightSetUniform, LightUniform, MAX_LIGHTS, SHADOW_FORMAT, SHADOW_MAP_SIZE,
};

/// 그림자 쉐이더 리소스입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct ShadowResource {
    pub view: wgpu::TextureView,
    pub uniform: LightUniform,
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
                    // 0번 바인딩: 조명 데이터 유니폼 버퍼
                    LightUniform::bind_group_layout_entry(
                        wgpu::ShaderStages::VERTEX, 
                        0
                    ),
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(label: Option<&str>, device: &wgpu::Device, view: wgpu::TextureView) -> Self {
        let uniform = LightUniform::uninit(label, device);
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
    pub uniform: LightSetUniform,
    texture: wgpu::Texture,
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
                    // 0번 바인딩: 조명 데이터 집합 유니폼 버퍼
                    LightSetUniform::bind_group_layout_entry(wgpu::ShaderStages::FRAGMENT, 0),
                    // 1번 바인딩: 그림자 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
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
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        let uniform = LightSetUniform::uninit(label, device);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Depth({})", label.unwrap_or("Unknown"))),
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: MAX_LIGHTS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler(Shadow)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
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
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            uniform,
            texture,
            bind_group,
        }
    }

    /// 주어진 인덱스에 해당하는 그림자 쉐이더 리소스를 반환합니다.
    ///
    /// # Panics
    /// 주어진 인덱스가 범위를 벗어나는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn get(&self, label: Option<&str>, device: &wgpu::Device, index: usize) -> ShadowResource {
        assert!(index < MAX_LIGHTS, "index out of range!");
        let view = self.texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: index as u32,
            array_layer_count: Some(1),
            ..Default::default()
        });

        ShadowResource::new(label, device, view)
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
