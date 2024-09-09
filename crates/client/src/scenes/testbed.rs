use core::f32;
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
use mod_render::camera::PerspectiveLh;
use mod_render::camera::Projection;
use mod_render::object::update_hierarchy;
use mod_render::object::GameObjectComponent;
use mod_render::object::GameObjectDataLayout;
use mod_render::object::Transform;
use mod_render::object::WorldTransform;
use mod_render::skin::BoneMatrixDataLayout;
use mod_scene::AppHandle;
use mod_scene::GameScene;
use winit::event::Modifiers;
use winit::keyboard::KeyCode;
use winit::keyboard::KeyLocation;
use winit::window::Window;

const PIXEL_PER_METER: f32 = 1.0 / 0.1;
const FORCE: f32 = 100.0; // 단위: N



/// 캐릭터 엔티티의 정보입니다.
struct CharacterEntity {
    id: Entity, 
    
    animations: Vec<Animation>, 
    anim_index: usize, 
    prev_anim_index: usize, 
    anim_timer: f32, 
    
    rotation: gmm::Float4, 
    translation: gmm::Float3, 
    force: gmm::Float3, 
    velocity: gmm::Float3, 
    inverse_mass: f32, 
    damping: f32, 
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
        let projection: Projection = PerspectiveLh::new()
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
        player: Player, 
        world: &mut World, 
        app: &dyn AppHandle
    ) {
        // ※ 추후 변경될 함수입니다.
        let (entity, animations) = crate::model::spawn_model_from_asset(
            app.render_device(), 
            app.render_queue(), 
            world, 
            TextureBrush, 
            "Aris_Original_Mesh.ron"
        );

        // 플레이어 오브젝트 이동
        let translation = player.translation;
        let rotation = player.rotation;
        let mat = gmm::Matrix::from_rotation_translation(rotation.into(), translation.into());
        let (transform, world_transform) = world.query_one_mut::<(&mut Transform, &mut WorldTransform)>(entity).unwrap();
        (**transform) = (**transform) * mat;
        (**world_transform) = **transform;

        // // 계층 구조를 갱신합니다.
        update_hierarchy(world, None, entity);


        // 플레이어를 추가합니다.
        self.players.insert(
            player.id, 
            CharacterEntity { 
                id: entity, 
                animations, 
                anim_index: 0, 
                prev_anim_index: 0, 
                anim_timer: 0.0, 
                rotation, 
                translation, 
                force: gmm::Float3::ZERO, 
                velocity: gmm::Float3::ZERO, 
                inverse_mass: 1.0 / 43.0, 
                damping: 0.002, 
            }
        );
    }



    /// 플레이어를 갱신합니다.
    fn update_player(
        elapsed_time_sec: f32, 
        player: &mut CharacterEntity, 
        world: &mut World, 
        app: &dyn AppHandle
    ) {
        Self::update_player_animation(elapsed_time_sec, player, world, app);
        Self::update_player_transform(elapsed_time_sec, player, world);
        update_hierarchy(world, None, player.id);
    }

    /// 플레이어의 애니메이션을 갱신합니다.
    fn update_player_animation(
        elapsed_time_sec: f32, 
        player: &mut CharacterEntity, 
        world: &mut World, 
        app: &dyn AppHandle
    ) {
        // 애니메이션을 갱신합니다.
        if player.prev_anim_index != player.anim_index {
            player.anim_timer = 0.0;
        }

        if let Some(animation) = player.animations.get(player.anim_index) {
            (player.anim_index, player.anim_timer) = match player.anim_index {
                2 => {
                    let timer = player.anim_timer + elapsed_time_sec;
                    if timer >= animation.length() {(
                        0, 
                        timer, 
                    )} else {(
                        player.anim_index, 
                        timer
                    )}
                }, 
                _ => (
                    player.anim_index, 
                    (player.anim_timer + elapsed_time_sec) % animation.length()
                )
            };
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
            player.prev_anim_index = player.anim_index;
        }
    }

    /// 플레이어의 위치, 회전량을 갱신합니다.
    fn update_player_transform(
        elapsed_time_sec: f32, 
        player: &mut CharacterEntity, 
        world: &mut World, 
    ) {
        // 위치를 갱신합니다.
        // 0. 질량이 무한대(0.0)인 경우 생략합니다.
        if player.inverse_mass >= f32::EPSILON {
            // 1. 속도를 이용하여 이동 거리를 계산합니다.
            player.translation += player.velocity * elapsed_time_sec;
            
            // 2. 가속도를 구하고, 속도를 갱신합니다.
            let acceleration = player.force * player.inverse_mass;
            player.velocity += acceleration * elapsed_time_sec;

            // 3. 저항을 적용합니다.
            player.velocity *= player.damping.powf(elapsed_time_sec);
            if (player.velocity.x * player.velocity.x
            + player.velocity.y * player.velocity.y
            + player.velocity.z * player.velocity.z)
            <= f32::EPSILON {
                player.velocity = gmm::Float3::ZERO;
            } else {
                let dir: gmm::Vector = player.velocity.into();
                let x_axis: gmm::Vector = gmm::Float3::X.into();
                let norm_dir = dir.vec3_normalize().unwrap();
                let cross: gmm::Float3 = x_axis.vec3_cross(norm_dir).into();
                let dot: gmm::Float3 = x_axis.vec3_dot(norm_dir).into();
                let theta = if cross.y >= 0.0 { 
                    dot.x.acos() 
                } else { 
                    360f32.to_radians() - dot.x.acos() 
                };
                player.rotation = gmm::Quaternion::from_rotation_y(theta).into();
            }

            // 플레이어 오브젝트 이동
            let translation = player.translation * PIXEL_PER_METER;
            let transformation = gmm::Matrix::from_rotation_translation(
                player.rotation.into(), 
                translation.into()
            );
            let (transform, world_transform) = world.query_one_mut::<(&mut Transform, &mut WorldTransform)>(player.id).unwrap();
            (**transform) =  transformation;
            (**world_transform) = **transform;
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
            let view_trans = gmm::Matrix::look_to_lh(eye, dir, up);

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
                player, 
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
            Self::update_player(
                elapsed_time_sec, 
                player, 
                world, 
                app
            );
        }

        Ok(())
    }

    fn on_keyboard_pressed(
        &mut self, 
        keycode: KeyCode, 
        _location: KeyLocation, 
        _modifiers: Modifiers, 
        _window: &Window, 
        _world: &mut World, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let player = self.players.get_mut(&self.client_id).unwrap();

        if keycode == KeyCode::KeyW {
            player.anim_index = 1;
            player.force.z += FORCE;
        } else if keycode == KeyCode::KeyA {
            player.anim_index = 1;
            player.force.x -= FORCE;
        } else if keycode == KeyCode::KeyS {
            player.anim_index = 1;
            player.force.z -= FORCE;
        } else if keycode == KeyCode::KeyD {
            player.anim_index = 1;
            player.force.x += FORCE;
        }

        Ok(())
    }

    fn on_keyboard_released(
        &mut self, 
        keycode: KeyCode, 
        _location: KeyLocation, 
        _modifiers: Modifiers, 
        _window: &Window, 
        _world: &mut World, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let player = self.players.get_mut(&self.client_id).unwrap();

        if keycode == KeyCode::KeyW {
            player.anim_index = 2;
            player.force.z -= FORCE;
        } else if keycode == KeyCode::KeyA {
            player.anim_index = 2;
            player.force.x += FORCE;
        } else if keycode == KeyCode::KeyS {
            player.anim_index = 2;
            player.force.z += FORCE;
        } else if keycode == KeyCode::KeyD {
            player.anim_index = 2;
            player.force.x -= FORCE;
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
