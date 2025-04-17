use std::sync::OnceLock;

/// Weighted Blended Order-Independent Transparency에 사용되는 쉐이더 리소스입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct WeightedBlendedOITResource {
    pub accum_render_target: wgpu::TextureView,
    pub reveal_render_target: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

impl WeightedBlendedOITResource {
    /// 누적(accumulate) 값 렌더 타켓 텍스처의 포맷입니다.
    pub const ACCUM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    /// 노출(revealage) 값 렌더 타겟 텍스처의 포맷입니다.
    pub const REVEAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(WeightedBlendedOIT)"),
                entries: &[
                    // 0번 바인딩: 누적 값 렌더 타겟 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 1번 바인딩: 노출 값 렌더 타겟 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    /// 주어진 크기의 누적 값 렌더 타겟 텍스처를 생성합니다.
    fn create_accumulate_texture(
        width: u32,
        height: u32,
        device: &wgpu::Device,
    ) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("RenderTarget(Accumulate)"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                format: Self::ACCUM_FORMAT,
                dimension: wgpu::TextureDimension::D2,
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    .union(wgpu::TextureUsages::TEXTURE_BINDING),
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// 주어진 크기의 노출 값 렌더 타겟 텍스처를 생성합니다.
    fn create_reveal_texture(width: u32, height: u32, device: &wgpu::Device) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("RenderTarget(Revealage)"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                format: Self::REVEAL_FORMAT,
                dimension: wgpu::TextureDimension::D2,
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    .union(wgpu::TextureUsages::TEXTURE_BINDING),
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        let accum_render_target = Self::create_accumulate_texture(width, height, device);
        let reveal_render_target = Self::create_reveal_texture(width, height, device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup(WeightedBlendedOIT)"),
            layout: Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&accum_render_target),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&reveal_render_target),
                },
            ],
        });

        Self {
            accum_render_target,
            reveal_render_target,
            bind_group,
        }
    }
}
