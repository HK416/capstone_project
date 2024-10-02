use std::{collections::HashMap, error::Error, io::{BufWriter, Write}, net::TcpStream, sync::Arc};

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::{PacketType, Player, PullPacket, PushPacket, RawPacket};
use mod_parallelism::collections::{Queue, SkipMap};
use mod_physics::rigid_body::RigidBody;
use mod_world::{component::{player_keyboard_pressed, player_keyboard_released, player_update, AnimationSet, Camera, Direction, GameObject, IdGenerator, InputController, PlayerState, Projection, ThirdPersonCamera, Transform, WorldID}, render::{camera::CameraDataLayout, mesh::{BoneDataLayout, Mesh, MeshDataLayout}, pipeline::mesh::MeshRenderer}};
use winit::{event::Modifiers, keyboard::{KeyCode, KeyLocation}, window::Window};

const FORCE: f32 = 500.0;
const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0, 
    g: 116.0 / 255.0, 
    b: 183.0 / 255.0, 
    a: 1.0
};



/// TestBed Game Scene
pub struct TestBedScene {
    stream: Arc<TcpStream>, 
    stage_data: Vec<Player>, 

    client_id: u32, 

    /// 게임 오브젝트 식별자 생성기입니다.
    id_generator: Arc<IdGenerator>, 

    /// 게임 오브젝트를 관리하는 풀 객체입니다.
    world: Arc<SkipMap<WorldID, GameObject>>, 

    /// 메인 카메라의 게임 오브젝트 식별자입니다.
    main_camera: WorldID, 

    /// 플레이어 목록입니다.
    players: HashMap<u32, WorldID>, 

    /// 메쉬 렌더러 오브젝트를 관리합니다.
    renderer: Arc<Queue<Arc<dyn MeshRenderer>>>, 

    /// 사용자의 입력입니다.
    direction: Direction, 

    /// 사용자 입력기입니다.
    controller: InputController, 
}

