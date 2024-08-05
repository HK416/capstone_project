//! 3차원 큐브를 렌더링하는 예제 애플리케이션입니다.
//! 
use std::fmt;
use std::thread;
use std::sync::Arc;

use hecs::World;
use hecs::Entity;
use winit::keyboard::KeyCode;
use winit::keyboard::KeyLocation;
use winit::window::Window;
use gmm::{Quaternion, Matrix};
use client_framework::app::AppBuilder;
use client_framework::app::Dpi;
use client_framework::app::Handler;
use client_framework::components::Transform;
use client_framework::components::Projection;
use client_framework::components::PerspectiveBuilder;
use client_framework::error::ErrorMessage;
use client_framework::render::scale::RenderScale;
use client_framework::render::bind_group::EntityBindGroup;
use client_framework::render::bind_group::GlobalBindGroup;
use client_framework::render::mesh::shape;
use client_framework::render::material::GraphicsPipeline;
use client_framework::render::material::forward::TexcoordMaterialID;
use client_framework::render::material::forward::TexcoordMaterial;
use client_framework::render::material::forward::TextureMaterial;
use client_framework::render::material::forward::TextureMaterialID;
use client_framework::render::variable::EntityDataLayout;
use client_framework::render::variable::EntityUniform;
use client_framework::render::variable::CameraDataLayout;
use client_framework::render::variable::CameraUniform;
use client_framework::scene::GameScene;



/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점 입니다.
/// 
/// 게임 화면은 16 : 9 비율의 scaled 크기를 가집니다.
/// 
/// `Windows`, `macOS` 플랫폼의 경우 최초 실행시 전체 화면으로 실행됩니다.
/// 
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    use framework::concurrency::MAIN_THREAD_ID;
    assert_eq!(thread::current().id(), *MAIN_THREAD_ID, "Invalid main thread id!");

    // 로그 시스템을 초기화 합니다.
    env_logger::init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(ExampleScene::default()))
        .set_title("Example: Cube")
        .set_fullscreen(false)
        .set_dpi(Dpi::W1280H720)
        .build_and_run()
}



/// 예제 게임 장면 입니다.
pub struct ExampleScene {
    main_camera: Option<Entity>, 
    materials: Vec<Box<dyn GraphicsPipeline>>, 
}

impl Default for ExampleScene {
    #[inline]
    fn default() -> Self {
        Self { 
            main_camera: None, 
            materials: Vec::with_capacity(4), 
        }
    }
}

impl ExampleScene {
    /// 카메라를 생성합니다.
    fn spawn_camera(&mut self, window: &Window, world: &mut World, app: &dyn Handler) -> Result<(), ErrorMessage> {
        // 카메라의 위치를 생성합니다.
        let transform = Transform::from_rotation_translation(
            Quaternion::from_rotation_x(30f32.to_radians()), 
            (0.0, 1.0, -2.0)
        );

        // 카메라의 투영 변환 행렬을 생성합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        let projection = PerspectiveBuilder::new()
            .with_aspect(width as f32 / height as f32)
            .with_fov_y(60f32.to_radians())
            .build();

        // 카메라의 유니폼 버퍼를 생성합니다.
        let uniform = CameraUniform::from_data(
            Some("MainCamera"), 
            app.ref_render_device(), 
            CameraDataLayout::default()
        );

        // 바인드 그룹을 생성합니다.
        let bind_group = GlobalBindGroup::new(
            Some("MainCamera"), 
            app.ref_render_device(), 
            &uniform
        );

        self.main_camera = world.spawn((transform, projection, uniform, bind_group)).into();

        Ok(())
    }

    /// 렌더링 머티리얼을 추가합니다.
    fn register_materials(&mut self, window: &Window, app: &dyn Handler) -> Result<(), ErrorMessage> {
        // 텍스처 좌표 디버깅 머티리얼을 추가합니다.
        let material = Box::new(TexcoordMaterial::new(window, app.ref_render_device()));
        self.materials.push(material);

        // 텍스처 매핑 머티리얼을 추가한다.
        let material = Box::new(TextureMaterial::new(window, app.ref_render_device()));
        self.materials.push(material);

        Ok(())
    }

