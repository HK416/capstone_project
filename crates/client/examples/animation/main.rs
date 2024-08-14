mod model;
use self::model::*;

use std::fmt;
use std::sync::Arc;
use std::thread;

use client_framework::app::AppBuilder;
use client_framework::app::Dpi;
use client_framework::app::Handler;
use client_framework::error::ErrorMessage;
use client_framework::render::camera::CameraDataLayout;
use client_framework::render::camera::CameraObject;
use client_framework::render::camera::PerspectiveRh;
use client_framework::render::camera::Projection;
use client_framework::render::material::Material;
use client_framework::render::mesh::Attribute;
use client_framework::render::mesh::Mesh;
use client_framework::render::object::GameObject;
use client_framework::render::object::GameObjectDataLayout;
use client_framework::render::object::Transform;
use client_framework::render::object::WorldTransform;
use client_framework::render::pipeline::TexturePipeline;
use client_framework::render::targets::DepthBuffer;
use client_framework::scene::GameScene;
use framework::concurrency::MAIN_THREAD_ID;
use hecs::Entity;
use hecs::World;
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
    assert_eq!(thread::current().id(), *MAIN_THREAD_ID, "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(ExampleScene::default()))
        .set_title("Hello to Halo!")
        .set_dpi(Dpi::W1280H720)
        .set_fullscreen(false)
        .build_and_run()
}



/// 플레이어 식별 컴포넌트 입니다.
#[derive(Debug)]
pub struct Player;



/// 예제 장면입니다.
struct ExampleScene {
    pipeline: Option<Arc<TexturePipeline>>, 
    main_camera: Entity, 
    player: Entity, 
}

impl ExampleScene {
    /// 카메라 오브젝트를 생성합니다.
    fn spawn_main_camera(&mut self, window: &Window, world: &mut World, app: &dyn Handler) {
        // 로컬 변환 행렬과 월드 변환 행렬을 생성합니다.
        let trans = Transform::from_rotation_translation(
            gmm::Quaternion::from_rotation_y(180f32.to_radians()), 
            gmm::Float3::new(0.0, 0.005, 0.015)
        );
        let mut world_trans = WorldTransform::new();
        (*world_trans) = *trans;

        // 투영 변환 행렬을 생성합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        let projection: Projection = PerspectiveRh::new()
            .with_fov_y(60f32.to_radians())
            .with_aspect_ratio(width as f32 / height as f32)
            .into();

        // 카메라 오브젝트 데이터를 생성합니다.
        let camera_object = CameraObject::new(
            Some("Main"), 
            app.ref_render_device()
        );

        // 카메라 오브젝트를 생성합니다.
        self.main_camera = world.spawn((
            trans, 
            world_trans, 
            projection, 
            camera_object, 
        ));
    }

    // 모델 에셋을 생성합니다.
    fn spawn_aris_original_model(&mut self, world: &mut World, app: &dyn Handler) {
        self.player = spawn_model(
            app.ref_render_device(), 
            app.ref_render_queue(), 
            world, 
            "Aris_Original.ron"
        );
    }

    fn update_hierarchy(
        world: &mut World, 
        parent: gmm::Matrix, 
        entity: Entity
    ) {
        let world_transform = {
            let transform = *world.get::<&Transform>(entity).unwrap();
            let world_transform = world.query_one_mut::<&mut WorldTransform>(entity).unwrap();
            (**world_transform) = parent * (*transform);
            **world_transform
        };

        let children = world.get::<&GameObject>(entity)
            .map(|object| object.children.clone())
            .unwrap_or_default();

        for child in children {
            Self::update_hierarchy(world, world_transform, child);
        }
    }

    /// 카메라를 준비합니다.
    fn preapre_camera(world: &mut World) {
        type QueryType<'a> = (&'a WorldTransform, &'a Projection, &'a CameraObject);
        let mut query = world.query::<QueryType>();
        for (_, (world_trans, projection, uniform)) in query.iter() {
            let eye = world_trans.get_translation();
            let dir = world_trans.get_forward_vector();
            let up = world_trans.get_up_vector();
            let view_trans = gmm::Matrix::look_to_rh(eye, dir, up);

            uniform.update(CameraDataLayout {
                proj_view: ((**projection) * view_trans).into(), 
                position: eye.into(), 
                direction: dir.into(), 
                ..Default::default()
            });
        }
    }

    fn prepare_objects(world: &mut World) {
        type QueryType<'a> = (&'a WorldTransform, &'a GameObject);
        let mut query = world.query::<QueryType>();
        for (_, (world_trans, uniform)) in query.iter() {
            uniform.update(GameObjectDataLayout {
                transform: (**world_trans).into(), 
            });
        }
    }
}