impl TestBedScene {
    /// 새로운 `TestBedScene` 장면을 생성합니다.
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
        Self { 
            stream, 
            stage_data: players.into_iter().collect(), 
            client_id, 
            id_generator: IdGenerator::new(), 
            world: Arc::new(SkipMap::new()), 
            main_camera: WorldID::default(), 
            players: HashMap::with_capacity(10),
            renderer: Arc::new(Queue::new()), 
            direction: Direction::default(), 
            controller: InputController::default(), 
        }
    }

    /// 플레이어를 추가합니다.
    fn insert_player(
        &mut self, 
        data: Player, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) {
        // 플레이어 게임 오브젝트를 생성합니다.
        let mut object = GameObject::new(
            &self.id_generator, 
            format!("Player({})", &data.id), 
            None
        );

        // 모델 파일을 로드합니다.
        let (root_id, clips) = crate::model::spawn_aris_original_model(
            &self.world, 
            &self.renderer, 
            &self.id_generator, 
            &device, 
            &queue
        );

        // 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
        object.set_local_transform(
            gmm::Matrix::from_rotation_translation(
                data.rotation.into(), 
                data.translation.into()
            )
        );

        // 하위 오브젝트를 설정합니다.
        object.set_child(Some(root_id));

        // 게임 오브젝트에 상태를 추가합니다.
        object.insert(PlayerState::default());

        // 게임 오브젝트에 애니메이션을 추가합니다.
        object.insert(AnimationSet {
            clips, 
            index: 0, 
            timer: 0.0
        });

        // 게임 오브젝트에 강체 물리 요소를 추가합니다.
        let mut rigid_body = RigidBody::new(Some(43.0));
        rigid_body.damping = 0.002;
        object.insert(rigid_body);

        // 플레이어를 게임 세상에 추가합니다.
        let world_id = object.id().clone();
        self.players.insert(data.id, world_id.clone());
        self.world.insert(object.id().clone(), object);
    }


    /// 플레이어를 제거합니다.
    fn remove_player(&self, player_id: &WorldID) {
        if let Some(object) = self.world.remove(player_id) {
            if let Some(sibling_id) = object.get_sibling() {
                self.remove_player(sibling_id);
            }

            if let Some(child_id) = object.get_child() {
                self.remove_player(child_id);
            }
        }
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(
        &mut self, 
        window: &Window, 
        device: &wgpu::Device, 
    ) {
        // 카메라 오브젝트를 생성합니다.
        let mut camera_object = GameObject::new(
            &self.id_generator, 
            "Main_Camera".to_string(), 
            None
        );

        // 카메라의 월드 변환 행렬을 설정합니다.
        // let transform = Transform(
        //     gmm::Matrix::from_rotation_translation(
        //         gmm::Quaternion::from_rotation_x(10f32.to_radians()), 
        //         gmm::Vector::new(0.0, 1.25, -2.0, 0.0)
        //     )
        // );
        // camera_object.set_local_transform(transform);

        // 카메라 오브젝트에 원근 투영 변환 행렬을 추가합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        camera_object.insert(Projection::perspective(
            45f32.to_radians(), 
            width as f32 / height as f32, 
            0.001, 
            1000.0
        ));

        // 카메라 오브젝트에 카메라 요소를 추가합니다.
        camera_object.insert(Arc::new(Camera::new(
            Some(camera_object.name()), 
            device
        )));

        // 카메라 오브젝트에 대상 오브젝트 요소를 추가합니다.
        let world_id = self.players.get(&self.client_id).unwrap();
        camera_object.insert(ThirdPersonCamera { 
            target: world_id.clone(), 
            distance: -2.5, 
            polar: 180f32.to_radians(), 
            azimuthal: 10f32.to_radians()
        });

        // 게임 월드에 카메라 오브젝트를 추가합니다.
        let world_id = camera_object.id().clone();
        self.world.insert(world_id.clone(), camera_object);
        self.main_camera = world_id.clone();
    }


    /// 카메라의 위치를 갱신합니다.
    fn update_camera_pos(&self) {
        // 카메라 오브젝트의 삼인칭 카메라 요소를 가져옵니다.
        let camera = self.world.get(&self.main_camera).unwrap();
        let third_person = camera.get::<ThirdPersonCamera>().unwrap();
        
        // 카메라 오브젝트의 변위를 계산합니다.
        let polar = gmm::Quaternion::from_rotation_y(third_person.polar);
        let right: gmm::Float4 = (polar * gmm::Quaternion::new(-1.0, 0.0, 0.0, 0.0) * polar.inverse()).into();
        let offset = polar * gmm::Quaternion::new(0.0, 0.0, -third_person.distance, 0.0) * polar.inverse();

        let azimuthal = gmm::Quaternion::from_axis_angle(right.into(), third_person.azimuthal);
        let offset: gmm::Float4 = (azimuthal * offset * azimuthal.inverse()).into();
        let offset = gmm::Vector::load_float3(offset.xyz());

        // 대상 오브젝트의 위치를 가져옵니다.
        let target_id = third_person.target.clone();
        let object = self.world.get(&target_id).unwrap();
        let pivot = object.get_world_transform().get_translation() + gmm::Vector::new(0.0, 1.0, 0.0, 0.0);
        
        // 최종 카메라의 위치를 계산합니다.
        let translation = offset + pivot;
        let x_axis = gmm::Vector::load_float3(right.xyz());
        let z_axis = (pivot - translation).vec3_normalize();
        let y_axis = z_axis.vec3_cross(x_axis);

        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let transform = Transform(gmm::Matrix::from_rotation_translation(rotation, translation));

        // 카메라 오브젝트의 변환 행렬을 설정합니다.
        let mut camera = self.world.get_mut(&self.main_camera).unwrap();
        camera.set_world_transform(transform);
    }


    /// 플레이어 힘의 총량을 계산합니다.
    fn update_player_force(&self) {
        // 카메라 오브젝트의 월드 변환 행렬을 가져옵니다.
        let camera = self.world.get(&self.main_camera).unwrap();
        let camera_transform = camera.get_world_transform().clone();

        // 플레이어의 힘의 총량을 계산합니다.
        let mut right = camera_transform.get_right_vector();
        right.set_y(0.0);

        let mut look = camera_transform.get_look_vector();
        look.set_y(0.0);

        let vector = self.direction.get_vector();
        let mut force_accum = gmm::Vector::ZERO;
        force_accum += FORCE * vector.get_x() * right;
        force_accum += FORCE * vector.get_z() * look;

        // 플레이어 오브젝트를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();
        let mut player = self.world.get_mut(player_id).unwrap();
        
        // 플레이어 힘의 총량을 설정합니다.
        let rigid_body = player.get_mut::<RigidBody>().unwrap();
        rigid_body.force_accum = force_accum;

        // 속도의 크기가 f32::EPSILON보다 클 경우 플레이어 방향을 설정합니다.
        let velocity = rigid_body.velocity.clone();
        if velocity.vec3_len() > f32::EPSILON {
            let z_axis = velocity.vec3_normalize();
            let x_axis = gmm::Vector::X;
            let cross = x_axis.vec3_cross(z_axis);
            let dot = x_axis.vec3_dot_into(z_axis);
            let theta = match cross.get_y() >= 0.0 {
                true => dot.acos(), 
                false => 360f32.to_radians() - dot.acos()
            } + 90f32.to_radians();

            let mut transform = player.get_local_transform().clone();
            transform.set_rotation(gmm::Quaternion::from_rotation_y(theta));
            player.set_local_transform(transform);
        }
    }


    /// 플레이어 데이터를 서버로 전송합니다.
    fn upload_player_data(&self) {
        // 플레이어 오브젝트를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();
        let player = self.world.get(player_id).unwrap();
        let player_transform = player.get_world_transform().clone();
        let animation = player.get::<AnimationSet>().unwrap();

        // 업로드 데이터를 생성합니다.
        let push_data = Player {
            id: self.client_id, 
            translation: player_transform.get_translation().into(), 
            rotation: player_transform.get_rotation().into(), 
            anim_index: animation.index as u32, 
            anim_timer: animation.timer
        };
        
        // 패킷을 생성합니다.
        let packet = PushPacket::new(push_data).as_raw();
        let mut writer = BufWriter::new(self.stream.as_ref());
        writer.write_all(&packet.as_bytes()).unwrap();
    }
}







