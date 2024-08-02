use std::sync::Arc;
use winit::window::Window;

use crate::render::RenderError;

/// 창 표면에 사용되는 스왑체인 텍스처 포맷 입니다.
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;




/// `wgpu` 창 표면을 생성합니다.
#[must_use]
pub fn create_wgpu_surface(
    window: Arc<Window>, 
    instance: &wgpu::Instance, 
    adapter: &wgpu::Adapter
) -> Result<Arc<wgpu::Surface<'static>>, RenderError> {
    // `wgpu` 창 표면을 생성합니다.
    let surface = instance.create_surface(wgpu::SurfaceTarget::from(window))
        .map_err(|e| RenderError::from(e))?;

    // 생성된 창 표면이 장치 어뎁터와 호환되는지 확인합니다.
    if !adapter.is_surface_supported(&surface) {
        return Err(RenderError::NoSuitableAdapter);
    }

    Ok(surface.into())
}



/// 스왑체인 텍스처의 크기를 설정 합니다.
pub fn config_swapchain(
    width: u32, 
    height: u32, 
    device: &wgpu::Device, 
    surface: &wgpu::Surface<'_>
) {
    surface.configure(
        device, 
        &wgpu::SurfaceConfiguration {
            width, 
            height, 
            format: SWAPCHAIN_FORMAT, 
            alpha_mode: wgpu::CompositeAlphaMode::Auto, 
            present_mode: wgpu::PresentMode::AutoVsync, 
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
            desired_maximum_frame_latency: 2, 
            view_formats: vec![], 
        }
    );
}
