use std::{mem, num::NonZeroU64, sync::{Arc, OnceLock}};

use bytemuck::{Pod, Zeroable};

use crate::render::pool::{SamplerPool, TexturePool, TextureViewPool};

use super::MaterialResource;



/// 모델 재질 데이터 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ModelMaterialDataLayout {
    /// 재질의 매끄러운 정도입니다. (0.0..=1.0)
    pub glossiness: f32, 

    /// 재질의 부드러운 정도입니다. (0.0..=1.0)
    pub smoothness: f32, 

    /// 재질의 금속성 정도입니다. (0.0..=1.0)
    pub metallic: f32, 
    pub _padding0: [u8; 4], 

    /// 재질의 `Ambient` 색깔입니다.
    pub ambient: [f32; 4], 

    /// 재질의 `Diffuse` 색깔입니다.
    pub diffuse: [f32; 4], 

    /// 재질의 `Specular` 색깔입니다.
    pub specular: [f32; 4], 

    /// 재질의 `Emissive` 색깔입니다.
    pub emissive: [f32; 4], 
}

impl Default for ModelMaterialDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            glossiness: 0.5, 
            smoothness: 0.5, 
            metallic: 0.25, 
            _padding0: [0; 4], 
            ambient: [0.2, 0.2, 0.2, 1.0], 
            diffuse: [0.85, 0.85, 0.85, 1.0], 
            specular: [1.0, 1.0, 1.0, 1.0], 
            emissive: [1.0, 1.0, 1.0, 1.0] 
        }
    }
}





/// 모델 재질 데이터 유니폼 버퍼
#[derive(Debug, Clone)]
pub struct ModelMaterialUniform {
    inner: Arc<wgpu::Buffer>
}

impl ModelMaterialUniform {
    /// 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<ModelMaterialDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl ModelMaterialUniform {
    /// 초기화되지 않은 새로운 모델 재질 데이터 유니폼 버퍼를 생성합니다.
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

    /// 모델 재질 유니폼 버퍼 데이터를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: ModelMaterialDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut ModelMaterialDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

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

    /// 모델 재질 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

static_assertions::const_assert_ne!(ModelMaterialUniform::SIZE, 0);
static_assertions::const_assert_eq!(ModelMaterialUniform::SIZE as usize, mem::size_of::<ModelMaterialDataLayout>());




/// 모델 재질 쉐이더 리소스
#[derive(Debug)]
pub struct ModelMaterialResource {
    /// 모델 재질 쉐이더 리소스의 이름입니다.
    name: String, 

    /// 모델 재질 데이터 유니폼 버퍼입니다.
    material_uniform: ModelMaterialUniform, 

    /// 모델 재질의 [wgpu::BindGroup]입니다.
    bind_group: wgpu::BindGroup 
}

impl ModelMaterialResource {
    /// 모델 재질 쉐이더 리소스의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(ModelMaterialResource)"), 
                    entries: &[
                        // 0번 바인딩: 모델 재질 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(ModelMaterialUniform::SIZE))
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
                    ]
                }
            )
        })
    }
}

impl ModelMaterialResource {
    /// 새로운 모델 재질 쉐이더 리소스를 생성합니다.
    #[must_use]
    pub fn new(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        desc: &ModelMaterialDescriptor
    ) -> Self {
        let name = desc.name.clone();
        let material_uniform = ModelMaterialUniform::new(
            Some(&format!("ModelMaterialUniform({})", &name)), 
            device
        );
        material_uniform.write(device, queue, ModelMaterialDataLayout {
            glossiness: desc.glossiness, 
            smoothness: desc.smoothness, 
            metallic: desc.metallic, 
            ambient: desc.ambient, 
            diffuse: desc.diffuse, 
            specular: desc.specular, 
            emissive: desc.emissive, 
            ..Default::default()
        });

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup(ModelMaterialResource({}))", &name)), 
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
                    } 
                ]
            }
        );

        Self { name, material_uniform, bind_group }
    }

    /// 모델 재질 쉐이더 리소스의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 모델 재질 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn material_uniform(&self) -> &ModelMaterialUniform {
        &self.material_uniform
    }
}

impl MaterialResource for ModelMaterialResource {
    #[inline]
    #[must_use]
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}





/// 모델 재질 쉐이더 리소스 설명자
#[derive(Debug, Clone)]
pub struct ModelMaterialDescriptor {
    /// 모델 재질 쉐이더 리소스의 이름입니다.
    pub name: String, 

    /// 재질의 매끄러운 정도입니다. (0.0..=1.0)
    pub glossiness: f32, 

    /// 재질의 부드러운 정도입니다. (0.0..=1.0)
    pub smoothness: f32, 

    /// 재질의 금속성 정도입니다. (0.0..=1.0)
    pub metallic: f32, 

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
    pub normal_sampler: Arc<wgpu::Sampler> 
}

impl ModelMaterialDescriptor {
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

        // 기본 텍스처 샘플러를 가져옵니다.
        let default_sampler = SamplerPool::linear(device);

        Self {
            name: name.into(), 
            glossiness: 0.5, 
            smoothness: 0.5, 
            metallic: 0.25, 
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
            normal_sampler: default_sampler.clone()
        }
    }
}