    /// 큐브 텍스처를 생성합니다.
    fn load_cube_texture(&self, app: &dyn Handler) -> wgpu::TextureView {
        use std::io::Cursor;
        use wgpu::util::DeviceExt;
        use image::io::Reader as ImageReader;

        // 이미지 바이트 배열
        let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/test.png"));

        // 이미지 바이트 배열로부터 이미지를 로드합니다.
        let reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        let device = app.ref_render_device();
        let queue = app.ref_render_queue();

        // 텍스처를 생성합니다.
        device.create_texture_with_data(
            queue, 
            &wgpu::TextureDescriptor {
                label: Some("Texture(Cube)"), 
                size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 }, 
                dimension: wgpu::TextureDimension::D2, 
                format: wgpu::TextureFormat::Rgba8Unorm, 
                mip_level_count: 1, 
                sample_count: 1, 
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, 
                view_formats: &[]
            }, 
            wgpu::util::TextureDataOrder::LayerMajor, 
            &reader.to_rgba8()
        ).create_view(
            &wgpu::TextureViewDescriptor { ..Default::default() }
        )
    }

    /// 큐브 오브젝트를 생성합니다.
    fn spawn_cube_object(&self, world: &mut World, app: &dyn Handler) -> Result<(), ErrorMessage> {
        let diffuse_texture = &self.load_cube_texture(app);

        // 큐브 메쉬를 생성합니다.
        let mesh = Arc::new(shape::create_cube_mesh(
            1.0, 1.0, 1.0, 
            app.ref_render_device(), 
            app.ref_render_queue()
        ));
        
        // 큐브의 위치를 생성합니다.
        let transform = Transform::new();

        // 오브젝트 유니폼 버퍼를 생성합니다.
        let uniform = EntityUniform::from_data(
            Some("Cube"), 
            app.ref_render_device(), 
            EntityDataLayout::default()
        );

        // 바인드 그룹을 생성합니다.
        let sampler = EntityBindGroup::get_default_sampler(app.ref_render_device());
        let ambient_texture = EntityBindGroup::get_default_ambient(app.ref_render_device(), app.ref_render_queue());
        // let diffuse_texture = EntityBindGroup::get_default_diffuse(app.ref_render_device(), app.ref_render_queue());
        let normal_texture = EntityBindGroup::get_default_normal(app.ref_render_device(), app.ref_render_queue());
        let specular_texture = EntityBindGroup::get_default_specular(app.ref_render_device(), app.ref_render_queue());
        let emissive_texture = EntityBindGroup::get_default_emissive(app.ref_render_device(), app.ref_render_queue());

        let bind_group = EntityBindGroup::new(
            Some("Cube"), 
            app.ref_render_device(), 
            &uniform, 
            (ambient_texture, sampler), 
            (diffuse_texture, sampler), 
            (normal_texture, sampler), 
            (specular_texture, sampler), 
            (emissive_texture, sampler)
        );

        world.spawn((TextureMaterialID, mesh, transform, uniform, bind_group));

        Ok(())
    }
}

impl ExampleScene {
    /// 카메라의 유니폼 버퍼를 갱신합니다.
    fn prepare_camera(&self, world: &mut World, app: &dyn Handler) -> Result<(), ErrorMessage> {
        let queue = app.ref_render_queue();

        type QueryType<'a> = (&'a Transform, &'a Projection, &'a Arc<CameraUniform>);
        for (_id, (transform, projection, uniform)) in world.query::<QueryType>().iter() {
            let projection: Matrix = (**projection).into();
            let view = Matrix::look_to_rh(
                transform.get_translation().into(), 
                transform.get_forward_vector().into(), 
                transform.get_up_vector().into()
            );

            queue.write_buffer(&uniform, 0, bytemuck::bytes_of(&CameraDataLayout {
                proj_view: (projection * view).into(), 
                position: transform.get_translation(), 
                direction: transform.get_forward_vector(), 
                ..Default::default()
            }));
        }

