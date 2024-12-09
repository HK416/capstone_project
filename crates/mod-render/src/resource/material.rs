use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};

use crate::{SamplerPool, TexturePool, TextureViewPool};

/// ## Material Uniform Buffer Data Layout
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MaterialDataLayout {
    pub glossiness: f32,
    pub smoothness: f32,
    pub metallic: f32,
    pub bump_scale: f32,
    pub parallax: f32,
    pub strength: f32,
    pub _padding: [u8; 8],
    pub albedo: [f32; 4],
    pub specular: [f32; 4],
    pub emissive: [f32; 4],
}

impl Default for MaterialDataLayout {
    fn default() -> Self {
        Self {
            glossiness: 0.0,
            smoothness: 0.0,
            metallic: 0.0,
            bump_scale: 0.0,
            parallax: 0.0,
            strength: 0.0,
            _padding: [0; 8],
            albedo: [0.0; 4],
            specular: [0.0; 4],
            emissive: [0.0; 4],
        }
    }
}

/// ## Material Uniform Buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialUniform(Arc<wgpu::Buffer>);

impl MaterialUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress =
        core::mem::size_of::<MaterialDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [`wgpu::BufferUsages`]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl MaterialUniform {
    /// 초기화되지 않은 새로운 재질 데이터 유니폼 버퍼를 생성합니다.
    pub fn uninit(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: Self::SIZE,
            usage: Self::USAGES,
        })))
    }

    /// 재질 데이터 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: MaterialDataLayout) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    let layout: &mut MaterialDataLayout = bytemuck::from_bytes_mut(&mut view);

                    *layout = data;

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        let index = queue.submit([]);
        device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 재질 데이터 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Vec<u8>,
    ) {
        let capturable = self.0.clone();
        self.0
            .slice(..)
            .map_async(wgpu::MapMode::Write, move |result| match result {
                Ok(_) => {
                    let mut view = capturable.slice(..).get_mapped_range_mut();
                    view.copy_from_slice(&data);

                    drop(view);
                    capturable.unmap();
                }
                Err(e) => {
                    log::warn!("failed to update uniform buffer! (REASON:{})", e)
                }
            });

        let index = queue.submit([]);
        device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 범위에 해당하는 슬라이스된 유니폼 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }

    /// 유니폼 버퍼의 [`wgpu::BindingResource`]를 반환합니다.
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        self.0.as_entire_binding()
    }

    /// 유니폼 버퍼의 [`wgpu::BufferBinding`]을 반환합니다.
    pub fn as_entire_buffer_binding(&self) -> wgpu::BufferBinding<'_> {
        self.0.as_entire_buffer_binding()
    }
}

static_assertions::const_assert_ne!(MaterialUniform::SIZE, 0);
static_assertions::const_assert_eq!(
    MaterialUniform::SIZE as usize,
    core::mem::size_of::<MaterialDataLayout>()
);

/// ## Material Shader Resource Descriptor
#[derive(Debug, Clone)]
pub struct MaterialDescriptor {
    pub name: String,
    pub layout: MaterialDataLayout,
    pub albedo_map: Arc<wgpu::TextureView>,
    pub albedo_sampler: Arc<wgpu::Sampler>,
    pub specular_map: Arc<wgpu::TextureView>,
    pub specular_sampler: Arc<wgpu::Sampler>,
    pub emissive_map: Arc<wgpu::TextureView>,
    pub emissive_sampler: Arc<wgpu::Sampler>,
    pub normal_map: Arc<wgpu::TextureView>,
    pub normal_sampler: Arc<wgpu::Sampler>,
    pub parallax_map: Arc<wgpu::TextureView>,
    pub parallax_sampler: Arc<wgpu::Sampler>,
    pub occlusion_map: Arc<wgpu::TextureView>,
    pub occlusion_sampler: Arc<wgpu::Sampler>,
}

impl MaterialDescriptor {
    pub fn new(name: &str, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let black_texture = TexturePool::black(device, queue);
        let black_texture_view =
            TextureViewPool::get_or_init(&black_texture, &wgpu::TextureViewDescriptor::default());
        let normal_texture = TexturePool::normal(device, queue);
        let normal_texture_view =
            TextureViewPool::get_or_init(&normal_texture, &wgpu::TextureViewDescriptor::default());
        let height_texture = TexturePool::height(device, queue);
        let height_texture_view =
            TextureViewPool::get_or_init(&height_texture, &wgpu::TextureViewDescriptor::default());
        let sampler = SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default());

        Self {
            name: name.to_string(),
            layout: MaterialDataLayout::default(),
            albedo_map: black_texture_view.clone(),
            albedo_sampler: sampler.clone(),
            specular_map: black_texture_view.clone(),
            specular_sampler: sampler.clone(),
            emissive_map: black_texture_view.clone(),
            emissive_sampler: sampler.clone(),
            normal_map: normal_texture_view.clone(),
            normal_sampler: sampler.clone(),
            parallax_map: height_texture_view.clone(),
            parallax_sampler: sampler.clone(),
            occlusion_map: height_texture_view.clone(),
            occlusion_sampler: sampler.clone(),
        }
    }
}

/// ## Material Shader Resource
#[derive(Debug)]
pub struct MaterialResource {
    pub name: String,
    pub material_uniform: MaterialUniform,
    pub bind_group: wgpu::BindGroup,
}

impl MaterialResource {
    /// 재질 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(MaterialResource)"),
                entries: &[
                    // 0번 바인딩: 재질 데이터 유니폼 버퍼
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: unsafe {
                                Some(NonZeroU64::new_unchecked(MaterialUniform::SIZE))
                            },
                        },
                        count: None,
                    },
                    // 1번 바인딩: Albedo 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 2번 바인딩: Albedo 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 3번 바인딩: Specular 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 4번 바인딩: Specular 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 5번 바인딩: Emissive 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 6번 바인딩: Emissive 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 7번 바인딩: Normal 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 8번 바인딩: Normal 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 9번 바인딩: Parallax 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 10번 바인딩: Parallax 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 10,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // 11번 바인딩: Occlusion 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 11,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 12번 바인딩: Occlusion 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 12,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
        })
    }
}

impl MaterialResource {
    /// 새로운 재질 쉐이더 리소스를 생성합니다.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, desc: &MaterialDescriptor) -> Self {
        let tag = format!("Uniform(Material({}))", desc.name);
        let material_uniform = MaterialUniform::uninit(Some(&tag), device);
        material_uniform.update(device, queue, desc.layout);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("BindGroup(Material({}))", desc.name)),
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: material_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&desc.albedo_map),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&desc.albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&desc.specular_map),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&desc.specular_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&desc.emissive_map),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&desc.emissive_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&desc.normal_map),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&desc.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&desc.parallax_map),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&desc.parallax_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: wgpu::BindingResource::TextureView(&desc.occlusion_map),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: wgpu::BindingResource::Sampler(&desc.occlusion_sampler),
                },
            ],
        });

        Self {
            name: desc.name.to_string(),
            material_uniform,
            bind_group,
        }
    }
}
