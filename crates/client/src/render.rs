use crate::error::AppError;

use std::sync::Arc;
use std::sync::OnceLock;
use winit::window::Window;

static SWAPCHAIN_FORMAT: OnceLock<wgpu::TextureFormat> = OnceLock::new();



/// `wgpu` 렌더러 객체들을 생성합니다.
#[inline]
pub async fn create_renderer(enable_debug_layer: bool) -> Result<(Arc<wgpu::Instance>, Arc<wgpu::Adapter>, Arc<wgpu::Device>, Arc<wgpu::Queue>), AppError> {
    let instance = create_wgpu_instance(enable_debug_layer);
    let adapter = create_wgpu_adapter(&instance).await?;
    let (device, queue) = create_wgpu_device_and_queue(&adapter).await?;
    Ok((instance, adapter, device, queue))
}

/// `wgpu` 렌더링 인스턴스를 생성합니다.
fn create_wgpu_instance(enable_debug_layer: bool) -> Arc<wgpu::Instance> {
    let mut instance_desc = wgpu::InstanceDescriptor::default();
    if enable_debug_layer {
        instance_desc.flags = wgpu::InstanceFlags::debugging();
    }

    #[cfg(target_os = "windows")] {
        instance_desc.backends = wgpu::Backends::DX12;
        instance_desc.dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env()
            .unwrap_or_default();
    }
    #[cfg(target_os = "macos")] {
        instance_desc.backends = wgpu::Backends::METAL;
    }

    Arc::new(wgpu::Instance::new(instance_desc))
}

/// `wgpu` 장치 어뎁터를 생성합니다.
async fn create_wgpu_adapter(instance: &wgpu::Instance) -> Result<Arc<wgpu::Adapter>, AppError> {
    instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }
    ).await
    .map_or(Err(AppError::NoSuitableAdapter), |adapter| Ok(Arc::new(adapter)))
}

/// `wgpu` 논리적 장치와 명령 대기열을 생성합니다.
async fn create_wgpu_device_and_queue(adapter: &wgpu::Adapter) -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>), AppError> {
    adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::TEXTURE_COMPRESSION_BC, 
            required_limits: wgpu::Limits::downlevel_defaults()
                .using_resolution(adapter.limits()),
        }, 
        None
    ).await
    .map(|(device, queue)| (Arc::new(device), Arc::new(queue)))
    .map_err(|e| AppError::from(e))
}



/// `wgpu` 장치 표면을 생성합니다.
#[allow(unused_must_use)]
pub fn create_wgpu_surface(window: Arc<Window>, instance: &wgpu::Instance, adapter: &wgpu::Adapter) -> Result<Arc<wgpu::Surface<'static>>, AppError> {
    let surface = instance.create_surface(
        wgpu::SurfaceTarget::from(window.clone())
    )
    .map(|surface| Arc::new(surface))
    .map_err(|e| AppError::from(e))?;

    if !adapter.is_surface_supported(&surface) {
        return Err(AppError::NoSuitableAdapter);
    }

    let format = surface.get_capabilities(&adapter)
        .formats
        .first()
        .cloned()
        .unwrap();

    log::info!("스왑체인 텍스처 포맷: {:?}", format);
    SWAPCHAIN_FORMAT.set(format);

    Ok(surface)
}

/// 현재 스왑체인 텍스처 포맷을 가져옵니다.
#[inline]
pub fn get_swapchain_format() -> wgpu::TextureFormat {
    SWAPCHAIN_FORMAT.get()
        .cloned()
        .unwrap_or(wgpu::TextureFormat::Bgra8Unorm)
}

/// 스왑체인을 설정합니다.
#[inline]
pub fn config_swapchain(width: u32, height: u32, device: &wgpu::Device, surface: &wgpu::Surface<'_>) {
    surface.configure(
        device, 
        &wgpu::SurfaceConfiguration {
            width, 
            height, 
            format: get_swapchain_format(),
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            present_mode: wgpu::PresentMode::AutoVsync, 
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        }
    )
}
