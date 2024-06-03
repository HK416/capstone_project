//! 렌더링과 관련된 코드를 작성합니다.
//! 

use super::error::AppError;

use std::sync::Arc;
use winit::window::Window;



/// 렌더러의 실행 가능 플랫폼 목록 입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    Unknown,
    Windows,
    MacOS,
}

impl Default for Platform {
    #[inline]
    fn default() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else {
            Platform::Unknown
        }
    }
}



/// `wgpu` 렌더링 컨텍스트 입니다.
/// 
/// [`Instance`](wgpu::Instance), [`Adapter`](wgpu::Adapter)를 가지고 있습니다.
/// 
#[derive(Debug)]
pub struct DrawContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
}

impl DrawContext {
    /// `wgpu`의 렌더링 컨텍스트를 생성합니다.
    /// 
    /// ※ `Windows`의 경우 `DX12`, `macOS`의 경우 `Metal` API를 백엔드로 사용하도록 설정됩니다.
    /// 
    pub async fn new(enable_debug_layer: bool) -> Result<Arc<Self>, AppError> {
        // `wgpu` 인스턴스를 생성합니다.
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor {
                backends: match Platform::default() {
                    Platform::Windows => wgpu::Backends::DX12,
                    Platform::MacOS => wgpu::Backends::METAL,
                    Platform::Unknown => wgpu::Backends::PRIMARY
                },
                flags: match enable_debug_layer { 
                    true => wgpu::InstanceFlags::debugging(),
                    false => wgpu::InstanceFlags::default()
                },
                dx12_shader_compiler: wgpu::util::dx12_shader_compiler_from_env()
                    .unwrap_or_default(),
                gles_minor_version: wgpu::Gles3MinorVersion::Automatic
            }
        );

        // `wgpu` 장치 어뎁터를 생성합니다.
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance, 
                force_fallback_adapter: false,
                compatible_surface: None,
            }
        ).await
        .ok_or(AppError::NoSuitableAdapter)?;

        Ok(Self { instance, adapter }.into())
    }
}



/// `wgpu` 렌더링 표면 입니다.
/// 
/// [`Window`](winit::window::Window), [`Surface`](wgpu::Surface)를 가지고 있습니다.
/// 
#[derive(Debug)]
pub struct DrawSurface {
    pub window: Arc<Window>, 
    pub surface: wgpu::Surface<'static>, 
    pub swapchain_format: wgpu::TextureFormat,
}

impl DrawSurface {
    /// `wgpu`의 렌더링 표면을 생성합니다.
    pub fn new(
        window: Arc<Window>, 
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter
    ) -> Result<Arc<Self>, AppError> {
        // 주어진 윈도우로부터 `wgpu` 렌더링 표면을 생성합니다.
        let surface = instance.create_surface(
            wgpu::SurfaceTarget::from(window.clone())
        ).map_err(|e| AppError::from(e))?;

        // 생성된 `wgpu` 표면이 장치 어뎁터랑 호환되는지 확인합니다.
        if !adapter.is_surface_supported(&surface) {
            return Err(AppError::NoSuitableAdapter);
        }

        // 렌더링 표면의 텍스처 포맷을 가져옵니다.
        let swapchain_format = surface.get_capabilities(adapter)
            .formats
            .first()
            .cloned()
            .unwrap();
        log::info!("스왑체인 텍스처 포맷: {:?}", swapchain_format);

        Ok(Self { window, surface, swapchain_format }.into())
    }

    pub fn config_swapchain(&self, width: u32, height: u32, device: &wgpu::Device) {
        self.surface.configure(
            device, 
            &wgpu::SurfaceConfiguration {
                width,
                height,
                format: self.swapchain_format,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                present_mode: wgpu::PresentMode::AutoVsync,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                desired_maximum_frame_latency: 2,
                view_formats: vec![],
            }
        )
    }
}



/// `wgpu` 렌더링 장치 입니다.
/// 
/// [`Device`](wgpu::Device), [`Queue`](wgpu::Queue)를 가지고 있습니다.
/// 
#[derive(Debug)]
pub struct DrawDevice {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl DrawDevice {
    pub async fn new(adapter: &wgpu::Adapter) -> Result<Arc<Self>, AppError> {
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Graphics Device"), 
                required_features: wgpu::Features::TEXTURE_COMPRESSION_BC,
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
            }, 
            None
        ).await
        .map_err(|e| AppError::from(e))?;
        
        Ok(Self { device, queue }.into())
    }
}
