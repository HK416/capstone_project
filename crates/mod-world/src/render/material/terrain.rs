use std::{mem, num::NonZeroU64, sync::{Arc, OnceLock}};

use bytemuck::{Pod, Zeroable};

use crate::render::pool::{SamplerPool, TexturePool, TextureViewPool};

use super::MaterialResource;



/// 지형 재질 데이터 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TerrainMaterialDataLayout {
    /// 재질의 매끄러운 정도입니다. (0.0..=1.0)
    pub glossiness: f32, 

    /// 재질의 부드러운 정도입니다. (0.0..=1.0)
    pub smoothness: f32, 

    /// 재질의 금속성 정도입니다. (0.0..=1.0)
    pub metallic: f32, 
    
    /// 지형의 최대 높이입니다.
    pub max_height: f32, 

    /// 재질의 `Ambient` 색깔입니다.
    pub ambient: [f32; 4], 

    /// 재질의 `Diffuse` 색깔입니다.
    pub diffuse: [f32; 4], 

    /// 재질의 `Specular` 색깔입니다.
    pub specular: [f32; 4], 

    /// 재질의 `Emissive` 색깔입니다.
    pub emissive: [f32; 4], 
}

impl Default for TerrainMaterialDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            glossiness: 0.5, 
            smoothness: 0.5, 
            metallic: 0.25, 
            max_height: 1.0, 
            ambient: [0.2, 0.2, 0.2, 1.0], 
            diffuse: [0.85, 0.85, 0.85, 1.0], 
            specular: [1.0, 1.0, 1.0, 1.0], 
            emissive: [1.0, 1.0, 1.0, 1.0] 
        }
    }
}





/// 지형 재질 데이터 유니폼 버퍼
#[derive(Debug, Clone)]
pub struct TerrainMaterialUniform {
    inner: Arc<wgpu::Buffer>
}

impl TerrainMaterialUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<TerrainMaterialDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl TerrainMaterialUniform {
    /// 초기화되지 않은 새로운 지형 재질 데이터 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self { 
            inner: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES
                }
            ).into() 
        }
    }

    /// 지형 재질 유니폼 버퍼 데이터를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: TerrainMaterialDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut TerrainMaterialDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (UNIFORM:{})", e);
                }
            }
        });

        // 제출된 작업이 끝날 때 까지 대기합니다.
        let index = queue.submit([]);
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(index));
    }

    /// 지형 재질 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

static_assertions::const_assert_ne!(TerrainMaterialUniform::SIZE, 0);
static_assertions::const_assert_eq!(TerrainMaterialUniform::SIZE as usize, mem::size_of::<TerrainMaterialDataLayout>());





/// 지형 재질 쉐이더 리소스
#[derive(Debug)]
pub struct TerrainMaterialResource {
    /// 지형 재질 쉐이더 리소스의 이름입니다.
    name: String, 

    /// 지형 재질 데이터 유니폼 버퍼입니다.
    material_uniform: TerrainMaterialUniform, 

    /// 지형 재질의 [wgpu::BindGroup]입니다.
    bind_group: wgpu::BindGroup 
}

impl TerrainMaterialResource {
    /// 지형 재질 쉐이더 리소스의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(TerrainMaterialResource)"), 
                    entries: &[
                        // 0번 바인딩: 지형 재질 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(TerrainMaterialUniform::SIZE))
                                } 
                            }, 
                            count: None
                        }, 
                        // 1번 바인딩: Diffuse 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None 
                        }, 
                        // 2번 바인딩: Diffuse 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None 
                        }, 
                        // 3번 바인딩: Specular 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 3, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None 
                        }, 
                        // 4번 바인딩: Specular 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 4, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None 
                        }, 
                        // 5번 바인딩: Emissive 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 5, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None 
                        }, 
                        // 6번 바인딩: Emissive 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 6, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None 
                        }, 
                        // 7번 바인딩: Normal 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 7, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None 
                        }, 
                        // 8번 바인딩: Normal 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 8, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None 
                        }, 
                        // 9번 바인딩: Height 텍스처
                        wgpu::BindGroupLayoutEntry {
                            binding: 9, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Texture { 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true }, 
                                view_dimension: wgpu::TextureViewDimension::D2, 
                                multisampled: false 
                            }, 
                            count: None 
                        }, 
                        // 10번 바인딩: Height 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 10, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None 
                        }, 
                    ]
                }
            )
        })
    }
}

