//! 조명 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::{Arc, OnceLock};

use crate::{asset::SamplerPool, component::LightUniform};

/// 그림자 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowResource(Arc<wgpu::BindGroup>);

impl ShadowResource {
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(ShadowResource)"),
                entries: &[
                    // 0번 바인딩: 조명 데이터 유니폼 버퍼
                    LightUniform::bind_group_layout_entry(wgpu::ShaderStages::VERTEX, 0),
                ],
            })
        })
    }

    pub fn new(label: Option<&str>, device: &wgpu::Device, light_resource: &LightResource) -> Self {
        Self(Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_resource.uniform.as_entire_binding(),
                }],
            },
        )))
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}

/// 조명 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightResource {
    pub uniform: LightUniform,
    pub view: Arc<wgpu::TextureView>,
    pub bind_group: Arc<wgpu::BindGroup>,
}

impl LightResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(LightResource)"),
                entries: &[
                    // 0번 바인딩: 조명 데이터 유니폼 버퍼
                    LightUniform::bind_group_layout_entry(wgpu::ShaderStages::VERTEX_FRAGMENT, 0),
                    // 1번 바인딩: 그림자 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
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
    ///
    /// # Panics
    /// 주어진 텍스처 포맷이 깊이-스텐실 형식이 아닌 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: u32,
        pool: &SamplerPool,
    ) -> Self {
        let uniform = LightUniform::uninit(label, device);
        let view = Self::create_depth_texture(label, device, format, size);
        let sampler = pool.get_or_init(
            device,
            &wgpu::SamplerDescriptor {
                label: Some("Sampler(Shadow)"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            },
        );
        let bind_group = Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        }));

        Self {
            uniform,
            view,
            bind_group,
        }
    }

    /// 깊이 텍스처를 생성합니다.
    ///
    /// # Panics
    /// 주어진 텍스처 포맷이 깊이-스텐실 형식이 아닌 경우 [`panic!`]을 호출합니다.
    ///
    fn create_depth_texture(
        label: Option<&str>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: u32,
    ) -> Arc<wgpu::TextureView> {
        assert!(
            format.is_depth_stencil_format(),
            "the given texture format must be depth-stencil format!"
        );
        Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(&format!("Depth({})", label.unwrap_or("Unknown"))),
                    size: wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        )
    }
}
