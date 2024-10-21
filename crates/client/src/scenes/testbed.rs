use std::{collections::HashMap, error::Error, sync::Arc};

use mod_app::{app::AppHandle, asset::AssetBundle, etc::WindowSize, ext::AppWindowExt, net::{IpAddress, NetManager}, scene::GameScene};
use mod_network::{BulletBlob, PacketType, Player, PullPacket, PushPacket, RawPacket, ShotPacket};
use mod_parallelism::collections::{Queue, SkipMap};
use mod_physics::rigid_body::RigidBody;
use mod_world::{component::{player_cursor_moved, player_keyboard_pressed, player_keyboard_released, player_mouse_btn_pressed, player_mouse_btn_released, player_update, AnimationSet, Bullet, BulletKind, GameObject, IdGenerator, InputController, PlayerFlags, PlayerState, Projection, TerrainFactory, Transform, Weapon, WorldID}, render::{camera::{CameraDataLayout, CameraResource, ThirdPersonCamera}, material::MaterialBuilder, mesh::{BoneDataLayout, Mesh, MeshDataLayout}, pipeline::mesh::{terrain::TerrainRenderer, MeshRenderer}}};
use winit::{dpi::PhysicalPosition, event::{Modifiers, MouseButton}, keyboard::{KeyCode, KeyLocation}, window::{CursorGrabMode, Window}};

const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0, 
    g: 116.0 / 255.0, 
    b: 183.0 / 255.0, 
    a: 1.0
};



/// TestBed Game Scene
pub struct TestBedScene {
    address: IpAddress, 
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

    /// 생성된 탄환 목록입니다.
    bullets: HashMap<u32, WorldID>, 

    /// 메쉬 렌더러 오브젝트를 관리합니다.
    renderer: Arc<Queue<Arc<dyn MeshRenderer>>>, 
    render_list: Vec<Arc<dyn MeshRenderer>>, 

    lock_cursor: bool, 

    egui_clip_primitives: Vec<egui::ClippedPrimitive>, 
    egui_free_texture_ids: Vec<egui::TextureId>, 
}