impl GameScene for ExampleScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        self.pipeline = Some(TexturePipeline::new(app.ref_render_device()));
        self.spawn_main_camera(window, world, app);
        self.spawn_aris_original_model(world, app);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        world.clear();
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Handler 
    ) -> Result<(), ErrorMessage> {
        // 플레이어 오브젝트 회전
        let world_transform = {
            type QueryPlayer<'a> = (&'a mut Transform, &'a mut WorldTransform, &'a GameObject);
            let (
                transform, 
                world_transform, 
                object
            ) = world.query_one_mut::<QueryPlayer>(self.player).unwrap();

            let rotation = gmm::Matrix::from_rotation_y(30f32.to_radians() * elapsed_time_sec);
            (**transform) = (**transform) * rotation;
            (**world_transform) = **transform;
            **world_transform
        };

        let children = world.get::<&GameObject>(self.player)
            .map(|object| object.children.clone())
            .unwrap_or_default();
        for entity in children {
            Self::update_hierarchy(world, world_transform, entity);
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_prepare_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        Self::preapre_camera(world);
        Self::prepare_objects(world);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        let device = app.ref_render_device();
        let queue = app.ref_render_queue();

        // 이전 작업이 끝날 때 까지 기다립니다.
        device.poll(wgpu::Maintain::Wait);

        // 현재 스왑체인 이미지를 가져옵니다.
        let frame = surface.get_current_texture()
            .expect("Failed to get swapchain texture.");
        
        // 렌더 타겟 뷰를 가져옵니다.
        let render_target_view = frame.texture.create_view(
            &wgpu::TextureViewDescriptor { ..Default::default() }
        );

        // 명령어 레코더를 생성합니다.
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { ..Default::default() }
        );

        // 카메라 오브젝트를 가져옵니다.
        let camera = world.get::<&CameraObject>(self.main_camera).unwrap();

        // 오브젝트를 가져옵니다.
        type QueryType<'a> = (&'a Mesh, &'a GameObject, &'a Vec<Material>);
        let mut query = world.query::<QueryType>();

        {
            let mut rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(TexturePipeline)"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &render_target_view, 
                            resolve_target: None, 
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color {
                                        r: 0.0, 
                                        g: 116.0 / 255.0, 
                                        b: 183.0 / 255.0, 
                                        a: 1.0, 
                                    }
                                ), 
                                store: wgpu::StoreOp::Store, 
                            }, 
                        }), 
                    ], 
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: DepthBuffer::get(window, device), 
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), 
                            store: wgpu::StoreOp::Store, 
                        }), 
                        stencil_ops: None, 
                    }), 
                    timestamp_writes: None, 
                    occlusion_query_set: None, 
                }
            );

            rpass.set_pipeline(self.pipeline.as_ref().unwrap());
            rpass.set_bind_group(0, &camera.bind_group(), &[]);
            for (_, (mesh, object, materials)) in query.iter() {
                rpass.set_bind_group(1, &object.bind_group(), &[]);
                rpass.set_vertex_buffer(0, mesh.vertices().slice(..));
                rpass.set_vertex_buffer(1, mesh.attribute(Attribute::Texcoords0).unwrap().slice(..));
                for (i, index) in mesh.submeshes().iter().enumerate() {
                    rpass.set_bind_group(2, &materials[i].bind_group(), &[]);
                    rpass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
                    rpass.draw_indexed(0..index.count(), 0, 0..1);
                }
            }
        }
        
        queue.submit([encoder.finish()]);
        frame.present();

        Ok(())
    }
}

impl Default for ExampleScene {
    #[inline]
    fn default() -> Self {
        Self { 
            pipeline: None, 
            main_camera: Entity::DANGLING, 
            player: Entity::DANGLING, 
        }
    }
}

impl fmt::Debug for ExampleScene {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(ExampleScene))
    }
}
