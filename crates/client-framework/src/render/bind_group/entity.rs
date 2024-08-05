use std::ops;
use std::hash;
use std::sync::Arc;
use std::sync::OnceLock;
use std::cmp::Ordering;
use wgpu::util::DeviceExt;
use crate::render::variable::EntityUniform;



/// 쉐이더에서 오브젝트 데이터에 대한 변수 묶음 입니다.
#[derive(Debug)]
pub struct EntityBindGroup(wgpu::BindGroup);

impl EntityBindGroup {
    /// 바인드 그룹의 레이아웃을 가져옵니다.
    #[must_use]
    pub fn layout(device: &wgpu::Device) -> &wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(EntityBindGroup)"), 
                    entries: &[
                        // 오브젝트 데이터 
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        },
                        // 오브젝트 Ambient 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None, 
                        }, 
                        // 오브젝트 Ambient 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None,
                        }, 
                        // 오브젝트 Diffuse 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 3, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None, 
                        }, 
                        // 오브젝트 Diffuse 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 4, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        },
                        // 오브젝트 Normal 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 5, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None, 
                        }, 
                        // 오브젝트 Normal 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 6, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        },
                        // 오브젝트 Specular 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 7, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None, 
                        }, 
                        // 오브젝트 Specular 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 8, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        },
                        // 오브젝트 Emissive 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 9, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None, 
                        }, 
                        // 오브젝트 Emissive 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 10, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        },
                    ],
                },
            )
        })
    }

    /// 기본 텍스처 샘플러를 가져옵니다.
    #[must_use]
    pub fn get_default_sampler(device: &wgpu::Device) -> &'static wgpu::Sampler {
        static SAMPLER: OnceLock<wgpu::Sampler> = OnceLock::new();
        SAMPLER.get_or_init(|| {
            device.create_sampler(
                &wgpu::SamplerDescriptor {
                    label: Some("Sampler(Default)"), 
                    address_mode_u: wgpu::AddressMode::ClampToEdge, 
                    address_mode_v: wgpu::AddressMode::ClampToEdge, 
                    address_mode_w: wgpu::AddressMode::ClampToEdge, 
                    mag_filter: wgpu::FilterMode::Linear, 
                    min_filter: wgpu::FilterMode::Linear, 
                    mipmap_filter: wgpu::FilterMode::Linear, 
                    ..Default::default()
                }
            )
        })
    }

    /// 기본 `Ambient` 텍스처를 가져옵니다.
    #[must_use]
    pub fn get_default_ambient(
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> &'static wgpu::TextureView {
        static TEXTURE: OnceLock<wgpu::TextureView> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Ambient(Default))"), 
                    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, 
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    dimension: wgpu::TextureDimension::D2, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[255, 255, 255, 255]
            ).create_view(
                &wgpu::TextureViewDescriptor { ..Default::default() }
            )
        })
    }

    /// 기본 `Diffuse` 텍스처를 가져옵니다.
    #[must_use]
    pub fn get_default_diffuse(
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> &'static wgpu::TextureView {
        static TEXTURE: OnceLock<wgpu::TextureView> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Diffuse(Default))"), 
                    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, 
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    dimension: wgpu::TextureDimension::D2, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[255, 255, 255, 255]
            ).create_view(
                &wgpu::TextureViewDescriptor { ..Default::default() }
            )
        })
    }

    /// 기본 `Normal` 텍스처를 가져옵니다.
    #[must_use]
    pub fn get_default_normal(
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> &'static wgpu::TextureView {
        static TEXTURE: OnceLock<wgpu::TextureView> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Normal(Default))"), 
                    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, 
                    format: wgpu::TextureFormat::Rgba8Snorm,
                    dimension: wgpu::TextureDimension::D2, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[0, 0, 127, 127]
            ).create_view(
                &wgpu::TextureViewDescriptor { ..Default::default() }
            )
        })
    }

    /// 기본 `Specular` 텍스처를 가져옵니다.
    #[must_use]
    pub fn get_default_specular(
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> &'static wgpu::TextureView {
        static TEXTURE: OnceLock<wgpu::TextureView> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Specular(Default))"), 
                    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, 
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    dimension: wgpu::TextureDimension::D2, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[0, 0, 0, 0]
            ).create_view(
                &wgpu::TextureViewDescriptor { ..Default::default() }
            )
        })
    }

    /// 기본 `Emissive` 텍스처를 가져옵니다.
    #[must_use]
    pub fn get_default_emissive(
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> &'static wgpu::TextureView {
        static TEXTURE: OnceLock<wgpu::TextureView> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Emissive(Default))"), 
                    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, 
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    dimension: wgpu::TextureDimension::D2, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[0, 0, 0, 0]
            ).create_view(
                &wgpu::TextureViewDescriptor { ..Default::default() }
            )
        })
    }
}

impl EntityBindGroup {
    /// 새로운 오브젝트 바인드 그룹을 생성합니다.
    #[must_use]
    pub fn new(
        name: Option<&str>, 
        device: &wgpu::Device, 
        entity: &EntityUniform, 
        ambient_map: (&wgpu::TextureView, &wgpu::Sampler), 
        diffuse_map: (&wgpu::TextureView, &wgpu::Sampler), 
        normal_map: (&wgpu::TextureView, &wgpu::Sampler), 
        specular_map: (&wgpu::TextureView, &wgpu::Sampler), 
        emissive_map: (&wgpu::TextureView, &wgpu::Sampler)
    ) -> Arc<Self> {
        // 라벨을 생성합니다.
        let label = format!("BindGroup(Entity({}))", name.unwrap_or("Unknown"));

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&label), 
                layout: EntityBindGroup::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: wgpu::BindingResource::Buffer(
                            entity.as_entire_buffer_binding()
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::TextureView(
                            ambient_map.0
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: wgpu::BindingResource::Sampler(
                            ambient_map.1
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3, 
                        resource: wgpu::BindingResource::TextureView(
                            diffuse_map.0
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 4, 
                        resource: wgpu::BindingResource::Sampler(
                            diffuse_map.1
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5, 
                        resource: wgpu::BindingResource::TextureView(
                            normal_map.0
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 6, 
                        resource: wgpu::BindingResource::Sampler(
                            normal_map.1
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7, 
                        resource: wgpu::BindingResource::TextureView(
                            specular_map.0
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 8, 
                        resource: wgpu::BindingResource::Sampler(
                            specular_map.1
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9, 
                        resource: wgpu::BindingResource::TextureView(
                            emissive_map.0
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 10, 
                        resource: wgpu::BindingResource::Sampler(
                            emissive_map.1
                        ),
                    },
                ],
            },
        );

        Self(bind_group).into()
    }
}

impl ops::Deref for EntityBindGroup {
    type Target = wgpu::BindGroup;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for EntityBindGroup {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for EntityBindGroup {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for EntityBindGroup { }

impl PartialEq<Self> for EntityBindGroup {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for EntityBindGroup {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