impl GameScene for TestBedScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 마우스 커서를 비활성화 합니다.
        window.set_cursor_visible(false);
        window.set_cursor_position(window.inner_position().unwrap()).unwrap();

        // 플레이어들을 생성합니다.
        while let Some(data) = self.stage_data.pop() {
            self.insert_player(data, app.render_device(), app.render_queue());
        }

        // 메인 카메라를 생성합니다.
        self.create_main_camera(window, app.render_device());

        Ok(())
    }

    fn on_paused(
        &mut self, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 마우스 커서를 활성화 합니다.
        if let Some(window) = app.window() {
            window.set_cursor_visible(true);
        }
        Ok(())
    }

    fn on_resumed(
        &mut self, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 마우스 커서를 비활성화 합니다.
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
                if let Some(world_id) = self.players.remove(&pull_data.id) {
                    if pull_data.id == self.client_id {
                        temp.push((pull_data.id, world_id));
                        continue;
                    }

                    let mut player = self.world.get_mut(&world_id).unwrap();
                    let mut transform = player.get_local_transform().clone();
                    transform.set_translation(pull_data.translation);
                    transform.set_rotation(pull_data.rotation);
                    player.set_local_transform(transform);

                    let animation = player.get_mut::<AnimationSet>().unwrap();
                    animation.index = pull_data.anim_index as usize;
                    animation.timer = pull_data.anim_timer;
                    temp.push((pull_data.id, world_id));
                } else {
                    self.insert_player(pull_data, app.render_device(), app.render_queue());
                    temp.push((pull_data.id, self.players.remove(&pull_data.id).unwrap()));
                }
            }

            for player_id in self.players.values() {
                self.remove_player(&player_id);
            }
            self.players.clear();
            
            for (id, player) in temp {
                self.players.insert(id, player);
            }
        }

        Ok(())
    }

    fn on_keyboard_pressed(
        &mut self, 
        keycode: KeyCode, 
        location: KeyLocation, 
        modifiers: Modifiers, 
        repeat: bool, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        // 사용자 입력을 처리합니다.
        player_keyboard_pressed(
            &self.world, 
            player_id, 
            &self.controller, 
            &mut self.direction, 
            keycode, 
            location, 
            modifiers, 
            repeat
        )
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    fn on_keyboard_released(
        &mut self, 
        keycode: KeyCode, 
        location: KeyLocation, 
        modifiers: Modifiers, 
        repeat: bool, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        // 사용자 입력을 처리합니다.
        player_keyboard_released(
            &self.world, 
            player_id, 
            &self.controller, 
            &mut self.direction, 
            keycode, 
            location, 
            modifiers, 
            repeat
        )
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    fn on_cursor_moved(
        &mut self, 
        x: f32, y: f32, 
        _dx: f32, _dy: f32, 
        window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 커서의 위치를 이동시킵니다.
        window.set_cursor_position(window.inner_position().unwrap()).unwrap();

        // 커서의 이동량을 계산합니다.
        let (px, py): (f32, f32) = window.inner_position().unwrap().into();
        let delta_x = px - x;
        let delta_y = py - y;

        // 카메라 오브젝트의 삼인칭 카메라 요소를 가져옵니다.
        let mut camera = self.world.get_mut(&self.main_camera).unwrap();
        let third_person = camera.get_mut::<ThirdPersonCamera>().unwrap();
        third_person.polar = (third_person.polar + delta_x.to_radians()) % 360f32.to_radians();
        third_person.azimuthal = (third_person.azimuthal + delta_y.to_radians()).clamp(0f32.to_radians(), 30f32.to_radians());

        Ok(())
    }

    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let frame_rate = app.timer().frame_rate();
        window.set_title(&format!("Hello to Halo! (FPS: {})", frame_rate));

        self.update_player_force();
        self.update_camera_pos();

        // 플레이어 오브젝트를 갱신합니다.
        for player_id in self.players.values() {
            player_update(&self.world, player_id, elapsed_time_sec)
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        }

        self.upload_player_data();

        Ok(())
    }

    fn on_prepare_draw(
        &self, 
        _window: &Window, 
        _surface: &wgpu::Surface, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        // 카메라를 준비합니다.
        if let Some(camera_object) = self.world.get(&self.main_camera) {
            let world_transform = camera_object.get_world_transform();
            let eye = world_transform.get_translation();
            let dir = world_transform.get_look_vector();
            let up = world_transform.get_up_vector();
            let camera_matrix = gmm::Matrix::look_to_lh(eye, dir, up);
            let projection_matrix = match camera_object.get::<Projection>() {
                Some(projection) => projection.0, 
                None => gmm::Matrix::IDENTITY
            };

            if let Some(camera) = camera_object.get::<Arc<Camera>>() {
                camera.camera_uniform().update(device, queue, CameraDataLayout {
                    proj_view: (projection_matrix * camera_matrix).store_float4x4(), 
                    position: eye.store_float3(), 
                    direction: dir.store_float3(), 
                    ..Default::default()
                });
            }
        }

        // 렌더러를 준비합니다.
        let mut render_list = Vec::new();
        while let Some(renderer) = self.renderer.pop() {
            if self.world.contains_key(renderer.game_object()) {
                match renderer.mesh() {
                    Mesh::NonSkinnedMesh(mesh) => {
                        let world_transform = self.world.get(renderer.game_object())
                            .unwrap()
                            .get_world_transform()
                            .clone();

                        mesh.mesh_uniform().update(device, queue, MeshDataLayout {
                            trans: world_transform.0.store_float4x4()
                        });
                    }, 
                    Mesh::SkinnedMesh(mesh) => {
                        let mut data = BoneDataLayout::default();
                        for (index, id) in mesh.bones().iter().enumerate() {
                            let bone_transform = self.world.get(id)
                                .expect("스키닝된 메쉬를 구성하는 게임 오브젝트를 찾을 수 없습니다!")
                                .get_world_transform()
                                .clone();
                            data[index] = bone_transform.0.store_float4x4();
                        }

                        mesh.bone_transforms_uniform().update(device, queue, data);
                    }
                }

                render_list.push(renderer);
            }
        }


        while let Some(renderer) = render_list.pop() {
            self.renderer.push(renderer);
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


        // 카메라 오브젝트의 카메라 요소를 가져옵니다.
        let camera = match self.world.get(&self.main_camera) {
            Some(camera) => {
                match camera.get::<Arc<Camera>>() {
                    Some(camera) => camera.clone(), 
                    None => {
                        log::warn!("카메라 오브젝트에 카메라 요소가 없습니다!");
                        return Ok(())
                    }
                }
            }, 
            None => {
                log::warn!("카메라가 게임 월드에 존재하지 않습니다!");
                return Ok(())
            }
        };


        // 렌더러를 준비합니다.
        let mut render_list = Vec::new();
        while let Some(renderer) = self.renderer.pop() {
            if self.world.contains_key(renderer.game_object()) {
                render_list.push(renderer);
            }
        }


        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );

        {
            let mut rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(TestBedScene)"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment { 
                            view: render_target_view, 
                            resolve_target: None, 
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(BACKGROUND_COLOR), 
                                store: wgpu::StoreOp::Store
                            } 
                        }), 
                    ], 
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { 
                        view: depth_stencil_view, 
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), 
                            store: wgpu::StoreOp::Store
                        }), 
                        stencil_ops: None 
                    }), 
                    timestamp_writes: None, 
                    occlusion_query_set: None
                }
            );

            for renderer in render_list.iter() {
                renderer.bind(&camera, &mut rpass);
                renderer.draw(&mut rpass);
            }
        }

        // 그리기 명령(Draw Call)을 명령 대기열에 제출합니다.
        queue.submit([encoder.finish()]);


        while let Some(renderer) = render_list.pop() {
            self.renderer.push(renderer);
        }

        Ok(())
    }
}

impl std::fmt::Debug for TestBedScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify!(TestBedScene))
    }
}