impl TestBedScene {
    /// 새로운 `TestBedScene` 장면을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<I>(
        address: IpAddress, 
        client_id: u32, 
        players: I, 
    ) -> Self 
    where 
        I: IntoIterator<Item = Player>, 
        I::IntoIter: ExactSizeIterator
    {   
        Self { 
            address, 
            stage_data: players.into_iter().collect(), 
            client_id, 
            id_generator: IdGenerator::new(), 
            world: Arc::new(SkipMap::new()), 
            main_camera: WorldID::default(), 
            players: HashMap::with_capacity(10),
            bullets: HashMap::with_capacity(64), 
            renderer: Arc::new(Queue::new()), 
            render_list: Vec::new(), 
            lock_cursor: true, 
            egui_clip_primitives: Vec::new(), 
            egui_free_texture_ids: Vec::new(), 
        }
    }


    fn ui_callback(&mut self, app: &dyn AppHandle) {
        const INFO_0: &'static str = "Escape 버튼을 누르면 마우스 커서를 활성화 할 수 있습니다.";
        const INFO_1: &'static str = "Escape 버튼을 누르면 마우스 커서를 비활성화 할 수 있습니다.";

        let timer = app.timer();
        let head_txt = format!("Hello2Halo (v{}) - FPS:{}", env!("CARGO_PKG_VERSION"), timer.frame_rate());
        let info_txt = if self.lock_cursor { INFO_0 } else { INFO_1 };

        let mut selected_size = app.window_size().clone();

        egui::Window::new("")
            .title_bar(false)
            .auto_sized()
            .movable(false)
            .frame(egui::Frame::none().fill(egui::Color32::from_black_alpha(128)))
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(4.0, 4.0))
            .show(app.egui_ctx(), |ui| {
                let head = egui::RichText::new(head_txt)
                    .color(egui::Color32::LIGHT_GRAY);
                let info = egui::RichText::new(info_txt)
                    .color(egui::Color32::LIGHT_GRAY);
                let size = egui::RichText::new("해상도")
                    .color(egui::Color32::LIGHT_GRAY);

                ui.label(head);
                ui.label(info);

                ui.add_space(8.0);
                ui.label(size);

                egui::ComboBox::from_label("")
                    .selected_text(format!("{}", selected_size.to_string()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected_size, WindowSize::W864H486, "864x486");
                        ui.selectable_value(&mut selected_size, WindowSize::W960H540, "960x540");
                        ui.selectable_value(&mut selected_size, WindowSize::W1024H576, "1024x576");
                        ui.selectable_value(&mut selected_size, WindowSize::W1152H648, "1152x648");
                        ui.selectable_value(&mut selected_size, WindowSize::W1280H720, "1280x720");
                        ui.selectable_value(&mut selected_size, WindowSize::W1366H768, "1366x768");
                        ui.selectable_value(&mut selected_size, WindowSize::W1600H900, "1600x900");
                        ui.selectable_value(&mut selected_size, WindowSize::W1920H1080, "1920x1080");
                    });
            });

        if selected_size != *app.window_size() {
            app.event_loop_proxy().send_event(
                mod_app::etc::AppEvent::ResizeRequest(selected_size)
            ).unwrap();
        }
    }


    /// 지형 오브젝트를 추가합니다.
    fn spwan_terrain(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        // 지형 게임 오브젝트를 생성합니다.
        let mut object = GameObject::new(
            &self.id_generator, 
            "Terrain", 
            None
        );

        // 지형 메쉬를 생성합니다.
        let mesh = TerrainFactory::mesh(
            Some("Terrain"), 
            device, 
            queue, 
            10.0, 
            10.0, 
            1.0
        );

        // 지형 재질을 생성합니다.
        let materials = MaterialBuilder::new("Terrain", device, queue)
            .build(device, queue);

        // 지형 메쉬 렌더러를 생성합니다.
        let renderer = Arc::new(TerrainRenderer::new(
            object.id().clone(), 
            mesh, 
            vec![materials], 
            device
        ));

        // 게임 오브젝트에 메쉬 렌더러를 추가합니다.
        object.insert(renderer.clone());
        self.world.insert(object.id().clone(), object);

        // 렌더러 목록에 메쉬 렌더러를 추가합니다.
        self.renderer.push(renderer);
    }


    /// 플레이어를 추가합니다.
    fn insert_player(
        &mut self, 
        data: Player, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        // 플레이어 게임 오브젝트를 생성합니다.
        let mut object = GameObject::new(
            &self.id_generator, 
            format!("Player({})", &data.id), 
            None
        );

        // 모델 파일을 로드합니다.
        let (root_id, clips, nodes) = crate::model::spawn_aris_original_model(
            &self.world, 
            &self.id_generator, 
            &self.renderer, 
            bundle, 
            device, 
            queue
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

        // 게임 오브젝트에 모델 및 총알 발사 지연시간 정보를 추가합니다.
        object.insert(BulletKind::ArisOriginal);

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

        
        if self.client_id == data.id {
            // 플레이어 오브젝트에 입력 제어기를 추가합니다.
            object.insert(InputController::default());

            // 플레이어 오브젝트에 플래그 변수를 추가합니다.
            object.insert(PlayerFlags::default());

            // 플레이어 오브젝트에 삼인칭 카메라를 추가합니다.
            object.insert(ThirdPersonCamera {
                target: self.main_camera.clone(), 
                distance: -1.0, 
                polar: 180f32.to_radians(), 
                azimuthal: 15f32.to_radians()
            });

            // 플레이어 오브젝트에 무기 요소를 추가합니다.
            object.insert(Weapon {
                muzzle: nodes.get("fire_01").unwrap().clone()
            });
        }


        // 플레이어를 게임 세상에 추가합니다.
        let world_id = object.id().clone();
        self.players.insert(data.id, world_id.clone());
        self.world.insert(object.id().clone(), object);
    }


    /// 게임 오브젝트 계층 구조를 제거합니다.
    fn remove_hierarchy(&self, object_id: &WorldID) {
        if let Some(object) = self.world.remove(object_id) {
            if let Some(sibling_id) = object.get_sibling() {
                self.remove_hierarchy(sibling_id);
            }

            if let Some(child_id) = object.get_child() {
                self.remove_hierarchy(child_id);
            }
        }
    }


    // 총알을 게임 월드에 추가합니다.
    fn insert_bullet(
        &mut self,
        data: BulletBlob, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        // 게임 오브젝트를 생성합니다.
        let mut object = GameObject::new(
            &self.id_generator, 
            format!("Bullet({})", &data.id), 
            None
        );

        // 임시 총알 모델을 로드합니다.
        let root_id = crate::model::spawn_sphere_shape(
            &self.world, 
            &self.id_generator, 
            &self.renderer, 
            bundle, 
            device, 
            queue
        );

        // 모델을 게임 오브젝트의 하위 오브젝트로 추가합니다.
        object.set_child(Some(root_id));

        // 게임 오브젝트의 변환 행렬을 생성합니다.
        let z_axis = gmm::Vector::from(data.direction);
        let y_axis = gmm::Vector::Y;
        let x_axis = y_axis.vec3_cross(z_axis);
        let y_axis = z_axis.vec3_cross(x_axis);
        let scale = gmm::Vector::fill(1.0);
        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let translation = gmm::Vector::from(data.translation);
        object.set_local_transform(Transform(gmm::Matrix::from_scale_rotation_translation(
            scale, 
            rotation, 
            translation
        )));

        // 게임 오브젝트에 탄환 요소를 추가합니다.
        object.insert(Bullet {
            kind: BulletKind::from_id(data.kind), 
            direction: data.direction.into(), 
            translation: data.translation.into(), 
            speed: data.speed, 
            range: data.range, 
        });

        // 게임 월드에 추가합니다.
        let world_id = object.id().clone();
        self.bullets.insert(data.id, world_id.clone());
        self.world.insert(object.id().clone(), object);
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

        // 카메라 오브젝트에 원근 투영 변환 행렬을 추가합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        camera_object.insert(Projection::perspective(
            80f32.to_radians(), 
            width as f32 / height as f32, 
            0.001, 
            1000.0
        ));

        // 카메라 오브젝트에 카메라 요소를 추가합니다.
        camera_object.insert(Arc::new(CameraResource::new(
            Some(camera_object.name()), 
            device
        )));

        // 게임 월드에 카메라 오브젝트를 추가합니다.
        let world_id = camera_object.id().clone();
        self.world.insert(world_id.clone(), camera_object);
        self.main_camera = world_id.clone();
    }


    /// 카메라의 위치를 갱신합니다.
    fn update_camera_pos(&self) {
        // 플레이어 오브젝트의 삼인칭 카메라 요소를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();
        let player = self.world.get(player_id).unwrap();
        let third_person_camera = player.get::<ThirdPersonCamera>().unwrap();
        
        // 카메라 오브젝트의 변위를 계산합니다.
        let polar = gmm::Quaternion::from_rotation_y(third_person_camera.polar);
        let right = polar.transform_vector(gmm::Vector::NEG_X);
        let offset = polar.transform_vector(gmm::Vector::NEG_Z * third_person_camera.distance);

        let azimuthal = gmm::Quaternion::from_axis_angle(right.into(), third_person_camera.azimuthal);
        let offset = azimuthal.transform_vector(offset);

        // 플레이어 오브젝트의 위치를 가져옵니다.
        let position = player.get_world_transform().get_translation();
        let pivot = position + gmm::Vector::Y * 0.85;
        
        // 최종 카메라의 위치를 계산합니다.
        let translation = offset + pivot;
        let x_axis = right.vec3_normalize();
        let z_axis = (pivot - translation).vec3_normalize();
        let y_axis = z_axis.vec3_cross(x_axis);

        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let transform = Transform(gmm::Matrix::from_rotation_translation(rotation, translation));

        // 카메라 오브젝트의 변환 행렬을 설정합니다.
        let mut camera = self.world.get_mut(&self.main_camera).unwrap();
        camera.set_world_transform(transform);
    }


    /// 플레이어 오브젝트를 갱신합니다.
    fn update_player(&self, elapsed_time_sec: f32) {
        for player_id in self.players.values() {
            player_update(&self.world, player_id, elapsed_time_sec).unwrap()
        }
    }


    /// 총알을 생성 및 삭제하고, 총알의 위치를 갱신합니다.
    fn update_bullet(&self, elapsed_time_sec: f32, network: &NetManager) {
        // 플레이어 오브젝트를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();
        let mut player = self.world.get_mut(player_id).unwrap();

        // 플레이어가 총알을 발사한 경우 서버에 생성 메시지를 전달합니다.
        if let Some(bullet) = player.remove::<Bullet>() {
            // 패킷을 생성합니다.
            let packet = ShotPacket::new(BulletBlob::new(
                bullet.kind.into_id(), 
                self.client_id, 
                bullet.translation, 
                bullet.direction, 
                bullet.speed, 
                bullet.range
            )).as_raw();
            
            // 패킷을 서버에 전송합니다.
            let socket = network.get(&self.address).unwrap();
            socket.push_packet(packet);
        }

        self.update_bullet_pos(elapsed_time_sec);
    }

    /// 총알의 위치를 갱신합니다.
    fn update_bullet_pos(&self, elapsed_time_sec: f32) {
        for id in self.bullets.values() {
            // 게임 오브젝트를 가져옵니다.
            let mut object = self.world.get_mut(id).unwrap();

            // 탄환 요소를 가져옵니다.
            let bullet = object.get_mut::<Bullet>().unwrap();

            // 위치를 갱신합니다.
            let distance = bullet.direction * bullet.speed * elapsed_time_sec;
            let translation = bullet.translation + distance;
            bullet.translation = translation;
            
            // 변환 행렬을 갱신합니다.
            let mut transform = object.get_local_transform().clone();
            transform.set_translation(translation);
            object.set_local_transform(transform);

            self.update_object_hierarchy(id, Transform::default());
        }
    }

    fn update_object_hierarchy(&self, id: &WorldID, parent: Transform) {
        // 현재 게임 오브젝트를 가져옵니다.
        let mut object = match self.world.get_mut(id) {
            Some(object) => object, 
            None => return
        };

        // 게임 오브젝트의 월드 변환 행렬을 갱신합니다.
        let local_transform = object.get_local_transform().clone();
        let world_transform = parent * local_transform;
        object.set_world_transform(world_transform);

        // 형제 게임 오브젝트를 갱신합니다.
        if let Some(sibling_id) = object.get_sibling() {
            self.update_object_hierarchy(sibling_id, parent);
        }

        // 자식 게임 오브젝트를 갱신합니다.
        if let Some(child_id) = object.get_child() {
            self.update_object_hierarchy(child_id, world_transform);
        }
    }


    /// 플레이어 데이터를 서버로 전송합니다.
    fn upload_player_data(&self, network: &NetManager) {
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
        
        // 패킷을 생성하고 전송합니다.
        let packet = PushPacket::new(push_data).as_raw();
        let socket = network.get(&self.address).unwrap();
        socket.push_packet(packet);
    }


    /// 게임 월드의 플레이어 오브젝트를 갱신합니다.
    fn pull_player_objects(
        &mut self, 
        players: Vec<Player>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        let mut next = Vec::with_capacity(self.players.len());
        for data in players {
            if let Some(world_id) = self.players.remove(&data.id) {
                if data.id == self.client_id {
                    next.push((data.id, world_id));
                    continue;
                }

                // 플레이어 오브젝트의 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
                let mut object = self.world.get_mut(&world_id).unwrap();
                let mut local_transform = object.get_local_transform().clone();
                local_transform.set_rotation(data.rotation);
                local_transform.set_translation(data.translation);
                object.set_local_transform(local_transform);

                // 플레이어 오브젝트의 애니메이션을 설정합니다.
                let animation = object.get_mut::<AnimationSet>().unwrap();
                animation.index = data.anim_index as usize;
                animation.timer = data.anim_timer;

                next.push((data.id, world_id));
            } else {
                self.insert_player(data, device, queue, bundle);
                next.push((data.id, self.players.remove(&data.id).unwrap()));
            }
        }

        // 제거된 플레이어를 게임 월드에서 삭제합니다.
        for world_id in self.players.values() {
            self.remove_hierarchy(world_id);
        }
        self.players.clear();


        // 남아있는 플레이어 이동.
        while let Some((id, world_id)) = next.pop() {
            self.players.insert(id, world_id);
        }
    }

    /// 게임 월드의 플레이어 오브젝트를 갱신합니다.
    fn pull_bullet_objects(
        &mut self, 
        bullets: Vec<BulletBlob>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        let mut next = Vec::with_capacity(self.players.len());
        for data in bullets {
            if let Some(world_id) = self.bullets.remove(&data.id) {
                // // 총알 오브젝트의 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
                // let mut object = self.world.get_mut(&world_id).unwrap();
                // let transform = object.get_local_transform().clone();
                // transform.set_translation(data.translation);
                // object.set_local_transform(transform);
                // self.update_object_hierarchy(&world_id, Transform::default());

                next.push((data.id, world_id));
            } else {
                self.insert_bullet(data, device, queue, bundle);
                next.push((data.id, self.bullets.remove(&data.id).unwrap()));
            }
        }

        // 제거된 총알을 게임 월드에서 삭제합니다.
        for world_id in self.bullets.values() {
            println!("REMOVE: {:?}", world_id);
            self.remove_hierarchy(world_id);
        }
        self.bullets.clear();


        // 남아있는 총알 이동.
        while let Some((id, world_id)) = next.pop() {
            self.bullets.insert(id, world_id);
        }
    }
}







