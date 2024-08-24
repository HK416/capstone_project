use std::error::Error;

use hecs::World;
use mod_app::AppBuilder;
use mod_scene::AppHandle;
use mod_scene::GameScene;
use mod_scene::GameSceneFlow;
use winit::window::Window;



/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점입니다.
/// 
/// 게임 화면은 16 : 9 비율의 scaled 크기를 가집니다.
/// 
/// `Windows`, `macOS` 플랫폼의 경우 최초 실행시 전체 화면으로 실행됩니다.
/// 
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    use mod_parallelism::is_main_thread;
    assert!(is_main_thread(), "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(Foo::new()))
        .with_title("Hello to Halo!")
        .with_dpi(mod_util::AppDpi::W1280H720)
        .with_fullscreen(false)
        .build_and_run()
}



#[derive(Debug)]
pub struct Foo {
    timer: f32, 
}

impl Foo {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { timer: 0.0 }
    }
}

impl GameScene for Foo {
    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        self.timer += elapsed_time_sec;
        if self.timer >= 3.0 {
            app.set_scene_flow(GameSceneFlow::Change(Box::new(Far::new())));
        }
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let mut encoder = app.render_device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor { ..Default::default() }
        );

        {
            let _rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Foo"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), 
                                store: wgpu::StoreOp::Store, 
                            }, 
                            view: render_target_view, 
                            resolve_target: None,
                        }),
                    ],
                    depth_stencil_attachment: None, 
                    timestamp_writes: None, 
                    occlusion_query_set: None, 
                }
            );
        }

        app.render_queue().submit([encoder.finish()]);
        Ok(())
    }
}



#[derive(Debug)]
pub struct Far {
    timer: f32, 
}

impl Far {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { timer: 0.0 }
    }
}

impl GameScene for Far {
    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        self.timer += elapsed_time_sec;
        if self.timer >= 3.0 {
            app.set_scene_flow(GameSceneFlow::Change(Box::new(Foo::new())));
        }
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let mut encoder = app.render_device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor { ..Default::default() }
        );

        {
            let _rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Far"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), 
                                store: wgpu::StoreOp::Store, 
                            }, 
                            view: render_target_view, 
                            resolve_target: None,
                        }),
                    ],
                    depth_stencil_attachment: None, 
                    timestamp_writes: None, 
                    occlusion_query_set: None, 
                }
            );
        }

        app.render_queue().submit([encoder.finish()]);
        Ok(())
    }
}