        Ok(())
    }

    /// 큐브의 유니폼 버퍼를 갱신합니다.
    fn prepare_cube(&self, world: &mut World, app: &dyn Handler) -> Result<(), ErrorMessage> {
        let queue = app.ref_render_queue();

        type QueryType<'a> = (&'a Transform, &'a Arc<EntityUniform>);
        for (_id, (transform, uniform)) in world.query::<QueryType>().iter() {
            queue.write_buffer(&uniform, 0, bytemuck::bytes_of(&EntityDataLayout {
                trans: **transform, 
                position: transform.get_translation(), 
                ..Default::default()
            }))
        }

        Ok(())
    }

    /// 큐브의 회전 방향을 갱신합니다.
    fn update_cube(&self, world: &mut World, app: &dyn Handler) -> Result<(), ErrorMessage> {
        for (_id, transform) in world.query_mut::<&mut Transform>().with::<&Arc<EntityUniform>>() {
            transform.rotate_y_axis(30f32.to_radians() * app.ref_timer().elapsed_time_sec());
        }

        Ok(())
    }

    /// 렌더 타겟을 주어진 색깔로 초기화 합니다.
    fn clear_render_target(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        render_target_view: &wgpu::TextureView, 
        clear_color: wgpu::Color
    ) {
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { ..Default::default() }
        );

        {
            let _rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(Clear)"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: render_target_view, 
                            resolve_target: None, 
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(clear_color), 
                                store: wgpu::StoreOp::Store, 
                            },
                        }),
                    ],
                    depth_stencil_attachment: None, 
                    timestamp_writes: None, 
                    occlusion_query_set: None
                }
            );
        }

        queue.submit([encoder.finish()]);
    }
}

impl GameScene for ExampleScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        // 파이프라인을 등록합니다.
        self.register_materials(window, app)?;

        // 카메라를 생성합니다.
        self.spawn_camera(window, world, app)?;

        // 큐브 오브젝트를 생성합니다.
        self.spawn_cube_object(world, app)?;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        // 모든 엔티티를 삭제합니다.
        world.clear();
        
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_resized(
        &mut self, 
        window: &Window,
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        for pipeline in self.materials.iter_mut() {
            pipeline.resize_buffer(RenderScale::P100, window, app.ref_render_device());
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_keyboard_pressed(
        &mut self, 
        code: KeyCode, 
        location: KeyLocation, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        if code == KeyCode::Tab {
            let entities: Vec<Entity> = world.iter().map(|e| e.entity()).collect();
            for entity in entities.into_iter() {
                let _ = world.exchange::<(TextureMaterialID, ), (TexcoordMaterialID, )>(entity, (TexcoordMaterialID, ));
            }
        }
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_keyboard_released(
        &mut self, 
        code: KeyCode,
        location: KeyLocation, 
        window: &Window, 
        world: &mut World, 
        app: &dyn Handler
    ) -> Result<(), ErrorMessage> {
        if code == KeyCode::Tab {
            let entities: Vec<Entity> = world.iter().map(|e| e.entity()).collect();
            for entity in entities.into_iter() {
                let _ = world.exchange::<(TexcoordMaterialID, ), (TextureMaterialID, )>(entity, (TextureMaterialID, ));
            }
        }
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
        self.update_cube(world, app)?;

        self.prepare_camera(world, app)?;
        self.prepare_cube(world, app)?;

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

        Self::clear_render_target(
            device, 
            queue, 
            &render_target_view,  
            wgpu::Color {
                r: 0.0, 
                g: 116.0 / 255.0, 
                b: 183.0 / 255.0, 
                a: 1.0, 
            }
        );

        if let Some(camera) = &self.main_camera {
            for pipeline in self.materials.iter() {
                pipeline.process(
                    world, 
                    *camera, 
                    device, 
                    queue, 
                    &render_target_view, 
                );
            }
        }
        
        frame.present();

        Ok(())
    }
}

impl fmt::Debug for ExampleScene {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(ExampleScene))
    }
}
