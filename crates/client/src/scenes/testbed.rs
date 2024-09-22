use std::{collections::HashMap, error::Error, fmt, io::{BufWriter, Write}, net::TcpStream, sync::Arc};

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::{PacketType, Player, PullPacket, PushPacket, RawPacket};
use mod_world::{object::{CameraElement, GameObject, PerspectiveLh, Projection}, render::{animation::AnimationClip, camera::CameraDataLayout, mesh::{BoneDataLayout, MeshDataLayout, MeshRenderer}, pipeline::CharacterShader}};
use winit::{event::Modifiers, keyboard::{KeyCode, KeyLocation}, window::Window};


const PIXEL_PER_METER: f32 = 1.0 / 0.1;
const FORCE: f32 = 100.0; // 단위: N



/// 캐릭터 물리 데이터입니다.
#[derive(Debug)]
struct Physics {
    pub rotation: gmm::Float4, 
    pub translation: gmm::Float3, 
    pub force: gmm::Float3, 
    pub velocity: gmm::Float3, 
    pub inverse_mass: f32, 
    pub damping: f32, 
}

/// 캐릭터 애니메이션 데이터입니다.
#[derive(Debug)]
struct Animator {
    anim_index: usize, 
    prev_anim_index: usize, 
    anim_timer: f32, 
}



/// TestBed Game Scene
pub struct TestBedScene {
    stream: Arc<TcpStream>, 
    stage_data: Vec<Player>, 

    client_id: u32, 
    players: HashMap<u32, Arc<GameObject>>, 

