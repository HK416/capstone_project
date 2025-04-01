use std::fmt::Debug;

use mod_network::protocol::RawPacket;
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{app::AppHandle, net::NetworkError};

/// ## Game Scene Control Flow
/// 게임 장면 전환을 제어합니다.
#[derive(Debug)]
pub enum GameSceneFlow {
    Clear,
    Change(Box<dyn GameScene>),
    Push(Box<dyn GameScene>),
    Pop,
    Reset(Box<dyn GameScene>),
}

/// ## Game Scene Interface
#[allow(unused_variables)]
pub trait GameScene: Debug + Send {
    /// 현재 게임 장면이 투명한지 여부를 반환합니다.
    ///
    /// `true`를 반환할 경우 게임 장면을 그릴 때 하위 장면도 그려집니다.
    /// (하위 장면이 갱신되지는 않습니다)
    ///
    fn transparents(&self) -> bool {
        false
    }

    /// 애플리케이션 창의 종료 버튼이 눌렸을 때 호출되는 콜백 함수입니다.  
    /// `true`를 반환할 경우 애플리케이션을 종료합니다.
    fn on_close_request(&mut self, app: &dyn AppHandle) -> bool {
        true
    }

    /// 게임 장면에 진입할 때 한 번만 호출되는 콜백 함수입니다.
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 게임 장면에서 빠져나올 때 한 번만 호출되는 콜백 함수입니다.
    fn on_exit(&mut self, window: Option<&Window>, app: &dyn AppHandle) {
        /* empty */
    }

    /// 게임 장면을 일시 정지할 때 호출되는 콜백 함수입니다.
    fn on_pause(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 게임 장면을 재개할 때 호출되는 콜백 함수입니다.
    fn on_resume(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션이 background로 이동될 때 호출되는 콜백 함수입니다.
    fn on_enter_background(&mut self, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션이 foreground로 이동될 때 호출되는 콜백 함수입니다.
    fn on_enter_foreground(&mut self, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션 창의 크기가 변경되거나 창이 다른 모니터로 이동되어 `Scale`값이 달라질 때 호출되는 콜백 함수입니다.
    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션 창이 이동될 때 호출되는 콜백 함수입니다.
    fn on_window_moved(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션에 키보드 눌림 이벤트가 발생됐을 때 호출되는 콜백 함수입니다.
    fn on_keyboard_pressed(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        modifiers: Modifiers,
        repeat: bool,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        /* empty */
    }

    /// 애플리케이션에 키보드 떼임 이벤트가 발생됐을 때 호출되는 콜백 함수입니다.
    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        modifiers: Modifiers,
        repeat: bool,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        /* empty */
    }

    /// 애플리케이션에 마우스 버튼 눌림 이벤트가 발생됐을 때 호출되는 콜백 함수입니다.
    fn on_mouse_btn_pressed(
        &mut self,
        x: f32,
        y: f32,
        button: MouseButton,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        /* empty */
    }

    /// 애플리케이션에 마우스 버튼 떼임 이벤트가 발생됐을 때 호출되는 콜백 함수입니다.
    fn on_mouse_btn_released(
        &mut self,
        x: f32,
        y: f32,
        button: MouseButton,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        /* empty */
    }

    /// 애플리케이션 창에 마우스 휠 조작 이벤트가 발생됐을 때 호출되는 콜백 함수입니다.
    fn on_mouse_wheel(&mut self, dx: f32, dy: f32, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션 창에 커서가 이동됐을 때 호출되는 콜백 함수입니다.
    fn on_cursor_moved(
        &mut self,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        /* empty */
    }

    /// 애플리케이션에서 네트워크 오류를 처리합니다.
    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션이 패킷을 수신받았을 때 호출되는 콜백 함수입니다.
    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) {
        /* empty */
    }

    /// 게임 장면을 갱신 전에 호출되는 콜백 함수입니다.
    fn on_pre_update(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    fn on_update(&mut self, elapsed_time_sec: f32, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 지정된 일정 시간만큼 게임 장면을 갱신할 때 호출되는 콜백 함수입니다.
    ///
    /// 이 함수는 한 게임 루프에서 여러 번 호출될 수 있으며,
    /// 너무 많이 호출될 경우 `fixed_time_sec`이 지정한 시간임을 보장할 수 없습니다.
    ///
    fn on_fixed_update(&mut self, fixed_time_sec: f32, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 게임 장면을 갱신 후에 호출되는 콜백 함수입니다.
    fn on_post_update(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션이 게임 장면을 그리기 전에 호출하는 콜백 함수입니다.
    fn on_prepare_draw(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// UI 콜백 함수입니다.
    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }

    /// 애플리케이션이 게임 장면을 그릴 때 호출하는 콜백 함수입니다.
    fn on_draw(
        &self,
        window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        app: &dyn AppHandle,
    ) {
        /* empty */
    }

    /// 애플리케이션이 게임 장면을 그린 후에 호출하는 콜백 함수입니다.
    fn on_finish_draw(&mut self, window: &Window, app: &dyn AppHandle) {
        /* empty */
    }
}
