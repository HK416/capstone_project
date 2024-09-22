use std::sync::{Arc, OnceLock};

use super::MaterialUniform;



/// 재질 데이터입니다.
#[derive(Debug)]
pub struct Material {
    /// 재질의 이름입니다.
    pub(super) name: String, 

    /// 재질의 유니폼 버퍼입니다.
    pub(super) uniform: MaterialUniform, 

    /// 재질의 바인드 그룹입니다.
    pub(super) bind_group: wgpu::BindGroup, 
}

impl Material {
    /// 범용적으로 사용 가능한 재질의 [wgpu::BindGroupLayout]를 반환합니다.
    #[must_use]
    pub fn bind_group_layout(device: &Arc<wgpu::Device>) -> &'static wgpu::BindGroupLayout {
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
    /// 재질의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 재질의 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn uniform(&self) -> &MaterialUniform {
        &self.uniform
    }

    /// 재질의 바인드 그룹을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