    main_camera: Option<Arc<GameObject>>, 
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
            main_camera: None, 
        }
    }



    /// 카메라 오브젝트를 생성합니다.
    fn spawn_main_camera(&mut self, window: &Window, app: &dyn AppHandle) {
        let camera = GameObject::new(None, "Main_Camera");

        // 카메라의 월드 변환 행렬을 생성하고, 설정합니다.
        let world_trans = gmm::Matrix::from_rotation_translation(
            gmm::Quaternion::from_rotation_x(15f32.to_radians()), 
            gmm::Float3::new(0.0, 1.5, -2.0).into()
        );
        camera.set_world_trans(|result| {
            let mut lock_guard = result.unwrap();
            *lock_guard = world_trans;
        });

        // 투영 변환 행렬을 생성하고, 요소에 추가합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        let projection: Projection = PerspectiveLh::new()
            .with_fov_y(60f32.to_radians())
            .with_aspect_ratio(width as f32 / height as f32)
            .into();
        camera.add_element(projection);

        // 카메라에 카메라 요소를 추가합니다.
        camera.add_element(CameraElement::new(Some(camera.name()), app.render_device()));

        // 카메라 오브젝트를 설정합니다.
        self.main_camera = camera.into();
    }

    /// 모델 에셋을 생성합니다.
    fn spawn_aris_original_model(
        &mut self, 
        player: Player, 
        app: &dyn AppHandle
    ) -> Arc<GameObject> {
        // ※ 추후 변경될 함수입니다.
        // 게임 오브젝트를 생성합니다.
        let object = crate::model::spawn_aris_original(
            app.render_device(), 
            app.render_queue(), 
        );

        // 플레이어 오브젝트를 주어진 위치로 이동시킵니다.
        let matrix = gmm::Matrix::from_rotation_translation(
            player.rotation.into(), 
            player.translation.into()
        );
        object.set_world_trans(|result| {
            let mut lock_guard = result.unwrap();
            *lock_guard = (*lock_guard) * matrix;
        });

        // 플레이어 오브젝트에 물리 요소를 추가합니다.
        object.add_element(Physics {
            rotation: player.rotation, 
            translation: player.translation, 
            force: gmm::Float3::ZERO, 
            velocity: gmm::Float3::ZERO, 
            inverse_mass: 1.0 / 43.0, 
            damping: 0.002
        });

        // 플레이어 오브젝트에 애니메이션 요소를 추가합니다.
        object.add_element(Animator {
            anim_index: 0, 
            prev_anim_index: 0, 
            anim_timer: 0.0, 
        });

        object
    }



    /// 플레이어를 갱신합니다.
    fn update_player(
        elapsed_time_sec: f32, 
        player: &Arc<GameObject>, 
        app: &dyn AppHandle
    ) {
        Self::update_player_animation(elapsed_time_sec, player, app);
        Self::update_player_transform(elapsed_time_sec, player);
        player.update_hierarchy(None);
    }

    /// 플레이어의 애니메이션을 갱신합니다.
    fn update_player_animation(
        elapsed_time_sec: f32, 
        player: &Arc<GameObject>,
        app: &dyn AppHandle
    ) {
        let animator = player.get_mut_element::<Animator>().unwrap();
        let animations = player.get_element::<Vec<AnimationClip>>().unwrap();
        
        // 애니메이션을 갱신합니다.
        if animator.prev_anim_index != animator.anim_index {
            animator.anim_timer = 0.0;
        }

        if let Some(animation) = animations.get(animator.anim_index) {
            (animator.anim_index, animator.anim_timer) = match animator.anim_index {
                2 => {
                    let timer = animator.anim_timer + elapsed_time_sec;
                    if timer >= animation.length() {(
                        0, 
                        timer, 
                    )} else {(
                        animator.anim_index, 
                        timer
                    )}
                }, 
                _ => (
                    animator.anim_index, 
                    (animator.anim_timer + elapsed_time_sec) % animation.length()
                )
            };
            let keyframe = animation.sample_animation(animator.anim_timer);

            for skinning in keyframe.meshes() {
                for (object, bone_transform) in skinning.skinned_mesh.bones().iter().zip(skinning.transforms.iter()) {
                    object.set_to_parent_trans(|result| {
                        let mut lock_guard = result.unwrap();
                        *lock_guard = (*bone_transform).into();
                    });
                }
                
                player.update_hierarchy(None);

                let mut iter = skinning.skinned_mesh.bones().iter()
                    .map(|object| object.get_world_trans());
                let mut data = BoneDataLayout::default();
                for dst in data.iter_mut() {
                    *dst = match iter.next() {
                        Some(mat) => mat.into(), 
                        None => break
                    };
                }
                skinning.skinned_mesh.bone_transforms_uniform().update(app.render_queue(), data);
            }
            animator.prev_anim_index = animator.anim_index;
        }
    }

    /// 플레이어의 위치, 회전량을 갱신합니다.
    fn update_player_transform(
        elapsed_time_sec: f32, 
        player: &Arc<GameObject>, 
    ) {
        let physics = player.get_mut_element::<Physics>().unwrap();

        // 위치를 갱신합니다.
        // 0. 질량이 무한대(0.0)인 경우 생략합니다.
        if physics.inverse_mass >= f32::EPSILON {
            // 1. 속도를 이용하여 이동 거리를 계산합니다.
            physics.translation += physics.velocity * elapsed_time_sec;
            
            // 2. 가속도를 구하고, 속도를 갱신합니다.
            let acceleration = physics.force * physics.inverse_mass;
            physics.velocity += acceleration * elapsed_time_sec;

            // 3. 저항을 적용합니다.
            physics.velocity *= physics.damping.powf(elapsed_time_sec);
            if (physics.velocity.x * physics.velocity.x
            + physics.velocity.y * physics.velocity.y
            + physics.velocity.z * physics.velocity.z)
            <= f32::EPSILON {
                physics.velocity = gmm::Float3::ZERO;
            } else {
                let dir: gmm::Vector = physics.velocity.into();
                let x_axis: gmm::Vector = gmm::Float3::X.into();
                let norm_dir = dir.vec3_normalize().unwrap();
                let cross: gmm::Float3 = x_axis.vec3_cross(norm_dir).into();
                let dot: gmm::Float3 = x_axis.vec3_dot(norm_dir).into();
                let theta = if cross.y >= 0.0 { 
                    dot.x.acos() 
                } else { 
                    360f32.to_radians() - dot.x.acos() 
                };
                physics.rotation = gmm::Quaternion::from_rotation_y(theta).into();
            }

            // 플레이어 오브젝트 이동
            let translation = physics.translation * PIXEL_PER_METER;
            let world_trans = gmm::Matrix::from_rotation_translation(
                physics.rotation.into(), 
                translation.into()
            );

            player.set_world_trans(|result| {
                let mut lock_guard = result.unwrap();
                *lock_guard = world_trans;
            });
        }
    }



    /// 카메라를 준비합니다.
    fn preapre_camera(&self, app: &dyn AppHandle) {
        if let Some(camera) = &self.main_camera {
            let world_trans = camera.get_world_trans();
            let world_trans: gmm::Float4x4 = world_trans.into();
            let eye: gmm::Vector = world_trans.w_axis.xyz().into();
            let dir: gmm::Vector = world_trans.z_axis.xyz().into();
            let dir = dir.vec3_normalize().unwrap();
            let up: gmm::Vector = world_trans.y_axis.xyz().into();
            let up = up.vec3_normalize().unwrap();
            let camera_trans = gmm::Matrix::look_to_lh(eye, dir, up);

            let projection = camera.get_element::<Projection>().unwrap();
            let element = camera.get_element::<CameraElement>().unwrap();
            element.camera_uniform().update(app.render_queue(), CameraDataLayout {
                proj_view: ((**projection) * camera_trans).into(), 
                position: eye.into(), 
                direction: dir.into(), 
                ..Default::default()
            });
        }
    }

    /// 오브젝트를 준비합니다.
    fn prepare_objects(object: &Arc<GameObject>, app: &dyn AppHandle) {
        if let Some(mesh_renderer) = object.get_element::<MeshRenderer>() {
            match mesh_renderer {
                MeshRenderer::NonSkinnedMesh(mesh) => {
                    mesh.mesh_uniform()
                        .update(app.render_queue(), MeshDataLayout {
                            trans: object.get_world_trans().into()
                        });
                }, 
                _ => { }
            }
        }

        if let Some(sibling) = object.get_sibling() {
            Self::prepare_objects(&sibling, app);
        }

        if let Some(child) = object.get_child() {
            Self::prepare_objects(&child, app);
        }
    }
}

