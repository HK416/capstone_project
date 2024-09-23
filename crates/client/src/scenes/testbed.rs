use std::{collections::HashMap, error::Error, fmt, io::{BufWriter, Write}, net::TcpStream, sync::Arc};

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::{PacketType, Player, PullPacket, PushPacket, RawPacket};
use mod_world::{object::{CameraElement, GameObject, PerspectiveLh, Projection}, render::{animation::AnimationClip, camera::CameraDataLayout, mesh::{BoneDataLayout, MeshDataLayout, MeshRenderer}, pipeline::CharacterShader}};
use winit::{event::Modifiers, keyboard::{KeyCode, KeyLocation}, window::{CursorGrabMode, Window}};


const PIXEL_PER_METER: f32 = 5.0 / 1.0;
const FORCE: f32 = 500.0; // 단위: N
const DISTANCE: f32 = 0.5 * PIXEL_PER_METER;
const POS_OFFSET: gmm::Float3 = gmm::Float3::new(0.0, 0.2 * PIXEL_PER_METER, 0.0);


/// 캐릭터로부터 회전 방향을 나타냅니다.
#[derive(Debug)]
struct Angle {
    polar: f32, 
    azimuthal: f32, 
    dir: gmm::Float3, 
}

/// 캐릭터 물리 데이터입니다.
#[derive(Debug)]
struct Physics {
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
    fn spawn_camera(
        target: Arc<GameObject>, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Arc<GameObject> {
        // 카메라 오브젝트를 생성합니다.
        let camera = GameObject::new(None, "MainCamera");

        // 투영 변환 행렬을 생성하고, 요소에 추가합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        let projection: Projection = PerspectiveLh::new()
            .with_fov_y(60f32.to_radians())
            .with_aspect_ratio(width as f32 / height as f32)
            .into();
        camera.add_element(projection);

        // 카메라에 카메라 요소를 추가합니다.
        camera.add_element(CameraElement::new(Some(camera.name()), app.render_device()));
        camera.add_element(Angle { polar: -15.0, azimuthal: 0.0, dir: gmm::Float3::Z });
        camera.add_element(target);

        // 카메라 오브젝트를 설정합니다.
        camera.into()
    }

    /// 모델 에셋을 생성합니다.
    fn spawn_aris_original_model(
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
        let world_trans = player.get_world_trans();
        let mut rotation: gmm::Quaternion = world_trans.try_into().unwrap();
        let world_trans: gmm::Float4x4 = world_trans.into();
        let mut translation = world_trans.w_axis.xyz();

        // 위치를 갱신합니다.
        // 0. 질량이 무한대(0.0)인 경우 생략합니다.
        if physics.inverse_mass >= f32::EPSILON {
            // 1. 속도를 이용하여 이동 거리를 계산합니다.
            translation += physics.velocity * elapsed_time_sec * PIXEL_PER_METER;
            
            // 2. 가속도를 구하고, 속도를 갱신합니다.
            let acceleration = physics.force * physics.inverse_mass;
            physics.force = gmm::Float3::ZERO;
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
                rotation = gmm::Quaternion::from_rotation_y(theta).into();
            }

            // 플레이어 오브젝트 이동
            let translation = translation;
            let world_trans = gmm::Matrix::from_rotation_translation(
                rotation.into(), 
                translation.into()
            );

            player.set_world_trans(|result| {
                let mut lock_guard = result.unwrap();
                *lock_guard = world_trans;
            });
        }
    }



    fn update_camera(
        camera: &Arc<GameObject>, 
    ) {
        let target = camera.get_element::<Arc<GameObject>>().unwrap();
        let target_trans: gmm::Float4x4 = target.get_world_trans().into();
        let taget_position = target_trans.w_axis.xyz();
        let target_pivot = taget_position + POS_OFFSET;

        let angle = camera.get_mut_element::<Angle>().unwrap();
        let mut axis: gmm::Quaternion = gmm::Float4::new(1.0, 0.0, 0.0, 0.0).into();
        let mut pos: gmm::Quaternion = gmm::Float4::new(0.0, 0.0, -DISTANCE, 0.0).into();
        let y_rotate = gmm::Quaternion::from_rotation_y(angle.azimuthal.to_radians());
        axis = y_rotate * axis * y_rotate.inverse().unwrap();
        pos = y_rotate * pos * y_rotate.inverse().unwrap();

        angle.dir = {
            let pos: gmm::Vector = pos.into();
            let dir = -pos.vec3_normalize().unwrap();
            dir.into()
        };

        let polar_rotate = gmm::Quaternion::from_axis_angle(axis.into(), angle.polar.to_radians());
        pos = polar_rotate * pos * polar_rotate.inverse().unwrap(); 
        let pos: gmm::Vector = pos.into();
        let pos: gmm::Float3 = pos.into();

        let camera_pos = target_pivot + pos;
        let camera_dir: gmm::Vector = (target_pivot - camera_pos).into();
        let camera_dir = camera_dir.vec3_normalize().unwrap();
        let camera_up: gmm::Vector = gmm::Float3::Y.into();
        let camera_right = camera_up.vec3_cross(camera_dir);
        let camera_up = camera_dir.vec3_cross(camera_right);
        
        let camera_transform = gmm::Float4x4::from_columns(
            camera_right.into(), 
            camera_up.into(), 
            camera_dir.into(), 
            gmm::Float4::new(camera_pos.x, camera_pos.y, camera_pos.z, 1.0)
        );

        camera.set_world_trans(|result| {
            let mut lock_guard = result.unwrap();
            *lock_guard = camera_transform.into();
        });
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
        window.set_cursor_visible(false);
        window.set_cursor_position(window.inner_position().unwrap()).unwrap();

        while let Some(player) = self.stage_data.pop() {
            let new_player = Self::spawn_aris_original_model(player, app);
            if player.id == self.client_id {
                self.main_camera = Some(Self::spawn_camera(
                    new_player.clone(), 
                    window, 
                    app
                ));
            }
            self.players.insert(player.id, new_player);
        }
        Ok(())
    }

