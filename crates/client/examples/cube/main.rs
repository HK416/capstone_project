mod model;
use self::model::*;

use std::error::Error;
use std::fmt;
use std::thread;

use hecs::Entity;
use hecs::World;
use mod_render::brush::TextureBrush;
use mod_render::camera::CameraComponent;
use mod_render::camera::CameraDataLayout;
use mod_render::camera::CameraObject;
use mod_render::camera::PerspectiveRh;
use mod_render::camera::Projection;
use mod_render::object::update_hierarchy;
use mod_render::object::GameObjectComponent;
use mod_render::object::GameObjectDataLayout;
use mod_render::object::Transform;
use mod_render::object::WorldTransform;
use mod_render::DepthBuffer;
use mod_scene::GameScene;
use mod_util::AppHandle;
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
    use mod_app::AppBuilder;
    use mod_parallelism::MAIN_THREAD_ID;
    use mod_util::AppDpi;

    assert_eq!(thread::current().id(), *MAIN_THREAD_ID, "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(ExampleScene::default()))
        .with_title("Example: Cube")
        .with_dpi(AppDpi::W1280H720)
        .with_fullscreen(false)
        .build_and_run()
}



/// 예제 장면입니다.
struct ExampleScene {
    main_camera: Entity, 
    cube: Entity, 
}

impl ExampleScene {
    /// 카메라 오브젝트를 생성합니다.
    fn spawn_main_camera(&mut self, window: &Window, world: &mut World, app: &dyn AppHandle) {
        // 로컬 변환 행렬과 월드 변환 행렬을 생성합니다.
        let trans = Transform::from_rotation_translation(
            gmm::Quaternion::from_rotation_x(30f32.to_radians()), 
            gmm::Float3::new(0.0, 1.0, -2.0)
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
            app.render_device()
        );

        // 카메라 오브젝트를 생성합니다.
        self.main_camera = world.spawn((
            trans, 
            world_trans, 
            projection, 
            camera_object, 
        ));
    }

    /// 모델 에셋을 생성합니다.
    fn spawn_cube_model(&mut self, world: &mut World, app: &dyn AppHandle) {
        let (cube, _) = spawn_model_from_asset(
            app.render_device(), 
            app.render_queue(), 
            world, 
            TextureBrush, 
            "Cube.ron"
        );
        self.cube = cube;
    }

    /// 카메라를 준비합니다.
    fn preapre_camera(world: &mut World, app: &dyn AppHandle) {
        type QueryType<'a> = (&'a WorldTransform, &'a Projection, &'a CameraComponent);
        let mut query = world.query::<QueryType>();
        for (_, (world_transform, projection, camera)) in query.iter() {
            let eye = world_transform.get_translation();
            let dir = world_transform.get_forward_vector();
            let up = world_transform.get_up_vector();
            let view_trans = gmm::Matrix::look_to_rh(eye, dir, up);

            camera.update(
                app.render_queue(), 
                CameraDataLayout {
                    proj_view: ((**projection) * view_trans).into(), 
                    position: eye.into(), 
                    direction: dir.into(), 
                    ..Default::default()
                }
            );
        }
    }

    /// 오브젝트를 준비합니다.
    fn prepare_objects(world: &mut World, app: &dyn AppHandle) {
        type QueryType<'a> = (&'a WorldTransform, &'a GameObjectComponent);
        let mut query = world.query::<QueryType>();
        for (_, (world_transform, object)) in query.iter() {
            object.update(
                app.render_queue(), 
                GameObjectDataLayout { 
                    transform: (**world_transform).into() 
                }
            );
        }
    }
}

impl GameScene for ExampleScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        self.spawn_main_camera(window, world, app);
        self.spawn_cube_model(world, app);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        world.clear();
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        let timer = app.timer();
        window.set_title(&format!("Example: Cube (FPS:{})", timer.frame_rate()));

        // 큐브 오브젝트 회전
        type QueryCube<'a> = (&'a mut Transform, &'a mut WorldTransform, &'a GameObjectComponent);
        let rotation = gmm::Matrix::from_rotation_y(30f32.to_radians() * elapsed_time_sec);
        let (
            transform, 
            world_transform, 
            object
        ) = world.query_one_mut::<QueryCube>(self.cube).unwrap();
        (**transform) = (**transform) * rotation;
        (**world_transform) = **transform;

        // // 계층 구조를 갱신합니다.
        update_hierarchy(world, None, self.cube);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_prepare_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        Self::preapre_camera(world, app);
        Self::prepare_objects(world, app);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        let device = app.render_device();
        let queue = app.render_queue();

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
        let camera = match world.get::<&CameraComponent>(self.main_camera) {
            Ok(component) => component, 
            _ => return Ok(()),
        };

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

            rpass.set_bind_group(0, &camera.bind_group(), &[]);
            TextureBrush::draw(world, device, &mut rpass);
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
            main_camera: Entity::DANGLING, 
            cube: Entity::DANGLING, 
        }
    }
}

impl fmt::Debug for ExampleScene {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(ExampleScene))
    }
}
