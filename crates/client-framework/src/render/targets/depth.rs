use std::ops;
use std::sync::Arc;
use winit::window::Window;



/// 깊이 테스트에 사용되는 깊이 버퍼 입니다.
#[derive(Debug)]
pub struct DepthBuffer(wgpu::TextureView);

impl DepthBuffer {
    /// 깊이 버퍼의 텍스처 포맷 입니다.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
}

impl DepthBuffer {
    pub fn new(
        window: &Window, 
        device: &wgpu::Device
    ) -> Arc<Self> {
        // 현재 창의 가로와 세로 크기를 가져옵니다.
        let (width, height): (u32, u32) = window.inner_size().into();

        // 깊이 텍스처를 생성합니다.
        let texture = device.create_texture(
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
        );

        // 텍스처 뷰를 생성합니다.
        let texture_view = texture.create_view(
            &wgpu::TextureViewDescriptor { 
                ..Default::default() 
            }, 
        );

        Self(texture_view).into()
    }
}

impl ops::Deref for DepthBuffer {
    type Target = wgpu::TextureView;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