    fn on_exit(
        &mut self, 
        window: Option<&Window>, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if let Some(window) = window {
            window.set_cursor_visible(true);
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
        }
        Ok(())
    }

    fn on_paused(
        &mut self, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if let Some(window) = app.window() {
            window.set_cursor_visible(true);
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
        }
        Ok(())
    }

    fn on_resumed(
        &mut self, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if let Some(window) = app.window() {
            window.set_cursor_visible(false);
            window.set_cursor_position(window.inner_position().unwrap()).unwrap();
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

                    let world_trans = gmm::Matrix::from_rotation_translation(
                        pull_data.rotation.into(), 
                        pull_data.translation.into()
                    );
                    player.set_world_trans(|result| {
                        let mut lock_guard = result.unwrap();
                        *lock_guard = world_trans;
                    });

                    let animator = player.get_mut_element::<Animator>().unwrap();
                    animator.anim_index = pull_data.anim_index as usize;
                    animator.anim_timer = pull_data.anim_timer;

                    temp.push((pull_data.id, player));
                } else {
                    let new_player = Self::spawn_aris_original_model(pull_data, app);
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

        let camera = self.main_camera.as_ref().unwrap();
        Self::update_camera(camera);

        if let Some(player) = self.players.get(&self.client_id) {
            let animator = player.get_element::<Animator>().unwrap();
            let world_trans = player.get_world_trans();
            let rotation: gmm::Quaternion = world_trans.try_into().unwrap();
            let rotation: gmm::Float4 = rotation.into();
            let world_trans: gmm::Float4x4 = world_trans.into();
            let translation = world_trans.w_axis.xyz();
            let push_data = Player {
                id: self.client_id, 
                translation, 
                rotation, 
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
        _repeat: bool, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let camera = self.main_camera.as_ref().unwrap();
        let angle = camera.get_element::<Angle>().unwrap();
        let camera_dir: gmm::Vector = angle.dir.into();
        let camera_up: gmm::Vector = gmm::Float3::Y.into();
        let camera_right: gmm::Float3 = camera_dir.vec3_cross(camera_up).into();
        let camera_dir = angle.dir;

        let player = self.players.get_mut(&self.client_id).unwrap();
        let physics = player.get_mut_element::<Physics>().unwrap();
        let animator = player.get_mut_element::<Animator>().unwrap();

        if keycode == KeyCode::KeyW {
            animator.anim_index = 1;
            physics.force += camera_dir * FORCE;
        } else if keycode == KeyCode::KeyA {
            animator.anim_index = 1;
            physics.force += camera_right * FORCE;
        } else if keycode == KeyCode::KeyS {
            animator.anim_index = 1;
            physics.force -= camera_dir * FORCE;
        } else if keycode == KeyCode::KeyD {
            animator.anim_index = 1;
            physics.force -= camera_right * FORCE;
        }

        Ok(())
    }

    fn on_keyboard_released(
        &mut self, 
        keycode: KeyCode, 
        _location: KeyLocation, 
        _modifiers: Modifiers, 
        _repeat: bool, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let player = self.players.get_mut(&self.client_id).unwrap();
        let animator = player.get_mut_element::<Animator>().unwrap();
        
        if keycode == KeyCode::KeyW {
            animator.anim_index = 2;
        } else if keycode == KeyCode::KeyA {
            animator.anim_index = 2;
        } else if keycode == KeyCode::KeyS {
            animator.anim_index = 2;
        } else if keycode == KeyCode::KeyD {
            animator.anim_index = 2;
        }

        Ok(())
    }

    fn on_cursor_moved(
        &mut self, 
        x: f32, y: f32, 
        _dx: f32, _dy: f32, 
        window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        window.set_cursor_position(window.inner_position().unwrap()).unwrap();

        let (px, py): (f32, f32) = window.inner_position().unwrap().into();
        let (dx, dy) = (px - x, py - y);

        let camera = self.main_camera.as_ref().unwrap();
        let angle = camera.get_mut_element::<Angle>().unwrap();
        angle.polar = (angle.polar + dy).clamp(-30.0, 30.0);
        angle.azimuthal = (angle.azimuthal + dx) % 360.0;

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