impl GameScene for TestBedScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        app: &dyn AppHandle
) -> Result<(), Box<dyn Error + Send>> {
        self.spawn_main_camera(window, app);
        while let Some(player) = self.stage_data.pop() {
            let new_player = self.spawn_aris_original_model(player, app);
            self.players.insert(player.id, new_player);
        }
        Ok(())
    }

    fn on_received_packet(
        &mut self, 
        raw_packet: RawPacket, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if raw_packet.packet_type() == PacketType::PULL {
            let packet = PullPacket::from_raw(raw_packet);
            let mut temp = Vec::new();
            for pull_data in packet.world {
                if let Some(player) = self.players.remove(&pull_data.id) {
                    if pull_data.id == self.client_id {
                        temp.push((pull_data.id, player));
                        continue;
                    }

                    let physics = player.get_mut_element::<Physics>().unwrap();
                    physics.rotation = pull_data.rotation;
                    physics.translation = pull_data.translation;

                    let animator = player.get_mut_element::<Animator>().unwrap();
                    animator.anim_index = pull_data.anim_index as usize;
                    animator.anim_timer = pull_data.anim_timer;

                    temp.push((pull_data.id, player));
                } else {
                    let new_player = self.spawn_aris_original_model(pull_data, app);
                    temp.push((pull_data.id, new_player));
                }
            }

            self.players.clear();
            for (id, player) in temp {
                self.players.insert(id, player);
            }
        }
        Ok(())
    }

    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let timer = app.timer();
        window.set_title(&format!("Example: Animation (FPS:{})", timer.frame_rate()));

        for player in self.players.values_mut() {
            Self::update_player(elapsed_time_sec, player, app);
        }

        if let Some(player) = self.players.get(&self.client_id) {
            let physics = player.get_element::<Physics>().unwrap();
            let animator = player.get_element::<Animator>().unwrap();
            let push_data = Player {
                id: self.client_id, 
                translation: physics.translation, 
                rotation: physics.rotation, 
                anim_index: animator.anim_index as u32, 
                anim_timer: animator.anim_timer
            };
            let packet = PushPacket::new(push_data).as_raw();
            let mut writer = BufWriter::new(self.stream.as_ref());
            writer.write_all(&packet.as_bytes())
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        }
        Ok(())
    }

    fn on_keyboard_pressed(
        &mut self, 
        keycode: KeyCode, 
        _location: KeyLocation, 
        _modifiers: Modifiers, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let player = self.players.get_mut(&self.client_id).unwrap();
        let physics = player.get_mut_element::<Physics>().unwrap();
        let animator = player.get_mut_element::<Animator>().unwrap();

        if keycode == KeyCode::KeyW {
            animator.anim_index = 1;
            physics.force.z += FORCE;
        } else if keycode == KeyCode::KeyA {
            animator.anim_index = 1;
            physics.force.x -= FORCE;
        } else if keycode == KeyCode::KeyS {
            animator.anim_index = 1;
            physics.force.z -= FORCE;
        } else if keycode == KeyCode::KeyD {
            animator.anim_index = 1;
            physics.force.x += FORCE;
        }

        Ok(())
    }

    fn on_keyboard_released(
        &mut self, 
        keycode: KeyCode, 
        _location: KeyLocation, 
        _modifiers: Modifiers, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let player = self.players.get_mut(&self.client_id).unwrap();
        let physics = player.get_mut_element::<Physics>().unwrap();
        let animator = player.get_mut_element::<Animator>().unwrap();
        
        if keycode == KeyCode::KeyW {
            animator.anim_index = 2;
            physics.force.z -= FORCE;
        } else if keycode == KeyCode::KeyA {
            animator.anim_index = 2;
            physics.force.x += FORCE;
        } else if keycode == KeyCode::KeyS {
            animator.anim_index = 2;
            physics.force.z += FORCE;
        } else if keycode == KeyCode::KeyD {
            animator.anim_index = 2;
            physics.force.x -= FORCE;
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_prepare_draw(
        &self, 
        window: &Window, 
        surface: &wgpu::Surface, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        self.preapre_camera(app);
        for player in self.players.values() {
            Self::prepare_objects(player, app);
        }
        Ok(())
    }
    
    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        // 명령어 레코더를 생성합니다.
        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { ..Default::default() }
        );

        // 카메라 오브젝트를 가져옵니다.
        let camera = self.main_camera.as_ref().unwrap();
        let camera = camera.get_element::<CameraElement>().unwrap();

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
            ).forget_lifetime();

            rpass.set_bind_group(0, &camera.bind_group(), &[]);
            CharacterShader::get(device).bind(&mut rpass);
            for player in self.players.values() {
                player.draw(&mut rpass);
            }
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