impl TerrainMaterialResource {
    /// 새로운 지형 재질 쉐이더 리소스를 생성합니다.
    #[must_use]
    pub fn new(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        desc: &TerrainMaterialDescriptor
    ) -> Self {
        let name = desc.name.clone();
        let material_uniform = TerrainMaterialUniform::new(
            Some(&format!("TerrainMaterialUniform({})", &name)), 
            device
        );
        material_uniform.write(device, queue, TerrainMaterialDataLayout {
            glossiness: desc.glossiness, 
            smoothness: desc.smoothness, 
            metallic: desc.metallic, 
            max_height: desc.max_height, 
            ambient: desc.ambient, 
            diffuse: desc.diffuse, 
            specular: desc.specular, 
            emissive: desc.emissive, 
            ..Default::default()
        });

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup(TerrainMaterialResource({}))", &name)), 
                layout: &Self::bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: material_uniform.buffer().as_entire_binding() 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::TextureView(&desc.diffuse_map)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: wgpu::BindingResource::Sampler(&desc.diffuse_sampler)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 3, 
                        resource: wgpu::BindingResource::TextureView(&desc.specular_map)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 4, 
                        resource: wgpu::BindingResource::Sampler(&desc.specular_sampler)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 5, 
                        resource: wgpu::BindingResource::TextureView(&desc.emissive_map)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 6, 
                        resource: wgpu::BindingResource::Sampler(&desc.emissive_sampler)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 7, 
                        resource: wgpu::BindingResource::TextureView(&desc.normal_map)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 8, 
                        resource: wgpu::BindingResource::Sampler(&desc.normal_sampler)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 9, 
                        resource: wgpu::BindingResource::TextureView(&desc.height_map)
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 10, 
                        resource: wgpu::BindingResource::Sampler(&desc.height_sampler)
                    } 
                ]
            }
        );

        Self { name, material_uniform, bind_group }
    }
    
    /// 지형 재질 쉐이더 리소스의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 지형 재질 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn material_uniform(&self) -> &TerrainMaterialUniform {
        &self.material_uniform
    }
}

impl MaterialResource for TerrainMaterialResource {
    #[inline]
    #[must_use]
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}





/// 지형 재질 쉐이더 리소스 설명자
#[derive(Debug, Clone)]
pub struct TerrainMaterialDescriptor {
    /// 지형 재질 쉐이더 리소스의 이름입니다.
    pub name: String, 

    /// 재질의 매끄러운 정도입니다. (0.0..=1.0)
    pub glossiness: f32, 

    /// 재질의 부드러운 정도입니다. (0.0..=1.0)
    pub smoothness: f32, 

    /// 재질의 금속성 정도입니다. (0.0..=1.0)
    pub metallic: f32, 

    /// 지형의 최대 높이입니다.
    pub max_height: f32, 

    /// 재질의 `Ambient` 색깔입니다.
    pub ambient: [f32; 4], 

    /// 재질의 `Diffuse` 색깔입니다.
    pub diffuse: [f32; 4], 

    /// 재질의 `Specular` 색깔입니다.
    pub specular: [f32; 4], 

    /// 재질의 `Emissive` 색깔입니다.
    pub emissive: [f32; 4], 

    /// 재질의 `Diffuse` 텍스처 뷰입니다.
    pub diffuse_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Diffuse` 텍스처 샘플러입니다.
    pub diffuse_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Specular` 텍스처 뷰입니다.
    pub specular_map: Arc<wgpu::TextureView>,

    /// 재질의 `Specular` 텍스처 샘플러입니다.
    pub specular_sampler: Arc<wgpu::Sampler>,  

    /// 재질의 `Emissive` 텍스처 뷰입니다.
    pub emissive_map: Arc<wgpu::TextureView>, 

    /// 재질의  `Emissive` 텍스처 샘플러입니다.
    pub emissive_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Normal` 텍스처 뷰입니다.
    pub normal_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Normal` 텍스처 샘플러입니다.
    pub normal_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Height` 텍스처 뷰입니다.
    pub height_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Height` 텍스처 샘플러입니다.
    pub height_sampler: Arc<wgpu::Sampler> 
}

impl TerrainMaterialDescriptor {
    pub fn new<S>(device: &wgpu::Device, queue: &wgpu::Queue, name: S) -> Self 
    where S: Into<String> {
        // 기본 하얀색 텍스처를 가져옵니다.
        let default_texture = TexturePool::white(device, queue);
        let default_texture = TextureViewPool::get_or_init(
            &default_texture, 
            &wgpu::TextureViewDescriptor::default()
        );

        // 기본 노멀 텍스처를 가져옵니다.
        let default_normal = TexturePool::normal(device, queue);
        let default_normal = TextureViewPool::get_or_init(
            &default_normal, 
            &wgpu::TextureViewDescriptor::default()
        );

        // 기본 높이 텍스처를 가져옵니다.
        let default_height = TexturePool::height(device, queue);
        let default_height = TextureViewPool::get_or_init(
            &default_height, 
            &wgpu::TextureViewDescriptor::default()
        );

        // 기본 텍스처 샘플러를 가져옵니다.
        let default_sampler = SamplerPool::linear(device);

        Self {
            name: name.into(), 
            glossiness: 0.5, 
            smoothness: 0.5, 
            metallic: 0.25, 
            max_height: 1.0, 
            ambient: [0.2, 0.2, 0.2, 1.0], 
            diffuse: [0.85, 0.85, 0.85, 1.0], 
            specular: [1.0, 1.0, 1.0, 1.0], 
            emissive: [1.0, 1.0, 1.0, 1.0], 
            diffuse_map: default_texture.clone(), 
            diffuse_sampler: default_sampler.clone(), 
            specular_map: default_texture.clone(), 
            specular_sampler: default_sampler.clone(), 
            emissive_map: default_texture.clone(), 
            emissive_sampler: default_sampler.clone(), 
            normal_map: default_normal.clone(), 
            normal_sampler: default_sampler.clone(), 
            height_map: default_height.clone(), 
            height_sampler: default_sampler.clone() 
        }
    }
}
