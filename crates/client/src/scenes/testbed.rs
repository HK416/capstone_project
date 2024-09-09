use std::fmt;
use std::error::Error;
use std::net::TcpStream;
use std::collections::HashMap;
use std::sync::Arc;

use hecs::Entity;
use hecs::World;
use mod_network::Player;
use mod_render::anim::Animation;
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
use mod_render::skin::BoneMatrixDataLayout;
use mod_scene::AppHandle;
use mod_scene::GameScene;
use winit::window::Window;



/// 캐릭터 엔티티의 정보입니다.
struct CharacterEntity {
    id: Entity, 
    animations: Vec<Animation>, 
    anim_index: usize, 
    anim_timer: f32, 
}



/// TestBed Game Scene
pub struct TestBedScene {
    stream: Arc<TcpStream>, 
    stage_data: Vec<Player>, 

    client_id: u32, 
    players: HashMap<u32, CharacterEntity>, 

    main_camera: Entity, 
}

impl TestBedScene {
    #[inline]
    #[must_use]
    pub fn new<I>(
        stream: Arc<TcpStream>, 
        client_id: u32, 
        players: I, 
    ) -> Self 
    where 
    I: IntoIterator<Item = Player>, 
        I::IntoIter: ExactSizeIterator
    {
        let init_data: Vec<_> = players.into_iter().collect();
        let size = init_data.len();
        
        Self { 
            stream, 
            stage_data: init_data, 
            client_id, 
            players: HashMap::with_capacity(size), 
            main_camera: Entity::DANGLING, 
        }
    }


    /// 카메라 오브젝트를 생성합니다.
    fn spawn_main_camera(&mut self, window: &Window, world: &mut World, app: &dyn AppHandle) {
        // 로컬 변환 행렬과 월드 변환 행렬을 생성합니다.
        let trans = Transform::from_rotation_translation(
            gmm::Quaternion::from_rotation_x(15f32.to_radians()), 
            gmm::Float3::new(0.0, 1.5, -2.0)
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
    fn spawn_aris_original_model(
        &mut self, 
        id: u32, 
        translation: gmm::Float3, 
        world: &mut World, 
        app: &dyn AppHandle
    ) {
        // ※ 추후 변경될 함수입니다.
        let (entity, animations) = crate::model::spawn_model_from_asset(
            app.render_device(), 
            app.render_queue(), 
            world, 
            TextureBrush, 
            "Aris_Original.ron"
        );

        // 플레이어 오브젝트 이동
        let translation = gmm::Matrix::from_translation(translation.into());
        let (transform, world_transform) = world.query_one_mut::<(&mut Transform, &mut WorldTransform)>(entity).unwrap();
        (**transform) = (**transform) * translation;
        (**world_transform) = **transform;

        // // 계층 구조를 갱신합니다.
        update_hierarchy(world, None, entity);


        // 플레이어를 추가합니다.
        self.players.insert(
            id, 
            CharacterEntity { 
                id: entity, 
                animations, 
                anim_index: 0, 
                anim_timer: 0.0, 
            }
        );
    }

    /// 플레이어의 애니메이션을 갱신합니다.
    fn update_animation(
        elapsed_time_sec: f32, 
        player: &mut CharacterEntity, 
        world: &mut World, 
        app: &dyn AppHandle
    ) {
        if let Some(animation) = player.animations.get(player.anim_index) {
            player.anim_timer = (player.anim_timer + elapsed_time_sec) % animation.length();
            let keyframe = animation.sample_animation(player.anim_timer);

            for bone in keyframe.bones() {
                for (entity, bone_transform) in bone.target().bones().iter().cloned().zip(bone.transforms()) {
                    if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity) {
                        **transform = bone_transform.as_matrix();
                    }
                }
                
                let root_entity = bone.target().root_bone().clone();
                update_hierarchy(world, None, root_entity);

                let iter = bone.target().bones().iter()
                    .map(|&entity| **world.get::<&WorldTransform>(entity).unwrap())
                    .map(|matrix| matrix.into());
                bone.target().update(app.render_queue(), BoneMatrixDataLayout::new(iter));
            }

            update_hierarchy(world, None, player.id);
        }
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

impl GameScene for TestBedScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        self.spawn_main_camera(window, world, app);
        while let Some(player) = self.stage_data.pop() {
            self.spawn_aris_original_model(
                player.id, 
                gmm::Float3::new(
                    player.x, 
                    player.y, 
                    player.z
                ), 
                world, 
                app
            );
        }
        Ok(())
    }

    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let timer = app.timer();
        window.set_title(&format!("Example: Animation (FPS:{})", timer.frame_rate()));

        for player in self.players.values_mut() {
            Self::update_animation(
                elapsed_time_sec, 
                player, 
                world, 
                app
            );
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_prepare_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        Self::preapre_camera(world, app);
        Self::prepare_objects(world, app);
        Ok(())
    }
    
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

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
                        view: depth_stencil_view, 
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
        Ok(())
    }
}

impl fmt::Debug for TestBedScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestBedScene))
    }
}
