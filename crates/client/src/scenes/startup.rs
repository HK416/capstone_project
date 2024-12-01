use std::{error::Error, fmt};

use hecs::World;
use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::UiRenderer;
use winit::window::Window;

use crate::asset::{ModelHierarchyPool, MotionPool};

/// 게임을 초기화 하는 장면입니다.
/// 게임 모델을 불러오거나 게임 서버와 연결을 하는 작업을 수행합니다.
//
pub struct StartupScene {}

impl StartupScene {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameScene for StartupScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let mut world = World::new();
        let (_, _, batch_commands) = ModelHierarchyPool::spawn(
            "aris_original",
            "characters/aris_original",
            app.bundle(),
            app.render_device(),
            app.render_queue(),
            &world,
        )
        .unwrap();

        MotionPool::get_or_init(
            "aris_original",
            "characters/aris_original",
            app.bundle(),
            |map| println!("okay!"),
        )
        .unwrap();

        for (entity, mut builder) in batch_commands {
            world.spawn_at(entity, builder.build());
        }

        println!("finish!");

        Ok(())
    }

    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_stencil_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 게임을 초기화 하는 동안 검정색 화면을 출력합니다.
        //
        let device = app.render_device();
        let queue = app.render_queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(StartupScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        queue.submit([encoder.finish()]);
        Ok(())
    }
}

impl fmt::Debug for StartupScene {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(StartupScene))
    }
}
