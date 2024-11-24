use std::sync::Arc;

use error::{SurfaceInitError, WgpuInitError};
use winit::window::Window;

/// 프레임 버퍼의 텍스처 포맷
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

/// 깊이 버퍼 텍스처 포맷
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// `wgpu` 렌더링 객체들을 생성합니다.  
/// 렌더링 객체 생성에 실패한 경우 `WgpuInitError`를 반환합니다.
pub async fn init_wgpu() -> Result<
    (
        Arc<wgpu::Instance>,
        Arc<wgpu::Adapter>,
        Arc<wgpu::Device>,
        Arc<wgpu::Queue>,
    ),
    WgpuInitError,
> {
    let instance = create_instance();
    let adapter = create_adapter(&instance).await?;
    let (device, queue) = create_device_and_queue(&adapter).await?;

    Ok((instance, adapter, device, queue))
}

/// `wgpu` 렌더링 인스턴스를 생성합니다.
fn create_instance() -> Arc<wgpu::Instance> {
    #[allow(unused_mut)]
    let mut desc = wgpu::InstanceDescriptor::default();

    #[cfg(feature = "enable-debug-layer")]
    {
        desc.flags |= wgpu::InstanceFlags::debugging();
    }

    #[cfg(target_os = "windows")]
    {
        desc.backends = wgpu::Backends::DX12;
        desc.dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();
    }
    #[cfg(target_os = "macos")]
    {
        desc.backends = wgpu::Backends::METAL;
    }

    Arc::new(wgpu::Instance::new(desc))
}

/// 적절한 `wgpu` 장치 어뎁터를 생성합니다.  
/// 장치 어뎁터 생성에 실패한 경우 `WgpuInitError::NoSuitableAdapter`를 반환합니다.
async fn create_adapter(instance: &wgpu::Instance) -> Result<Arc<wgpu::Adapter>, WgpuInitError> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .map(|adapter| Arc::new(adapter))
        .ok_or(WgpuInitError::NoSuitableAdapter)
}

/// 적절한 `wgpu` 논리적 장치와 명령 대기열을 생성합니다.  
/// 논리적 장치 생성에 실패한 경우 `WgpuInitError::NoSuitableDevice`를 반환합니다.
async fn create_device_and_queue(
    adapter: &wgpu::Adapter,
) -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>), WgpuInitError> {
    #[allow(unused_mut)]
    let mut desc = wgpu::DeviceDescriptor::default();

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        desc.memory_hints = wgpu::MemoryHints::Performance;
        desc.required_features = wgpu::Features::default()
            .union(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
            .union(wgpu::Features::TEXTURE_COMPRESSION_BC);
        desc.required_limits =
            wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
    }

    adapter
        .request_device(&desc, None)
        .await
        .map(|(device, queue)| (Arc::new(device), Arc::new(queue)))
        .map_err(|e| WgpuInitError::from(e))
}

/// `wgpu` 창 표면 객체를 생성합니다.  
/// 창 표면 객체 생성에 실패하거나, 창 표면 객체가 장치 어뎁터와 호환되지 않는 경우 `SurfaceInitError`를 반환합니다.
pub fn create_surface(
    window: Arc<Window>,
    instance: &wgpu::Instance,
    adapter: &wgpu::Adapter,
) -> Result<Arc<wgpu::Surface<'static>>, SurfaceInitError> {
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::from(window))
        .map(|surface| Arc::new(surface))
        .map_err(|e| SurfaceInitError::from(e))?;

    if !adapter.is_surface_supported(&surface) {
        return Err(SurfaceInitError::NotCompatible);
    }

    Ok(surface)
}

/// `wgpu` 스왑체인 텍스처를 설정합니다.
pub fn config_swapchain(
    width: u32,
    height: u32,
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    vsync: bool,
) {
    surface.configure(
        device,
        &wgpu::SurfaceConfiguration {
            width,
            height,
            format: SWAPCHAIN_FORMAT,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            present_mode: match vsync {
                true => wgpu::PresentMode::AutoVsync,
                false => wgpu::PresentMode::AutoNoVsync,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            desired_maximum_frame_latency: 0,
            view_formats: vec![],
        },
    );
}

pub mod error;
pub mod mesh;
