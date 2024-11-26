use std::{cell::RefCell, sync::Arc};

use mod_render::{config_swapchain, create_surface, SurfaceInitError, DEPTH_FORMAT};
use winit::{
    error::OsError,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

use crate::etc::AppFlags;

/// ## Application Window Initialization Error
#[derive(Debug, thiserror::Error)]
pub enum WindowInitError {
    /// 애플리케이션 창을 생성하지 못한 경우 발생하는 오류입니다.
    #[error("The application window could not be created for the following reason: {0}")]
    WindowCreationFailed(#[from] OsError),

    /// `wgpu` 장치 표면을 생성하지 못한 경우 발생하는 오류입니다.
    #[error("{0}")]
    SurfaceCreationFailed(#[from] SurfaceInitError),
}

pub struct AppWindow {
    pub window: Arc<Window>,
    pub egui_state: RefCell<egui_winit::State>,
    pub surface: Arc<wgpu::Surface<'static>>,
    pub depth_buffer_view: RefCell<Arc<wgpu::TextureView>>,
    pub disable_vsync: bool,
}

impl AppWindow {
    /// 새로운 애플리케이션 창을 생성합니다.
    ///
    /// # Panics
    /// 애플리케이션 창의 가로와 세로의 크기가 0인 경우 [`panic!`]을 호출합니다.
    ///
    #[must_use]
    pub fn create(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
        flags: &AppFlags,
        egui_ctx: &egui::Context,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
    ) -> Result<Self, WindowInitError> {
        // 새로운 애플리케이션 창을 생성합니다.)
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| WindowInitError::from(e))?,
        );

        // `wgpu` 장치 표면을 생성합니다.
        let surface = create_surface(window.clone(), &instance, &adapter)
            .map_err(|e| WindowInitError::from(e))?;

        // 생성된 애플리케이션 창의 크기를 가져옵니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        assert!(
            width != 0 && height != 0,
            "The size of the application window cannot be zero!"
        );

        // `wgpu` 스왑체인을 설정합니다.
        let disable_vsync = flags.contains(AppFlags::DISABLE_VSYNC);
        config_swapchain(width, height, device, &surface, disable_vsync);

        // 깊이 버퍼 뷰를 생성합니다.
        let depth_buffer_view = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Depth-Buffer"),
                    dimension: wgpu::TextureDimension::D2,
                    format: DEPTH_FORMAT,
                    mip_level_count: 1,
                    sample_count: 1,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        )
        .into();

        // `egui` winit 상태를 생성합니다.
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        )
        .into();

        Ok(Self {
            window,
            egui_state,
            surface,
            depth_buffer_view,
            disable_vsync,
        })
    }

    /// 애플리케이션 창의 크기가 변경됐을 때 호출되는 콜백 함수입니다.
    pub fn on_resized(&self, instance: &wgpu::Instance, device: &wgpu::Device) {
        // 애플리케이션 창의 가로와 세로 크기를 가져옵니다.
        // 가로 또는 세로 크기가 0인 경우 함수 실행을 중단합니다.
        let (width, height): (u32, u32) = self.window.inner_size().into();
        if width == 0 || height == 0 {
            return;
        }

        // 이전에 제출한 모든 렌더링 작업이 끝날 때 까지 대기합니다.
        instance.poll_all(true);

        // 변경된 크기로 스왑체인을 재설정합니다.
        config_swapchain(width, height, device, &self.surface, self.disable_vsync);

        // 변경된 크기로 깊이 버퍼를 재설정합니다.
        let mut depth_buffer_view = self.depth_buffer_view.borrow_mut();
        *depth_buffer_view = Arc::new(
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Depth-Buffer"),
                    dimension: wgpu::TextureDimension::D2,
                    format: DEPTH_FORMAT,
                    mip_level_count: 1,
                    sample_count: 1,
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
    }
}
