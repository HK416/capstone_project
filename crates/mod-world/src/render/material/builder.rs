use std::sync::Arc;

use crate::render::pool::{SamplerPool, TexturePool, TextureViewPool};

use super::{Material, MaterialDataLayout, MaterialUniform};



/// 재질을 생성하는 빌더입니다.
#[derive(Debug, Clone)]
pub struct MaterialBuilder {
    /// 재질의 이름입니다.
    pub(crate) name: String, 
    
    /// 재질의 매끄러운 정도입니다.
    pub glossiness: f32, 

    /// 재질의 부드러운 정도입니다.
    pub smoothness: f32, 

    /// 재질의 금속성 정도입니다.
    pub metallic: f32, 

    /// 재질의 `Diffuse` 색상입니다.
    pub diffuse: gmm::Float4, 

    /// 재질의 `Specular` 색상입니다.
    pub specular: gmm::Float4, 

    /// 재질의 `Emissive` 색상입니다.
    pub emissive: gmm::Float4, 

    /// 재질의 `Diffuse` 텍스처 뷰입니다.
    pub diffuse_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Diffuse` 텍스처 샘플러입니다.
    pub diffuse_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Specular` 텍스처 뷰입니다.
    pub specular_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Specular` 텍스처 샘플러입니다.
    pub specular_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Normal` 텍스처 뷰입니다.
    pub normal_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Normal` 텍스처 샘플러입니다.
    pub normal_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Emissive` 텍스처 뷰입니다.
    pub emissive_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Emissive` 텍스처 샘플러입니다.
    pub emissive_sampler: Arc<wgpu::Sampler>, 

    /// 재질의 `Height` 텍스처 뷰입니다.
    pub height_map: Arc<wgpu::TextureView>, 

    /// 재질의 `Height` 텍스처 샘플러입니다.
    pub height_sampler: Arc<wgpu::Sampler>, 
}

impl MaterialBuilder {
    /// 새로운 재질 빌더를 생성합니다.
    #[must_use]
    pub fn new<N: Into<String>>(
        name: N, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) -> Self {
        let default_white = TexturePool::white(device, queue);
        let default_white = TextureViewPool::get_or_init(
            &default_white, 
            &wgpu::TextureViewDescriptor::default()
        );
        let default_normal = TexturePool::normal(device, queue);
        let default_normal = TextureViewPool::get_or_init(
            &default_normal, 
            &wgpu::TextureViewDescriptor::default()
        );
        let default_height = TexturePool::height(device, queue);
        let default_height = TextureViewPool::get_or_init(
            &default_height, 
            &wgpu::TextureViewDescriptor::default()
        );
        let default_linear = SamplerPool::linear(device);

        Self { 
            name: name.into(), 
            glossiness: 0.75, 
            smoothness: 0.75, 
            metallic: 0.25, 
            diffuse: gmm::Float4::new(0.9, 0.9, 0.9, 1.0), 
            specular: gmm::Float4::fill(1.0), 
            emissive: gmm::Float4::fill(1.0), 
            diffuse_map: default_white.clone(), 
            diffuse_sampler: default_linear.clone(), 
            specular_map: default_white.clone(), 
            specular_sampler: default_linear.clone(), 
            normal_map: default_normal.clone(), 
            normal_sampler: default_linear.clone(), 
            emissive_map: default_white.clone(), 
            emissive_sampler: default_linear.clone(), 
            height_map: default_height.clone(), 
            height_sampler: default_linear.clone()
        }
    }

    /// 재질 빌더로부터 재질을 생성합니다.
    #[must_use]
    pub fn build(self, device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<Material> {
        let uniform = MaterialUniform::new(Some(&format!("MaterialUniform({})", &self.name)), device);
        uniform.update(device, queue, MaterialDataLayout {
            glossiness: self.glossiness, 
            smoothness: self.smoothness, 
            metallic: self.metallic, 
            diffuse: self.diffuse, 
            specular: self.specular, 
            emissive: self.emissive, 
            ..Default::default()
        });

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", &self.name)), 
                layout: &Material::bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: uniform.as_entire_binding(),
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::TextureView(
                            &self.diffuse_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: wgpu::BindingResource::Sampler(
                            &self.diffuse_sampler
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 3, 
                        resource: wgpu::BindingResource::TextureView(
                            &self.specular_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 4, 
                        resource: wgpu::BindingResource::Sampler(
                            &self.specular_sampler
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 5, 
                        resource: wgpu::BindingResource::TextureView(
                            &self.normal_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 6, 
                        resource: wgpu::BindingResource::Sampler(
                            &self.normal_sampler
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 7, 
                        resource: wgpu::BindingResource::TextureView(
                            &self.emissive_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 8, 
                        resource: wgpu::BindingResource::Sampler(
                            &self.emissive_sampler
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 9, 
                        resource: wgpu::BindingResource::TextureView(
                            &self.height_map
                        ), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 10, 
                        resource: wgpu::BindingResource::Sampler(
                            &self.height_sampler
                        ), 
                    },
                ]
            }
        );

        Material { 
            name: self.name, 
            uniform, 
            bind_group 
        }.into()
    }
}
