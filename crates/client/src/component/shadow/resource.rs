//! 그림자 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq)]
pub struct ShadowResource {
    format: wgpu::TextureFormat,
    shadow_texture: wgpu::Texture,
    shadow_texture_sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
}

impl ShadowResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(ShadowResource)"),
                entries: &[
                    // 0번 바인딩: 그림자 렌더 타겟
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: 그림자 렌더 타겟 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            })
        })
    }

    /// 주어진 크기의 그림자 텍스처를 생성합니다.
    fn create_shadow_texture(
        width: u32,
        height: u32,
        depth_or_array_layers: u32,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture(Shadow)"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers,
            },
            dimension: wgpu::TextureDimension::D2,
            format,
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                .union(wgpu::TextureUsages::TEXTURE_BINDING),
            view_formats: &[],
        })
    }

    /// 그림자 텍스처 샘플러를 생성합니다.
    fn create_shadow_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler(Shadow)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        })
    }

    /// 새로운 그림자 쉐이더 리소스를 생성합니다.
    pub fn new(
        width: u32,
        height: u32,
        depth_or_array_layers: u32,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shadow_texture =
            Self::create_shadow_texture(width, height, depth_or_array_layers, device, format);
        let texture_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_texture_sampler = Self::create_shadow_sampler(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Shadow)"),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_texture_sampler),
                },
            ],
        });

        Self {
            format,
            shadow_texture,
            shadow_texture_sampler,
            bind_group,
        }
    }

    /// 기존의 그림자 쉐이더 리소스로부터 새로운 쉐이더 리소스를 생성합니다.
    pub fn recreate(
        self,
        width: u32,
        height: u32,
        depth_or_array_layers: u32,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shadow_texture =
            Self::create_shadow_texture(width, height, depth_or_array_layers, device, format);
        let texture_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_texture_sampler = self.shadow_texture_sampler;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(Shadow)"),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_texture_sampler),
                },
            ],
        });

        Self {
            format,
            shadow_texture,
            shadow_texture_sampler,
            bind_group,
        }
    }
}
