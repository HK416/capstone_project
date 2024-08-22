use std::error::Error;
use std::fmt::Debug;

use hecs::World;
use mod_util::AppHandle;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;
use winit::keyboard::KeyLocation;
use winit::window::Window;



/// 게임 장면 `trait`입니다.
pub trait GameScene : Debug {
    /// 게임 장면의 투명한 여부를 반환합니다.
    /// 
    /// ※ 게임 장면이 투명할 경우 하위 게임 장면도 그려집니다.
    /// 
    #[inline]
    fn transparents(&self) -> bool {
        false
    }

    /// 애플리케이션 창의 종료 버튼이 눌렸을 때 호출되는 콜백 함수입니다.
    /// 
    /// 애플리케이션 종료를 원할 경우 `true`를 반환해야 합니다.
    /// 
    #[inline]
    #[allow(unused_variables)]
    fn on_close(&mut self, app: &dyn AppHandle) -> bool {
        true
    }

    /// 게임 장면에 진입할 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        log::info!("게임 장면({:?})에 진입함.", self);
        Ok(())
    }

    /// 게임 장면에 빠져나올 때 한번만 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        log::info!("게임 장면({:?})에 빠져나옴.", self);
        Ok(())
    }

    /// 애플리케이션이 일시정지될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_pause(
        &mut self, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        log::info!("게임 장면({:?})이 일시정지됨.", self);
        Ok(())
    }

    /// 애플리케이션이 재개될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resume(
        &mut self, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        log::info!("게임 장면({:?})이 재개됨.", self);
        Ok(())
    }

    /// 애플리케이션 창의 크기가 변경될 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_resized(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 눌림 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_pressed(
        &mut self, 
        x: i32, y: i32, 
        button: MouseButton, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 떼임 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_released(
        &mut self, 
        x: i32, y: i32, 
        button: MouseButton, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 이동 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_moved(
        &mut self, 
        x: i32, y: i32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 버튼이 눌린채로 이동 이벤트가 발생할 떄 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_dragged(
        &mut self, 
        x: i32, y: i32, 
        button: MouseButton, 
        window: &Window, 
        world: &World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 마우스 스크롤 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_mouse_wheel(
        &mut self, 
        delta_x: f32, 
        delta_y: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 키보드 눌림 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_keyboard_pressed(
        &mut self, 
        code: KeyCode, 
        location: KeyLocation, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 애플리케이션 창에 키보드 떼임 이벤트가 발생할 때 호출되는 콜백함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_keyboard_released(
        &mut self, 
        code: KeyCode, 
        location: KeyLocation, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 일정 시간만큼 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    /// 
    /// ※ 이 함수는 한 게임 루프에서 여러번 호출될 수 있습니다.
    /// 
    #[inline]
    #[allow(unused_variables)]
    fn on_fixed_update(
        &mut self, 
        fixed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 게임 장면이 그려지기 전 호출되는 콜백 함수입니다.
    #[inline]
    #[allow(unused_variables)]
    fn on_prepare_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /// 게임 장면이 그려질 때 호출되는 콜백 함수입니다.
    fn on_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>>;
}
