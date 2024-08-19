use std::ops;
use std::sync::OnceLock;
use winit::window::Window;



/// 깊이 테스트에 사용되는 깊이 버퍼입니다.
#[derive(Debug)]
pub struct DepthBuffer(wgpu::TextureView);

impl DepthBuffer {
    /// 깊이 버퍼의 텍스처 포맷입니다.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
}

impl DepthBuffer {
    /// 깊이 버퍼를 가져옵니다.
    #[must_use]
    pub fn get(window: &Window, device: &wgpu::Device) -> &'static Self {
        static THIS: OnceLock<DepthBuffer> = OnceLock::new();
        THIS.get_or_init(|| {
            let (width, height): (u32, u32) = window.inner_size().into();
            Self(device.create_texture(
                &wgpu::TextureDescriptor {
                    label: Some("DepthBuffer"), 
                    format: Self::FORMAT, 
                    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    dimension: wgpu::TextureDimension::D2, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST, 
                    view_formats: &[]
                }, 
            ).create_view(
                &wgpu::TextureViewDescriptor { ..Default::default() }, 
            ))
        })
    }
}

impl ops::Deref for DepthBuffer {
    type Target = wgpu::TextureView;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
