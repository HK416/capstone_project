use std::{
    num::NonZeroU64,
    ops::RangeBounds,
    sync::{Arc, OnceLock},
};

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::{SamplerPool, TexturePool, TextureViewPool};

/// 재질의 종류 목록입니다.
#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialKind {
    Opaque = 0,
    Transparent = 1,
}

impl Default for MaterialKind {
    fn default() -> Self {
        Self::Opaque
    }
}

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
    pub fn update(&self, _device: &wgpu::Device, _queue: &wgpu::Queue, data: MaterialDataLayout) {
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

        // let index = queue.submit([]);
        // device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
    }

    /// 재질 데이터 유니폼 버퍼의 내용을 갱신합니다.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub unsafe fn update_from_bytes(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
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

        // let index = queue.submit([]);
        // device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(index));
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

/// 재질의 Albedo 색상 데이터입니다.
#[derive(Debug, Clone)]
pub enum Albedo {
    None,
    Color([f32; 4]),
    Texture {
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    },
}

impl Albedo {
    /// [wgpu::TextureView]를 반환합니다.
    fn view(
        &self,
        name: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Arc<wgpu::TextureView> {
        match self {
            Albedo::None => {
                let texture = TexturePool::white(device, queue);
                let view =
                    TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
                view
            }
            Albedo::Color(color) => {
                let color: Vec<u8> = color
                    .iter()
                    .cloned()
                    .map(|i| (i * 255.0).ceil() as u8)
                    .collect();
                let texture = device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some(&format!("Texture({})", name)),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        mip_level_count: 1,
                        sample_count: 1,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &bytemuck::cast_slice(&color),
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                Arc::new(view)
            }
            Albedo::Texture { view, .. } => view.clone(),
        }
    }

    /// [wgpu::Sampler]를 반환합니다.
    fn sampler(&self, device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        match self {
            Albedo::Texture { sampler, .. } => sampler.clone(),
            _ => SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default()),
        }
    }
}

impl Default for Albedo {
    fn default() -> Self {
        Self::None
    }
}

/// 재질의 Specular 색상 데이터입니다.
#[derive(Debug, Clone)]
pub enum Specular {
    None,
    Color([f32; 4]),
    Texture {
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    },
}

impl Specular {
    /// [wgpu::TextureView]를 반환합니다.
    fn view(
        &self,
        name: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Arc<wgpu::TextureView> {
        match self {
            Specular::None => {
                let texture = TexturePool::white(device, queue);
                let view =
                    TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
                view
            }
            Specular::Color(color) => {
                let texture = device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some(&format!("Texture({})", name)),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba32Float,
                        mip_level_count: 1,
                        sample_count: 1,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &bytemuck::cast_slice(color),
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                Arc::new(view)
            }
            Specular::Texture { view, .. } => view.clone(),
        }
    }

    /// [wgpu::Sampler]를 반환합니다.
    fn sampler(&self, device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        match self {
            Specular::Texture { sampler, .. } => sampler.clone(),
            _ => SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default()),
        }
    }
}

impl Default for Specular {
    fn default() -> Self {
        Self::None
    }
}

/// 재질의 Emissive 색상 데이터입니다.
#[derive(Debug, Clone)]
pub enum Emissive {
    None,
    Color([f32; 4]),
    Texture {
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    },
}

impl Emissive {
    /// [wgpu::TextureView]를 반환합니다.
    fn view(
        &self,
        name: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Arc<wgpu::TextureView> {
        match self {
            Emissive::None => {
                let texture = TexturePool::white(device, queue);
                let view =
                    TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
                view
            }
            Emissive::Color(color) => {
                let texture = device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some(&format!("Texture({})", name)),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba32Float,
                        mip_level_count: 1,
                        sample_count: 1,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &bytemuck::cast_slice(color),
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                Arc::new(view)
            }
            Emissive::Texture { view, .. } => view.clone(),
        }
    }

    /// [wgpu::Sampler]를 반환합니다.
    fn sampler(&self, device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        match self {
            Emissive::Texture { sampler, .. } => sampler.clone(),
            _ => SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default()),
        }
    }
}

impl Default for Emissive {
    fn default() -> Self {
        Self::None
    }
}

/// 재질의 Normal 데이터입니다.
#[derive(Debug, Clone)]
pub enum Normal {
    None,
    Texture {
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    },
}

impl Normal {
    /// [wgpu::TextureView]를 반환합니다.
    fn view(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::TextureView> {
        match self {
            Normal::None => {
                let texture = TexturePool::normal(device, queue);
                let view =
                    TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
                view
            }
            Normal::Texture { view, .. } => view.clone(),
        }
    }

    /// [wgpu::Sampler]를 반환합니다.
    fn sampler(&self, device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        match self {
            Normal::Texture { sampler, .. } => sampler.clone(),
            _ => SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default()),
        }
    }
}

impl Default for Normal {
    fn default() -> Self {
        Self::None
    }
}

/// 재질의 Height 데이터입니다.
#[derive(Debug, Clone)]
pub enum Height {
    None,
    Texture {
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    },
}

