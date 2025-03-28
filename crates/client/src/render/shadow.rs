use std::sync::{Arc, OnceLock};

/// ## Shadow Map Shader Resource
#[derive(Debug)]
pub struct ShadowMapResource {
    pub texture: Arc<wgpu::TextureView>,
    pub sampler: Arc<wgpu::Sampler>,
    pub bind_group: wgpu::BindGroup,
}

impl ShadowMapResource {
    /// 그림자 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(ShadowMapResource)"),
                entries: &[
                    // 0번 바인딩: Shadow 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: Shadow 텍스처 샘플러
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
}

impl ShadowMapResource {
    /// 새로운 Shadow Map 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        // 렌더 텍스처를 생성합니다.
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Texture(Shadow({}))", &label.unwrap_or("Unknown"))),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            dimension: wgpu::TextureDimension::D2,
            format,
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_texture =
            Arc::new(shadow_texture.create_view(&wgpu::TextureViewDescriptor::default()));
        // 텍스처 샘플러를 생성합니다.
        let shadow_sampler = Arc::new(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Sampler(Shadow({}))", label.unwrap_or("Unknwon"))),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        }));
        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!(
                "BindGroup(Shadow({}))",
                label.unwrap_or("Unknown")
            )),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        Self {
            texture: shadow_texture,
            sampler: shadow_sampler,
            bind_group,
        }
    }

    /// 그림자 텍스처의 크기를 재설정합니다.
    pub fn resize(
        self,
        label: Option<&str>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        // 렌더 텍스처를 생성합니다.
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Texture(Shadow({}))", label.unwrap_or("Unknown"))),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            dimension: wgpu::TextureDimension::D2,
            format,
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_texture =
            Arc::new(shadow_texture.create_view(&wgpu::TextureViewDescriptor::default()));

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!(
                "BindGroup(Shadow({}))",
                label.unwrap_or("Unknown")
            )),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        Self {
            texture: shadow_texture,
            sampler: self.sampler,
            bind_group,
        }
    }
}
