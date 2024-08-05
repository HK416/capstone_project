use std::sync::Arc;

use crate::err_msg;
use crate::error::DebugInfo;
use crate::error::ErrorMessage;
use crate::render::error::RenderError;



/// `wgpu` 렌더러를 초기화 합니다.
/// 
/// # Errors
/// `wgpu` 렌더링 컨텍스트를 초기화 하는 도중 오류가 발생한 경우 `ErrorMessage`를 반환합니다.
/// 
#[must_use]
pub async fn init_wgpu_renderer(enable_debug_layer: bool) -> Result<(
    Arc<wgpu::Instance>, 
    Arc<wgpu::Adapter>, 
    Arc<wgpu::Device>, 
    Arc<wgpu::Queue>
), ErrorMessage> {
    let instance = create_wgpu_instance(enable_debug_layer);
    let adapter = create_wgpu_adapter(&instance).await?;
    let (device, queue) = create_wgpu_device_and_queue(&adapter).await?;
    Ok((instance, adapter, device, queue))
}



/// `wgpu` 렌더링 인스턴스를 생성합니다.
fn create_wgpu_instance(enable_debug_layer: bool) -> Arc<wgpu::Instance> {
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



/// `wgpu` 장치 어뎁터를 생성합니다.
/// 
/// # Errors
/// 적절한 `wgpu` 장치 어뎁터를 찾지 못한 경우 `ErrorMessage`를 반환합니다.
/// 
async fn create_wgpu_adapter(instance: &wgpu::Instance) -> Result<Arc<wgpu::Adapter>, ErrorMessage> {
    instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, 
            force_fallback_adapter: false, 
            compatible_surface: None, 
        }
    ).await
    .map(|adapter| adapter.into())
    .ok_or(err_msg!(RenderError::NoSuitableAdapter))
}



/// `wgpu` 논리적 장치와 장치의 명령 대기열을 생성합니다.
/// 
/// # Errors
/// 적절한 `wgpu` 논리적 장치를 찾지 못할 경우 `ErrorMessage`를 반환합니다.
/// 
async fn create_wgpu_device_and_queue(adapter: &wgpu::Adapter) -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>), ErrorMessage> {
    adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None, 
            memory_hints: wgpu::MemoryHints::Performance, 
            required_features: wgpu::Features::default(), 
            required_limits: wgpu::Limits::downlevel_defaults()
                .using_resolution(adapter.limits())
        }, 
        None
    ).await
    .map(|(device, queue)| (device.into(), queue.into()))
    .map_err(|e| err_msg!(RenderError::from(e)))
}