impl Height {
    /// [wgpu::TextureView]를 반환합니다.
    fn view(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::TextureView> {
        match self {
            Height::None => {
                let texture = TexturePool::height(device, queue);
                let view =
                    TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
                view
            }
            Height::Texture { view, .. } => view.clone(),
        }
    }

    /// [wgpu::Sampler]를 반환합니다.
    fn sampler(&self, device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        match self {
            Height::Texture { sampler, .. } => sampler.clone(),
            _ => SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default()),
        }
    }
}

impl Default for Height {
    fn default() -> Self {
        Self::None
    }
}

/// 재질의 Occlusion 데이터입니다.
#[derive(Debug, Clone)]
pub enum Occlusion {
    None,
    Texture {
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    },
}

impl Occlusion {
    /// [wgpu::TextureView]를 반환합니다.
    fn view(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::TextureView> {
        match self {
            Occlusion::None => {
                let texture = TexturePool::white(device, queue);
                let view =
                    TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
                view
            }
            Occlusion::Texture { view, .. } => view.clone(),
        }
    }

    /// [wgpu::Sampler]를 반환합니다.
    fn sampler(&self, device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        match self {
            Occlusion::Texture { sampler, .. } => sampler.clone(),
            _ => SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default()),
        }
    }
}

impl Default for Occlusion {
    fn default() -> Self {
        Self::None
    }
}

/// ## Material Shader Resource Descriptor
#[derive(Debug, Clone)]
pub struct MaterialDescriptor {
    pub name: String,
    pub kind: MaterialKind,
    pub layout: MaterialDataLayout,
    pub albedo: Albedo,
    pub specular: Specular,
    pub emissive: Emissive,
    pub normal: Normal,
    pub height: Height,
    pub occlusion: Occlusion,
}

impl MaterialDescriptor {
    pub fn new(name: &str, kind: MaterialKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            layout: MaterialDataLayout::default(),
            albedo: Albedo::default(),
            specular: Specular::default(),
            emissive: Emissive::default(),
            normal: Normal::default(),
            height: Height::default(),
            occlusion: Occlusion::default(),
        }
    }

    /// Albedo 색상을 설정합니다.
    pub fn with_albedo_color(&mut self, color: [f32; 4]) -> &mut Self {
        self.albedo = Albedo::Color(color);
        self
    }

    /// Albedo 텍스처를 설정합니다.
    pub fn with_albedo_texture(
        &mut self,
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    ) -> &mut Self {
        self.albedo = Albedo::Texture { view, sampler };
        self
    }

    /// Specular 색상을 설정합니다.
    pub fn with_specular_color(&mut self, color: [f32; 4]) -> &mut Self {
        self.specular = Specular::Color(color);
        self
    }

    /// Specular 텍스처를 설정합니다.
    pub fn with_specular_texture(
        &mut self,
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    ) -> &mut Self {
        self.specular = Specular::Texture { view, sampler };
        self
    }

    /// Emissive 색상을 설정합니다.
    pub fn with_emissive_color(&mut self, color: [f32; 4]) -> &mut Self {
        self.emissive = Emissive::Color(color);
        self
    }

    /// Emissive 텍스처를 설정합니다.
    pub fn with_emissive_texture(
        &mut self,
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    ) -> &mut Self {
        self.emissive = Emissive::Texture { view, sampler };
        self
    }

    /// Normal 텍스처를 설정합니다.
    pub fn with_normal_texture(
        &mut self,
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    ) -> &mut Self {
        self.normal = Normal::Texture { view, sampler };
        self
    }

    /// Height 텍스처를 설정합니다.
    pub fn with_height_texture(
        &mut self,
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    ) -> &mut Self {
        self.height = Height::Texture { view, sampler };
        self
    }

    /// Occlusion 텍스처를 설정합니다.
    pub fn with_occlusion_texture(
        &mut self,
        view: Arc<wgpu::TextureView>,
        sampler: Arc<wgpu::Sampler>,
    ) -> &mut Self {
        self.occlusion = Occlusion::Texture { view, sampler };
        self
    }
}

/// ## Material Shader Resource
#[derive(Debug)]
pub struct MaterialResource {
    pub name: String,
    pub kind: MaterialKind,
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 4번 바인딩: Specular 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // 5번 바인딩: Emissive 텍스처
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // 6번 바인딩: Emissive 텍스처 샘플러
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
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
                    resource: wgpu::BindingResource::TextureView(
                        &desc.albedo.view(&desc.name, device, queue),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&desc.albedo.sampler(device)),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &desc.specular.view(&desc.name, device, queue),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&desc.specular.sampler(device)),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(
                        &desc.emissive.view(&desc.name, device, queue),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&desc.emissive.sampler(device)),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&desc.normal.view(device, queue)),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&desc.normal.sampler(device)),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&desc.height.view(device, queue)),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&desc.height.sampler(device)),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: wgpu::BindingResource::TextureView(
                        &desc.occlusion.view(device, queue),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: wgpu::BindingResource::Sampler(&desc.occlusion.sampler(device)),
                },
            ],
        });

        Self {
            name: desc.name.to_string(),
            kind: desc.kind,
            material_uniform,
            bind_group,
        }
    }
}
