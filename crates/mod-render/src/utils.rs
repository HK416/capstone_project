use std::sync::Arc;

use winit::window::Window;

use crate::RenderError;

/// 창 표면에 사용되는 스왑체인 텍스처 포맷입니다.
pub const SWAPCHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// 깊이-스탠실 버퍼에 사용되는 텍스처 포맷입니다.
pub const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;



/// `wgpu` 렌더링 객체를 생성합니다.
/// 
/// # Errors
/// `wgpu` 렌더링 객체를 생성하는 도중 오류가 발생할 경우 `RenderError`를 반환합니다.
/// 
#[must_use]
pub async fn init_wgpu(enable_debug_layer: bool) -> Result<(
    Arc<wgpu::Instance>, 
    Arc<wgpu::Adapter>, 
    Arc<wgpu::Device>, 
    Arc<wgpu::Queue>
), RenderError> {
    let instance = create_instance(enable_debug_layer);
    let adapter = create_adapter(&instance).await?;
    let (device, queue) = create_device_and_queue(&adapter).await?;
    Ok((instance, adapter, device, queue))
}



/// `wgpu` 렌더링 인스턴스를 생성합니다.
#[must_use]
fn create_instance(enable_debug_layer: bool) -> Arc<wgpu::Instance> {
    let mut instance_desc = wgpu::InstanceDescriptor::default();
    if enable_debug_layer {
        instance_desc.flags |= wgpu::InstanceFlags::debugging();
    }

    #[cfg(target_os = "windows")] {
        instance_desc.backends = wgpu::Backends::DX12;
        instance_desc.dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env()
            .unwrap_or_default();
    }
    #[cfg(target_os = "macos")] {
        instance_desc.backends = wgpu::Backends::METAL;
    }

    wgpu::Instance::new(instance_desc).into()
}



/// `wgpu` 장치 어댑터를 생성합니다.
/// 
/// # Errors 
/// 적절한 `wgpu` 장치 어댑터를 찾지 못한 경우 `RenderError`를 반환합니다.
/// 
#[must_use]
async fn create_adapter(instance: &wgpu::Instance)-> Result<Arc<wgpu::Adapter>, RenderError> {
    instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, 
            force_fallback_adapter: false, 
            compatible_surface: None, 
        }
    ).await
    .map(|adapter| adapter.into())
    .ok_or(RenderError::NoSuitableAdapter)
}



/// `wgpu` 논리적 장치와 장치의 명령 대기열을 생성합니다.
/// 
/// # Errors
/// 적절한 `wgpu` 논리적 장치를 찾지 못한 경우 `RenderError`를 반환합니다.
/// 
#[must_use]
async fn create_device_and_queue(adapter: &wgpu::Adapter) -> Result<(
    Arc<wgpu::Device>, 
    Arc<wgpu::Queue>
), RenderError> {
    adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None, 
            memory_hints: wgpu::MemoryHints::Performance, 
            required_features: wgpu::Features::default()
                | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
                | wgpu::Features::TEXTURE_COMPRESSION_BC, 
            required_limits: wgpu::Limits::downlevel_defaults()
                .using_resolution(adapter.limits())
        }, 
        None
    ).await
    .map(|(device, queue)| (device.into(), queue.into()))
    .map_err(|e| RenderError::from(e))
}



/// `wgpu` 장치 표면을 생성합니다.
/// 
/// # Errors
/// 생성된 창 표면이 장치 어댑터와 호환되지 않는 경우 `RenderError`를 반환합니다.
/// 
#[must_use]
pub fn create_surface(
    window: Arc<Window>, 
    instance: &wgpu::Instance, 
    adapter: &wgpu::Adapter
) -> Result<Arc<wgpu::Surface<'static>>, RenderError> {
    // `wgpu` 창 표면을 생성합니다.
    let result = instance.create_surface(wgpu::SurfaceTarget::from(window));
    let surface = match result {
        Ok(surface) => surface, 
        Err(e) => return Err(RenderError::from(e)), 
    };
    
    // 생성된 창 표면이 장치 어댑터와 호환되는지 확인합니다.
    if !adapter.is_surface_supported(&surface) {
        return Err(RenderError::NoSuitableAdapter);
    }

    Ok(surface.into())
}



/// 스왑체인을 설정합니다.
pub fn config_swapchain(
    width: u32, 
    height: u32, 
    device: &wgpu::Device, 
    surface: &wgpu::Surface<'_>, 
    disable_vsync: bool, 
) {
    surface.configure(
        device, 
        &wgpu::SurfaceConfiguration {
            width, 
            height, 
            format: SWAPCHAIN_FORMAT, 
            alpha_mode: wgpu::CompositeAlphaMode::Auto, 
            present_mode: match disable_vsync {
                true => wgpu::PresentMode::AutoNoVsync, 
                false => wgpu::PresentMode::AutoVsync, 
            }, 
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
            desired_maximum_frame_latency: 0, 
            view_formats: vec![], 
        }
    );
}
