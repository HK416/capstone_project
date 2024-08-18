use winit::application::ApplicationHandler;
use winit::keyboard::PhysicalKey;
use winit::window::WindowId;
use winit::event::StartCause;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use crate::error::success;
use crate::error::show_error_msg;
use crate::app::App;
use crate::app::AppFlags;
use crate::app::AppEvent;
use crate::render::targets::config_swapchain;
use crate::render::targets::create_wgpu_surface;



impl ApplicationHandler<AppEvent> for App {
    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        // 타이머를 갱신합니다.
        self.timer.tick();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // `winit` API는 애플리케이션이 생성되었을 때 `ApplicationHandler::resumed`를 호출합니다.
        // 또한 일부 시스템은 애플리케이션 초기화 이전에 창을 생성하는 것이 허용되지 않습니다.
        // 따라서 이 콜백 함수에서 애플리케이션 창을 생성하고, 렌더러 표면을 생성해야 합니다.
        //

        // 애플리케이션 창을 생성합니다.
        let window = success!(
            "Application Window Creation Failed", 
            self.create_window(event_loop), 
            None
        );

        // `wgpu` 렌더링 표면을 생성합니다.
        let surface = success!(
            "Render Surface Creation Failed", 
            create_wgpu_surface(window.clone(), &self.instance, &self.adapter), 
            Some(&window)
        );

        // 스왑체인을 설정합니다.
        let size = window.inner_size();
        config_swapchain(
            size.width, 
            size.height, 
            &self.device, 
            &surface, 
            self.flags.contains(AppFlags::DISABLE_VSYNC)
        );

        // 애플리케이션 시작 콜백 함수를 호출합니다.
        self.on_launching(&window, event_loop);

        // 애플리케이션에 저장합니다.
        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        // 애플리케이션 종료 콜백 함수를 호출합니다.
        self.on_finish(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.window.clone();
        let surface = self.surface.clone();
        if let Some((window, _)) = window.zip(surface) {
            // 게임 장면이 비어있는 경우 애플리케이션을 종료합니다.
            if self.scene_manager.borrow().is_empty() {
                return event_loop.exit();
            } 

            // 현재 장면을 갱신합니다.
            if let Err(e) = self.scene_manager.borrow_mut().scene_update(&window, self) {
                show_error_msg(
                    "Application Runtime Error", 
                    e.to_string(), 
                    self.window.as_deref()
                );
                return event_loop.exit();
            }

            // 등록된 애플리케이션 창이 존재할 경우 애플리케이션 창을 갱신합니다.
            window.request_redraw();
        } else {
            // 등록된 애플리케이션 창이 없는 경우 애플리케이션을 종료합니다.
            event_loop.exit();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: AppEvent) {
        /* empty */
    }

    fn window_event(
        &mut self,
        _: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // MEMO: 이 콜백 함수 안에서 이벤트 루프를 통한 종료를 하면 에러가 발생합니다.
        // 

        // 애플리케이션 창과 렌더링 표면을 가져옵니다.
        // 애플리케이션 창 또는 렌더링 표면이 없는 경우 (애플리케이션의 종료) 함수 실행을 생략합니다.
        let window = self.window.clone();
        let surface = self.surface.clone();
        let (window, surface) = match window.zip(surface) {
            Some(it) => it,
            None => return,
        };

        // 애플리케이션 창 식별자가 다른 경우 함수 실행을 생략합니다.
        if window_id != window.id() {
            return;
        }

        // 애플리케이션 창 이벤트를 처리합니다.
        if let Err(e) = match event {
            WindowEvent::Focused(focused) => match focused {
                true => self.on_resumed(&window),
                false => self.on_paused(&window)
            }, 
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state.is_pressed() {
                        self.on_keyboard_pressed(code, event.location, &window)
                    } else {
                        self.on_keyboard_released(code, event.location, &window)
                    }
                } else {
                    Ok(())
                }
            },
            WindowEvent::Resized(_) => self.on_resized(&window, &surface),
            WindowEvent::RedrawRequested => self.on_draw(&window, &surface),
            WindowEvent::CloseRequested => match self.on_close() {
                Ok(exiting) => {
                    if exiting {
                        drop(self.window.take());
                        drop(self.surface.take());
                    }
                    Ok(())
                },
                Err(e) => Err(e),
            },
            _ => { Ok(()) }
        } {
            show_error_msg(
                "Application runtime error", 
                e.to_string(), 
                self.window.as_deref()
            );
            drop(self.window.take());
            drop(self.surface.take());
        }
    }
}