impl GameScene for TestBedScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 마우스 커서를 비활성화 합니다.
        window.show_cursor(false);
        let (w, h): (u32, u32) = window.inner_size().into();
        window.set_cursor_position(PhysicalPosition::new(w / 2, h / 2)).unwrap();
        window.set_cursor_grab(CursorGrabMode::Confined)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
            .unwrap();

        // 지형을 생성합니다.
        self.spwan_terrain(app.render_device(), app.render_queue(), app.bundle());

        // 메인 카메라를 생성합니다.
        self.create_main_camera(window, app.render_device());

        // 플레이어들을 생성합니다.
        while let Some(data) = self.stage_data.pop() {
            self.insert_player(data, app.render_device(), app.render_queue(), app.bundle());
        }

        Ok(())
    }

    fn on_paused(
        &mut self, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 마우스 커서를 활성화 합니다.
        if let Some(window) = app.window() {
            window.show_cursor(true);
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
        }
        Ok(())
    }

    fn on_resumed(
        &mut self, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if let Some(window) = app.window() {
            if self.lock_cursor {
                // 마우스 커서를 비활성화 합니다.
                window.show_cursor(false);
                let (w, h): (u32, u32) = window.inner_size().into();
                window.set_cursor_position(PhysicalPosition::new(w / 2, h / 2)).unwrap();
                window.set_cursor_grab(CursorGrabMode::Confined)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
                    .unwrap();
            }
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
            self.pull_player_objects(packet.players, app.render_device(), app.render_queue(), app.bundle());
            self.pull_bullet_objects(packet.bullets, app.render_device(), app.render_queue(), app.bundle());
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
        if !self.lock_cursor {
            return Ok(())
        }

        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        // 사용자 입력을 처리합니다.
        player_keyboard_pressed(
            &self.world, 
            player_id, 
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
        window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if !repeat && keycode == KeyCode::Escape {
            self.lock_cursor = !self.lock_cursor;
            if self.lock_cursor {
                // 마우스 커서를 비활성화 합니다.
                window.show_cursor(false);
                let (w, h): (u32, u32) = window.inner_size().into();
                window.set_cursor_position(PhysicalPosition::new(w / 2, h / 2)).unwrap();
                window.set_cursor_grab(CursorGrabMode::Confined)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
                    .unwrap();
            } else {
                window.show_cursor(true);
                window.set_cursor_grab(CursorGrabMode::None).unwrap();
            }
        }

        if !self.lock_cursor {
            return Ok(())
        }

        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        // 사용자 입력을 처리합니다.
        player_keyboard_released(
            &self.world, 
            player_id, 
            keycode, 
            location, 
            modifiers, 
            repeat
        )
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    fn on_mouse_btn_pressed(
        &mut self, 
        x: f32, y: f32, 
        button: MouseButton, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if !self.lock_cursor {
            return Ok(())
        }

        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        player_mouse_btn_pressed(
            &self.world, 
            player_id, 
            x, y, 
            button, 
        )
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    fn on_mouse_btn_released(
        &mut self, 
        x: f32, y: f32, 
        button: MouseButton, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if !self.lock_cursor {
            return Ok(())
        }

        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        player_mouse_btn_released(
            &self.world, 
            player_id, 
            x, y, 
            button, 
        )
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    fn on_cursor_moved(
        &mut self, 
        _x: f32, _y: f32, 
        dx: f32, dy: f32, 
        _window: &Window, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if !self.lock_cursor {
            return Ok(())
        }

        // 유저의 게임 월드 식별자를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();

        // 사용자 입력을 처리합니다.
        player_cursor_moved(
            &self.world, 
            player_id, 
            dx, 
            dy
        )
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
    }

    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        _window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        self.update_camera_pos();
        self.update_player(elapsed_time_sec);
        self.update_bullet(elapsed_time_sec, app.network());

        self.upload_player_data(app.network());

        Ok(())
    }

    fn on_prepare_draw(
        &mut self, 
        window: &Window,
        egui_renderer: &mut egui_wgpu::Renderer, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        // 사용자 인터페이스를 준비합니다.
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: window.inner_size().into(), 
            pixels_per_point: window.scale_factor() as f32, 
        };

        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(app);
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive = egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device, 
            queue, 
            &mut encoder, 
            &egui_primitive, 
            &screen_descriptor
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, &image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;


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

            if let Some(camera) = camera_object.get::<Arc<CameraResource>>() {
                camera.uniform().write(device, queue, CameraDataLayout {
                    proj_view: (projection_matrix * camera_matrix).into(), 
                    position: eye.store_float3().into(), 
                    direction: dir.store_float3().into(), 
                    ..Default::default()
                });
            }
        }

        // 렌더러를 준비합니다.
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

                self.render_list.push(renderer);
            }
        }

        Ok(())
    }

    fn on_draw<'a>(
        &'a self, 
        window: &Window, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        egui_renderer: &egui_wgpu::Renderer,
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();


        // 카메라 오브젝트의 카메라 요소를 가져옵니다.
        let camera = match self.world.get(&self.main_camera) {
            Some(camera) => {
                match camera.get::<Arc<CameraResource>>() {
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

            for renderer in self.render_list.iter() {
                renderer.bind(&camera, &mut rpass);
                renderer.draw(&mut rpass);
            }

            egui_renderer.render(
                &mut rpass.forget_lifetime(), 
                &self.egui_clip_primitives, 
                &egui_wgpu::ScreenDescriptor {
                size_in_pixels: window.inner_size().into(), 
                pixels_per_point: window.scale_factor() as f32, 
            });
        }

        // 그리기 명령(Draw Call)을 명령 대기열에 제출합니다.
        queue.submit([encoder.finish()]);

        Ok(())
    }

    fn on_finish_draw(
        &mut self, 
        _window: &Window, 
        egui_renderer: &mut egui_wgpu::Renderer, 
        _app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        while let Some(renderer) = self.render_list.pop() {
            self.renderer.push(renderer);
        }

        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl std::fmt::Debug for TestBedScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify!(TestBedScene))
    }
}
