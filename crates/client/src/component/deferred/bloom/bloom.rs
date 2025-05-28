//! Bloom 쉐이더 기법에서 사용되는 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::Arc;

/// 발광체 오브젝트의 색상을 저장하는 렌더 타겟 텍스처입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightRenderTarget(Arc<wgpu::TextureView>);

impl BrightRenderTarget {
    /// 렌더 타겟 텍스처의 [wgpu::TextureFormat]입니다.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    /// 렌더 타겟 텍스처의 [wgpu::TextureUsages]입니다.
    pub const USAGES: wgpu::TextureUsages =
        wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::TEXTURE_BINDING);

    /// 새로운 렌더 타겟 텍스처를 생성합니다.
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        Self(Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("RenderTarget(Bright)"),
                    dimension: wgpu::TextureDimension::D2,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    format: Self::FORMAT,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: Self::USAGES,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        ))
    }

    /// [wgpu::TextureView]를 반환합니다.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.0
    }
}

/// 가우시안 블러를 수행한 결과를 저장하는 텍스처입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlurTextureResource(Arc<wgpu::TextureView>);

impl BlurTextureResource {
    /// 텍스처의 [wgpu::TextureFormat]입니다.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    /// 텍스처의 [wgpu::TextureUsages]입니다.
    pub const USAGES: wgpu::TextureUsages =
        wgpu::TextureUsages::STORAGE_BINDING.union(wgpu::TextureUsages::TEXTURE_BINDING);

    /// 새로운 텍스처를 생성합니다.
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        Self(Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("RenderTarget(Gaussian_Blur)"),
                    dimension: wgpu::TextureDimension::D2,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    format: Self::FORMAT,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: Self::USAGES,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        ))
    }

    /// [wgpu::TextureView]를 반환합니다.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.0
    }
}
