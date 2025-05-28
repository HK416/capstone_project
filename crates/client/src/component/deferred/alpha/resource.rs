//! Weighted Blended Order-Independent Transparency에서 사용되는 쉐이더 리소스와
//! 관련된 코드를 관리합니다.
//!

use std::sync::Arc;

/// 반투명 오브젝트의 누적 값(Accumuldate)을 저장하는 렌더 타겟 텍스처입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumRenderTarget(Arc<wgpu::TextureView>);

impl AccumRenderTarget {
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
                    label: Some("RenderTarget(Accumulate)"),
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

/// 반투명 오브젝트의 노출 값(Revalage)을 저장하는 렌더 타겟 텍스처입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealRenderTarget(Arc<wgpu::TextureView>);

impl RevealRenderTarget {
    /// 렌더 타겟 텍스처의 [wgpu::TextureFormat]입니다.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

    /// 렌더 타겟 텍스처의 [wgpu::TextureUsages]입니다.
    pub const USAGES: wgpu::TextureUsages =
        wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::TEXTURE_BINDING);

    /// 새로운 렌더 타겟 텍스처를 생성합니다.
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        Self(Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("RenderTarget(Revalage)"),
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
