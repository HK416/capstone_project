use std::{collections::HashMap, sync::Arc};

use lazy_static::lazy_static;
use mod_world::render::{
    config_swapchain, 
    create_surface, 
    RenderError, 
    DEPTH_STENCIL_FORMAT
};
use winit::{
    error::OsError, 
    event::{ElementState, KeyEvent, Modifiers, MouseButton}, 
    event_loop::ActiveEventLoop, 
    keyboard::{KeyCode, PhysicalKey}, 
    window::{Window, WindowAttributes}
};

use crate::etc::AppFlags;



#[derive(Debug, thiserror::Error)]
pub enum AppWindowError {
    /// 애플리케이션 창을 생성하지 못한 경우 발생하는 오류입니다.
    #[error("The application window could not be created for the following reason: {0}")]
    WindowCreationFailed(#[from] OsError), 

    /// `wgpu` 장치 표면을 생성하지 못한 경우 발생하는 오류입니다.
    #[error("{0}")]
    SurfaceCreationFailed(#[from] RenderError)
}


#[derive(Debug)]
pub struct AppWindow {
    window: Arc<Window>, 
    egui_raw_input: egui::RawInput, 
    surface: Arc<wgpu::Surface<'static>>, 
    depth_buffer_view: Arc<wgpu::TextureView>, 
    disable_vsync: bool 
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
        instance: &wgpu::Instance, 
        adapter: &wgpu::Adapter, 
        device: &wgpu::Device
    ) -> Result<Self, AppWindowError> {
        // 새로운 애플리케이션 창을 생성합니다.
        let window = Arc::new(event_loop.create_window(attributes)
            .map_err(|e| AppWindowError::from(e))?);

        // egui 입력기를 생성합니다.
        let mut egui_raw_input = egui::RawInput {
            focused: false, 
            ..Default::default()
        };

        // egui 뷰포트의 스케일을 설정합니다.
        let scale_factor = window.scale_factor() as f32;
        egui_raw_input.viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale_factor);

        // `wgpu` 장치 표면을 생성합니다.
        let surface = create_surface(window.clone(), &instance, &adapter)
            .map_err(|e| AppWindowError::from(e))?;

        // 생성된 애플리케이션 창의 크기를 가져옵니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        assert!(width != 0 && height != 0, "The size of the application window cannot be zero!");
        
        // `wgpu` 스왑체인을 설정합니다.
        let disable_vsync = flags.contains(AppFlags::DISABLE_VSYNC);
        config_swapchain(width, height, device, &surface, disable_vsync);

        // 깊이 버퍼 뷰를 생성합니다.
        let depth_buffer_view = Arc::new(device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Depth-Buffer"), 
                dimension: wgpu::TextureDimension::D2, 
                format: DEPTH_STENCIL_FORMAT, 
                mip_level_count: 1, 
                sample_count: 1, 
                size: wgpu::Extent3d {
                    width, 
                    height, 
                    depth_or_array_layers: 1
                }, 
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
                view_formats: &[]
            }
        ).create_view(&wgpu::TextureViewDescriptor::default()));

        Ok(Self {
            window, 
            egui_raw_input, 
            surface, 
            depth_buffer_view, 
            disable_vsync
        })
    }

    /// 애플리케이션 창이 주목받을 때 호출되는 콜백 함수입니다.
    pub fn on_focused(&mut self, focused: bool) {
        self.egui_raw_input.focused = focused;
        self.egui_raw_input.events.push(egui::Event::WindowFocused(focused));
    }

    /// 애플리케이션 창의 크기가 변경됐을 때 호출되는 콜백 함수입니다.
    pub fn on_resized(&mut self, instance: &wgpu::Instance, device: &wgpu::Device) {
        // 애플리케이션 창의 가로와 세로 크기를 가져옵니다.
        // 가로 또는 세로 크기가 0인 경우 함수 실행을 중단합니다.
        let (width, height): (u32, u32) = self.window.inner_size().into();
        if width == 0 || height == 0 {
            return;
        }

        // 이전에 제출한 모든 렌더링 작업이 끝날 때 까지 대기합니다.
        instance.poll_all(true);

        // egui 뷰포트의 스케일을 재설정합니다.
        let scale_factor = self.window.scale_factor() as f32;
        self.egui_raw_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale_factor);

        // 변경된 크기로 스왑체인을 재설정합니다.
        config_swapchain(width, height, device, &self.surface, self.disable_vsync);

        // 변경된 크기로 깊이 버퍼를 재설정합니다.
        self.depth_buffer_view = Arc::new(device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Depth-Buffer"), 
                dimension: wgpu::TextureDimension::D2, 
                format: DEPTH_STENCIL_FORMAT, 
                mip_level_count: 1, 
                sample_count: 1, 
                size: wgpu::Extent3d {
                    width, 
                    height, 
                    depth_or_array_layers: 1
                }, 
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
                view_formats: &[]
            }
        ).create_view(&wgpu::TextureViewDescriptor::default()));
    }

    /// 애플리케이션 키보드 이벤트가 발생했을 때 호출되는 콜백 함수입니다.
    pub fn on_keyboard_input(&mut self, event: &KeyEvent) {
        if let PhysicalKey::Code(keycode) = &event.physical_key {
            if let Some(&key) = KEY_MAP.get(keycode) {
                self.egui_raw_input.events.push(egui::Event::Key { 
                    key, 
                    physical_key: Some(key), 
                    pressed: event.state.is_pressed(), 
                    repeat: event.repeat, 
                    modifiers: self.egui_raw_input.modifiers 
                });
            }
        }
    }

    /// 애플리케이션 마우스 버튼 이벤트가 발생했을 떄 호출되는 콜백 함수입니다.
    pub fn on_mouse_input(&mut self, x: f64, y: f64, state: &ElementState, button: &MouseButton) {
        let button = match button {
            MouseButton::Left => Some(egui::PointerButton::Primary), 
            MouseButton::Right => Some(egui::PointerButton::Secondary), 
            MouseButton::Middle => Some(egui::PointerButton::Middle), 
            MouseButton::Back => Some(egui::PointerButton::Extra1), 
            MouseButton::Forward => Some(egui::PointerButton::Extra2), 
            _ => None
        };

        if let Some(button) = button {
            self.egui_raw_input.events.push(egui::Event::PointerButton { 
                pos: egui::pos2(x as f32, y as f32), 
                button, 
                pressed: state.is_pressed(), 
                modifiers: self.egui_raw_input.modifiers 
            });
        }
    }

    /// 애플리케이션 마우스 휠 이벤트가 발생했을 때 호출되는 콜백 함수입니다.
    pub fn on_mouse_wheel(&mut self, dx: f32, dy: f32) {
        self.egui_raw_input.events.push(egui::Event::MouseWheel { 
            unit: egui::MouseWheelUnit::Point, 
            delta: egui::vec2(dx, dy), 
            modifiers: self.egui_raw_input.modifiers 
        });
    }

    /// 애플리케이션 키보드 수정자 변경 이벤트가 발생했을 때 호출되는 콜백 함수입니다.
    pub fn on_modifiers_changed(&mut self, modifier: &Modifiers) {
        self.egui_raw_input.modifiers.alt = modifier.state().alt_key();
        self.egui_raw_input.modifiers.ctrl = modifier.state().control_key();
        self.egui_raw_input.modifiers.shift = modifier.state().shift_key();
        self.egui_raw_input.modifiers.mac_cmd = cfg!(target_os = "macos") | modifier.state().super_key();
        self.egui_raw_input.modifiers.command = if cfg!(target_os = "macos") {
            modifier.state().super_key()
        } else {
            modifier.state().control_key()
        };
    }

    /// `winit` 윈도우 핸들을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// `wgpu` 장치 표면을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn surface(&self) -> &Arc<wgpu::Surface<'static>> {
        &self.surface
    }

    /// 깊이 버퍼 뷰를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn depth_buffer_view(&self) -> &Arc<wgpu::TextureView> {
        &self.depth_buffer_view
    }

    /// `egui` 입력기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn egui_raw_input(&self) -> egui::RawInput {
        self.egui_raw_input.clone()
    }
}





