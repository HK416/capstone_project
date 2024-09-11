use std::error::Error; 
use std::fmt::Debug;

use hecs::World;
use mod_network::RawPacket;
use winit::event::{Modifiers, MouseButton};
use winit::keyboard::{KeyCode, KeyLocation};
use winit::window::Window;

use crate::AppHandle;



/// 게임 장면 접근 인터페이스입니다.
pub trait GameScene : Debug {
    /// 투명한 게임 장면인 경우 `true`를 반환합니다.
    /// 게임 장면이 투명할 경우 하위 게임 장면도 그려집니다.
    #[inline]
    fn transparents(&self) -> bool {
        false
    }

    /// 애플리케이션 창의 종료 버튼이 눌렸을 때 호출되는 콜백함수입니다.
    /// `true`를 반환할 경우 애플리케이션을 종료합니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_close_request(&mut self, app: &dyn AppHandle) -> bool {
        true
    }

    /// 게임 장면에 진입할 때 한 번만 호출되는 콜백함수입니다.
    /// 일반적으로 게임 장면을 초기화하는 데 사용합니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        log::debug!("{:?} > 게임 장면에 진입했습니다.", self);
        Ok(())
    }

    /// 게임 장면에 빠져나올 때 한 번만 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        log::debug!("{:?} > 게임 장면에 빠져나왔습니다.", self);
        Ok(())
    }

    /// 애플리케이션이 일시정지될 때 호출되는 콜백함수입니다.
    /// 
    /// 애플리케이션이 일시정지되는 상황은 다음과 같습니다.
    /// - 애플리케이션 창이 비활성화 될 때 
    /// 
    #[inline]
    #[allow(unused_variables)]
    fn on_pause(
        &mut self, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        log::debug!("{:?} > 게임 장면을 일시정지합니다.", self);
        Ok(())
    }

    /// 애플리케이션이 재개될 때 호출되는 콜백함수입니다.
    /// 
    /// 애플리케이션이 재개되는 상황은 다음과 같습니다.
    /// - 애플리케이션 창이 활성화 될 때 
    /// 
    #[inline]
    #[allow(unused_variables)]
    fn on_resume(
        &mut self, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        log::debug!("{:?} > 게임 장면을 재개합니다.", self);
        Ok(())
    }

    /// 애플리케이션 창의 크기 또는 모니터의 Dpi가 변경될 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resized(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창이 이동할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_moved(
        &mut self, 
        x: i32, 
        y: i32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창에 키보드 눌림 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_keyboard_pressed(
        &mut self, 
        keycode: KeyCode, 
        location: KeyLocation, 
        modifiers: Modifiers, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창에 키보드 떼임 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_keyboard_released(
        &mut self, 
        keycode: KeyCode, 
        location: KeyLocation, 
        modifiers: Modifiers, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 눌림 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_pressed(
        &mut self, 
        x: f32, 
        y: f32, 
        button: MouseButton, 
        window: &Window, 
        world: &mut World,
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 떼임 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_released(
        &mut self, 
        x: f32, 
        y: f32, 
        button: MouseButton, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 휠 조작 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_wheel(
        &mut self, 
        delta_x: f32, 
        delta_y: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 이동 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_cursor_moved(
        &mut self, 
        x: f32, 
        y: f32, 
        delta_x: f32, 
        delta_y: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 애플리케이션이 패킷을 수신받았을 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_received_packet(
        &mut self, 
        raw_packet: RawPacket, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 게임 장면을 갱신할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 일정 시간만큼 게임 장면을 갱신할 때 호출되는 콜백함수입니다.
    /// 이 함수는 한 게임루프에서 여러 번 호출될 수 있습니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_fixed_update(
        &mut self, 
        fixed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 게임 장면을 그리기 전에 호출되는 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_prepare_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }

    /// 게임 장면을 그릴 때 호출되는 콜백함수입니다.
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>>;
}
