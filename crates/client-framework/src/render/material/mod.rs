mod buffer;
pub use self::buffer::*;

mod layout;
pub use self::layout::*;

mod sampler;
pub use self::sampler::*;

mod texture;
pub use self::texture::*;

use std::sync::Arc;
use std::sync::OnceLock;



/// 재질 컴포넌트 타입입니다.
pub type MaterialComponent = Arc<Material>;

/// 3차원 메쉬의 재질을 나타내는 데이터입니다.
#[derive(Debug)]
pub struct Material {
    name: String, 
    buffer: Arc<MaterialBuffer>,  
    bind_group: wgpu::BindGroup, 
}

impl Material {
    /// 3차원 메쉬 재질의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(Material)"), 
                    entries: &[
                        // 0번 바인딩: 재질 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None, 
                        }, 
                        // 1번 바인딩: `Diffuse` 텍스처
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
                        // 2번 바인딩: `Diffuse` 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        }, 
                        // 3번 바인딩: `Specular` 텍스처
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
                        // 4번 바인딩: `Specular` 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 4, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        }, 
                        // 5번 바인딩: `Normal` 텍스처
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
                        // 6번 바인딩: `Normal` 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 6, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        }, 
                        // 7번 바인딩: `Emissive` 텍스처
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
                        // 8번 바인딩: `Emissive` 텍스처 샘플러
                        wgpu::BindGroupLayoutEntry {
                            binding: 8, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering
                            ), 
                            count: None, 
                        }, 
                    ]
                }
            )
        })
    }
}

impl Material {
    fn new(
        name: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        diffuse: gmm::Float4, 
        specular: gmm::Float4, 
        emissive: gmm::Float4, 
        diffuse_map: Arc<wgpu::TextureView>, 
        diffuse_sampler: Arc<wgpu::Sampler>, 
        specular_map: Arc<wgpu::TextureView>, 
        specular_sampler: Arc<wgpu::Sampler>, 
        normal_map: Arc<wgpu::TextureView>, 
        normal_sampler: Arc<wgpu::Sampler>, 
        emissive_map: Arc<wgpu::TextureView>, 
        emissive_sampler: Arc<wgpu::Sampler>
    ) -> MaterialComponent {
        // 디버깅 라벨을 생성합니다.
        let name = format!("Material({})", name.unwrap_or("Unknown"));

        // 유니폼 버퍼를 생성합니다.
        let buffer = MaterialBuffer::new(Some(&format!("Uniform({})", name)), device);

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", name)), 
                layout: &Self::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: buffer.as_entire_binding()
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::TextureView(
                            &diffuse_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: wgpu::BindingResource::Sampler(
                            &diffuse_sampler
                        )
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 3, 
                        resource: wgpu::BindingResource::TextureView(
                            &specular_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 4, 
                        resource: wgpu::BindingResource::Sampler(
                            &specular_sampler
                        )
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 5, 
                        resource: wgpu::BindingResource::TextureView(
                            &normal_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 6, 
                        resource: wgpu::BindingResource::Sampler(
                            &normal_sampler
                        )
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 7, 
                        resource: wgpu::BindingResource::TextureView(
                            &emissive_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 8, 
                        resource: wgpu::BindingResource::Sampler(
                            &emissive_sampler
                        )
                    }, 
                ],
            }
        );

        let material: MaterialComponent = Self { name, buffer, bind_group }.into();
        material.update(queue, MaterialDataLayout { diffuse, specular, emissive });
        return material;
    }

    /// 유니폼 버퍼를 갱신합니다.
    pub fn update(&self, queue: &wgpu::Queue, data: MaterialDataLayout) {
        let name =  self.name.clone();
        let capturable = self.buffer.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            if result.is_ok() {
                let mut view = capturable.slice(..).get_mapped_range_mut();
                let layout: &mut MaterialDataLayout = bytemuck::from_bytes_mut(&mut view);
                *layout = data;
                drop(view);
                capturable.unmap();
            }
            else {
                log::warn!("Failed to write uniform buffer! (name: {})", name);
            }
        });
        queue.submit([]);
    }
}

impl Material {
    /// 컴포넌트의 이름을 반환합니다.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 컴포넌트의 유니폼 버퍼를 반환합니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &MaterialBuffer {
        &self.buffer
    }

    /// 컴포넌트의 [wgpu::BindGroup]을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}



/// 3차원 메쉬 재질을 생성하는 빌더입니다.
#[derive(Debug)]
pub struct MaterialBuilder<'a> {
    /// 재질의 이름입니다.
    name: Option<&'a str>, 

    /// 재질에서 사용할 기본 제어 색상입니다.
    pub diffuse: gmm::Float4, 

    /// 재질에서 사용할 반사 제어 색상입니다.
    pub specular: gmm::Float4, 

    /// 재질에서 사용할 발광 제어 색상입니다.
    pub emissive: gmm::Float4, 

    /// 제질에서 사용할 기본 색상 텍스처입니다.
    pub diffuse_map: Arc<wgpu::TextureView>, 

    /// 기본 색상 텍스처의 텍스처 샘플러입니다.
    pub diffuse_sampler: Arc<wgpu::Sampler>, 

    /// 재질에서 사용할 반사 색상 텍스처입니다.
    pub specular_map: Arc<wgpu::TextureView>, 

    /// 반사 색상 텍스처의 텍스처 샘플러입니다.
    pub specular_sampler: Arc<wgpu::Sampler>, 

    /// 재질에서 사용할 법선 데이터 텍스처입니다.
    pub normal_map: Arc<wgpu::TextureView>, 

    /// 법선 데이터 텍스처의 텍스처 샘플러입니다.
    pub normal_sampler: Arc<wgpu::Sampler>, 

    /// 재질에서 사용할 발광 색상 텍스처입니다.
    pub emissive_map: Arc<wgpu::TextureView>, 

    /// 발광 색상 텍스처의 텍스처 샘플러입니다.
    pub emissive_sampler: Arc<wgpu::Sampler>, 
}

impl<'a> MaterialBuilder<'a> {
    /// 새로운 재질 빌더를 생성합니다.
    pub fn new(
        name: Option<&'a str>, 
        device: &'a wgpu::Device, 
        queue: &'a wgpu::Queue
    ) -> Self {
        let sampler = SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default());
        let texture_view = TexturePool::white(device, queue)
            .get_view_or_init(&wgpu::TextureViewDescriptor::default());
        Self { 
            name, 
            diffuse: gmm::Float4::ONE, 
            specular: gmm::Float4::ONE, 
            emissive: gmm::Float4::ONE, 
            diffuse_map: texture_view.clone(), 
            diffuse_sampler: sampler.clone(), 
            specular_map: texture_view.clone(), 
            specular_sampler: sampler.clone(), 
            normal_map: texture_view.clone(), 
            normal_sampler: sampler.clone(), 
            emissive_map: texture_view.clone(), 
            emissive_sampler: sampler.clone() 
        }
    }

    /// 3차원 매쉬의 재질을 생성합니다.
    #[inline]
    pub fn build(self, device: &wgpu::Device, queue: &wgpu::Queue) -> MaterialComponent {
        Material::new(
            self.name, 
            device, 
            queue, 
            self.diffuse, 
            self.specular, 
            self.emissive, 
            self.diffuse_map, 
            self.diffuse_sampler, 
            self.specular_map, 
            self.specular_sampler, 
            self.normal_map, 
            self.normal_sampler, 
            self.emissive_map, 
            self.emissive_sampler
        )
    }
}