lazy_static! {
    static ref KEY_MAP: HashMap<KeyCode, egui::Key> = HashMap::from_iter([
        (KeyCode::ArrowDown, egui::Key::ArrowDown), 
        (KeyCode::ArrowLeft, egui::Key::ArrowLeft), 
        (KeyCode::ArrowRight, egui::Key::ArrowRight), 
        (KeyCode::ArrowUp, egui::Key::ArrowUp), 

        (KeyCode::Escape, egui::Key::Escape), 
        (KeyCode::Tab, egui::Key::Tab), 
        (KeyCode::Backspace, egui::Key::Backspace), 
        (KeyCode::Enter, egui::Key::Enter), 
        (KeyCode::NumpadEnter, egui::Key::Enter), 

        (KeyCode::Insert, egui::Key::Insert),
        (KeyCode::Delete, egui::Key::Delete),
        (KeyCode::Home, egui::Key::Home),
        (KeyCode::End, egui::Key::End),
        (KeyCode::PageUp, egui::Key::PageUp),
        (KeyCode::PageDown, egui::Key::PageDown),

        (KeyCode::Space, egui::Key::Space),
        (KeyCode::Comma, egui::Key::Comma),
        (KeyCode::Period, egui::Key::Period),
        (KeyCode::Semicolon, egui::Key::Semicolon),
        (KeyCode::Backslash, egui::Key::Backslash),
        (KeyCode::Slash, egui::Key::Slash), 
        (KeyCode::NumpadDivide, egui::Key::Slash),
        (KeyCode::BracketLeft, egui::Key::OpenBracket),
        (KeyCode::BracketRight, egui::Key::CloseBracket),
        (KeyCode::Backquote, egui::Key::Backtick),
        (KeyCode::Quote, egui::Key::Quote),

        (KeyCode::Cut, egui::Key::Cut),
        (KeyCode::Copy, egui::Key::Copy),
        (KeyCode::Paste, egui::Key::Paste),
        (KeyCode::Minus, egui::Key::Minus), 
        (KeyCode::NumpadSubtract, egui::Key::Minus), 
        (KeyCode::NumpadAdd, egui::Key::Plus),
        (KeyCode::Equal, egui::Key::Equals),

        (KeyCode::Digit0, egui::Key::Num0),
        (KeyCode::Digit1, egui::Key::Num1),
        (KeyCode::Digit2, egui::Key::Num2),
        (KeyCode::Digit3, egui::Key::Num3),
        (KeyCode::Digit4, egui::Key::Num4),
        (KeyCode::Digit5, egui::Key::Num5),
        (KeyCode::Digit6, egui::Key::Num6),
        (KeyCode::Digit7, egui::Key::Num7),
        (KeyCode::Digit8, egui::Key::Num8),
        (KeyCode::Digit9, egui::Key::Num9),
        
        (KeyCode::Numpad0, egui::Key::Num0),
        (KeyCode::Numpad1, egui::Key::Num1),
        (KeyCode::Numpad2, egui::Key::Num2),
        (KeyCode::Numpad3, egui::Key::Num3),
        (KeyCode::Numpad4, egui::Key::Num4),
        (KeyCode::Numpad5, egui::Key::Num5),
        (KeyCode::Numpad6, egui::Key::Num6),
        (KeyCode::Numpad7, egui::Key::Num7),
        (KeyCode::Numpad8, egui::Key::Num8),
        (KeyCode::Numpad9, egui::Key::Num9),

        (KeyCode::KeyA, egui::Key::A),
        (KeyCode::KeyB, egui::Key::B),
        (KeyCode::KeyC, egui::Key::C),
        (KeyCode::KeyD, egui::Key::D),
        (KeyCode::KeyE, egui::Key::E),
        (KeyCode::KeyF, egui::Key::F),
        (KeyCode::KeyG, egui::Key::G),
        (KeyCode::KeyH, egui::Key::H),
        (KeyCode::KeyI, egui::Key::I),
        (KeyCode::KeyJ, egui::Key::J),
        (KeyCode::KeyK, egui::Key::K),
        (KeyCode::KeyL, egui::Key::L),
        (KeyCode::KeyM, egui::Key::M),
        (KeyCode::KeyN, egui::Key::N),
        (KeyCode::KeyO, egui::Key::O),
        (KeyCode::KeyP, egui::Key::P),
        (KeyCode::KeyQ, egui::Key::Q),
        (KeyCode::KeyR, egui::Key::R),
        (KeyCode::KeyS, egui::Key::S),
        (KeyCode::KeyT, egui::Key::T),
        (KeyCode::KeyU, egui::Key::U),
        (KeyCode::KeyV, egui::Key::V),
        (KeyCode::KeyW, egui::Key::W),
        (KeyCode::KeyX, egui::Key::X),
        (KeyCode::KeyY, egui::Key::Y),
        (KeyCode::KeyZ, egui::Key::Z),

        (KeyCode::F1, egui::Key::F1),
        (KeyCode::F2, egui::Key::F2),
        (KeyCode::F3, egui::Key::F3),
        (KeyCode::F4, egui::Key::F4),
        (KeyCode::F5, egui::Key::F5),
        (KeyCode::F6, egui::Key::F6),
        (KeyCode::F7, egui::Key::F7),
        (KeyCode::F8, egui::Key::F8),
        (KeyCode::F9, egui::Key::F9),
        (KeyCode::F10, egui::Key::F10),
        (KeyCode::F11, egui::Key::F11),
        (KeyCode::F12, egui::Key::F12),
        (KeyCode::F13, egui::Key::F13),
        (KeyCode::F14, egui::Key::F14),
        (KeyCode::F15, egui::Key::F15),
        (KeyCode::F16, egui::Key::F16),
        (KeyCode::F17, egui::Key::F17),
        (KeyCode::F18, egui::Key::F18),
        (KeyCode::F19, egui::Key::F19),
        (KeyCode::F20, egui::Key::F20),
        (KeyCode::F21, egui::Key::F21),
        (KeyCode::F22, egui::Key::F22),
        (KeyCode::F23, egui::Key::F23),
        (KeyCode::F24, egui::Key::F24),
        (KeyCode::F25, egui::Key::F25),
        (KeyCode::F26, egui::Key::F26),
        (KeyCode::F27, egui::Key::F27),
        (KeyCode::F28, egui::Key::F28),
        (KeyCode::F29, egui::Key::F29),
        (KeyCode::F30, egui::Key::F30),
        (KeyCode::F31, egui::Key::F31),
        (KeyCode::F32, egui::Key::F32),
        (KeyCode::F33, egui::Key::F33),
        (KeyCode::F34, egui::Key::F34),
        (KeyCode::F35, egui::Key::F35),
    ]);
}
