use std::{error::Error, fmt, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::{CameraResource, ScreenDescriptor, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT};
use winit::window::Window;

use crate::{
    component::{
        add_child, CameraBehaviorState, CameraTag, Projection, ToParentTrans, WorldTransform,
    },
    system::{
        sys_prepare_camera_resource, sys_prepare_mesh_resource, sys_student_animation,
        sys_student_draw, sys_student_hierarchy,
    },
};

const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 116.0 / 255.0,
    b: 183.0 / 255.0,
    a: 1.0,
};

pub struct TestbedInGameScene {
    /// 메인 카메라의 `Entity`
    main_camera: Entity,

    /// 유저의 클라이언트 식별자입니다.
    client_id: u32,

    /// 엔티티 목록입니다.
    entities: HashMap<u32, Entity>,

    world: World,

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedInGameScene {
    pub fn new(client_id: u32, entities: HashMap<u32, Entity>, world: World) -> Self {
        Self {
            main_camera: Entity::DANGLING,
            client_id,
            entities,
            world,
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    #[allow(unused_variables)]
    pub fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();
        let timer = app.timer();
        let frame_rate = timer.frame_rate();

        let frame_rate_text = egui::RichText::new(format!("FPS:{}", frame_rate))
            .color(egui::Color32::WHITE)
            .background_color(egui::Color32::from_black_alpha(128))
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.label(frame_rate_text);
            });

        Ok(())
    }

    pub fn prepare_ui(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();
        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, height): (f32, f32) = window.inner_size().into();

        let mut builder = EntityBuilder::new();
        builder.add(CameraTag);
        builder.add(CameraBehaviorState::Idle);
        builder.add(ToParentTrans(glam::Mat4::from_translation(glam::vec3(
            0.25, 0.75, -1.2,
        ))));
        builder.add(WorldTransform::default());
        builder.add(Projection(glam::Mat4::perspective_lh(
            75f32.to_radians(),
            width / height,
            0.0001,
            1000.0,
        )));
        builder.add(Arc::new(CameraResource::uninit(
            Some("main_camera"),
            app.render_device(),
        )));

        log::debug!("TestbedInGameScene :: 메인 카메라를 생성");
        self.main_camera = self.world.spawn(builder.build());

        log::debug!("TestbedInGameScene :: 메인 카메라를 학생에게 부착");
        let entity = self.entities.get(&self.client_id).cloned().unwrap();
        add_child(&mut self.world, entity, self.main_camera).unwrap();
    }
}

impl GameScene for TestbedInGameScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.create_main_camera(window, app);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        sys_student_animation(
            &mut self.world,
            app.asset_manager(),
            elapsed_time_sec,
            rayon::current_num_threads() as u32,
        );
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        sys_student_hierarchy(&mut self.world).map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

        sys_prepare_mesh_resource(
            &self.world,
            &self.main_camera,
            app.render_device(),
            app.render_queue(),
            rayon::current_num_threads() as u32,
        );

        sys_prepare_camera_resource(
            &self.world,
            &[self.main_camera],
            app.render_device(),
            app.render_queue(),
        );

        self.prepare_ui(window, egui_renderer, app)?;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let camera = self
            .world
            .get::<&Arc<CameraResource>>(self.main_camera)
            .unwrap();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TestbadEnterScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            sys_student_draw(
                &self.world,
                device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &camera,
                &mut rpass,
            );

            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl fmt::Debug for TestbedInGameScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestbedInGameScene))
    }
}
