use std::{collections::HashMap, error::Error, sync::Arc};

use mod_app::{
    app::AppHandle, 
    asset::AssetBundle, 
    etc::WindowSize, 
    ext::AppWindowExt, 
    net::{IpAddress, NetManager}, 
    scene::GameScene
};
use mod_network::{
    BulletBlob, 
    PacketType, 
    Player, 
    PullPacket, 
    PushPacket, 
    RawPacket, 
    ShotPacket
};
use mod_physics::rigid_body::RigidBody;
use mod_world::{
    component::{
        player_cursor_moved, 
        player_keyboard_pressed, 
        player_keyboard_released, 
        player_mouse_btn_pressed, 
        player_mouse_btn_released, 
        player_update, 
        AnimationSet, 
        Bullet, 
        BulletKind, 
        InputController, 
        PlayerFlags, 
        PlayerState, 
        Projection, 
        TerrainFactory, 
        Weapon
    }, 
    objects::{
        GameObjectDescriptor, 
        GameWorld, 
        ObjectId, 
        Transform
    }, 
    render::{
        brush::terrain::TerrainBrush, 
        camera::{CameraDataLayout, CameraResource, ThirdPersonCamera}, 
        material::universal::{UniversalMaterialDescriptor, UniversalMaterialResource}, 
        mesh::{StaticMeshDataLayout, StaticMeshResource} 
    }, 
    task::DrawTask
};
use winit::{
    dpi::PhysicalPosition, 
    event::{Modifiers, MouseButton}, 
    keyboard::{KeyCode, KeyLocation}, 
    window::{CursorGrabMode, Window}
};

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
    hp: u32,

    /// 게임 오브젝트를 관리하는 게임 세상입니다.
    world: GameWorld, 

    /// 메인 카메라의 게임 오브젝트 식별자입니다.
    main_camera: ObjectId, 

    /// 플레이어 목록입니다.
    players: HashMap<u32, ObjectId>, 

    /// 생성된 탄환 목록입니다.
    bullets: HashMap<u32, ObjectId>, 

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
            hp: 100,
            world: GameWorld::new(), 
            main_camera: ObjectId::NIL, 
            players: HashMap::with_capacity(10),
            bullets: HashMap::with_capacity(64), 
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
        let hp_txt = format!("HP: {}", self.hp);

        let mut selected_size = app.window_size().clone();

        let (width, height): (f32, f32) = app.window_size().size().into();

        egui::Area::new(egui::Id::new("hp_bar"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -16.0))
            .show(app.egui_ctx(), |ui| {
                let hp = egui::RichText::new("HP")
                    .color(egui::Color32::DARK_GRAY);
                let prograss = egui::ProgressBar::new(self.hp as f32 / 100.0)
                    .text(hp)
                    .animate(true)
                    .rounding(egui::Rounding::same(6.0))
                    .desired_width(width * 0.175)
                    .desired_height(height * 0.02)
                    .fill(egui::Color32::LIGHT_GREEN);
                ui.add(prograss);
            });


        egui::Window::new("debug")
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
                let hp = egui::RichText::new(hp_txt)
                    .color(egui::Color32::LIGHT_GRAY);
                let size = egui::RichText::new("해상도")
                    .color(egui::Color32::LIGHT_GRAY);

                ui.label(head);
                ui.label(info);

                ui.add_space(8.0);
                ui.label(hp);

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
        _bundle: &AssetBundle
    ) {
        // 지형 오브젝트를 생성합니다.
        let desc = GameObjectDescriptor::new()
            .with_name("Terrain");
        let id = self.world.spawn(desc);

        // 지형 메쉬를 생성합니다.
        let mesh = Arc::new(TerrainFactory::mesh(
            Some("Terrain"), 
            device, 
            queue, 
            10.0, 
            10.0, 
            1.0
        ));

        let mesh_resource = Arc::new(StaticMeshResource::new(Some("Terrain"), device));

        // 재질을 생성합니다.
        let desc = UniversalMaterialDescriptor::new(device, queue, "Terrain");
        let materials = vec![Arc::new(UniversalMaterialResource::new(device, queue, &desc))];

        // 지형 메쉬 브러쉬를 생성합니다.
        let brush = TerrainBrush::new(device, mesh, mesh_resource.clone(), materials);

        // 그리기 작업을 생성합니다.
        let mesh_resource_cloned = mesh_resource.clone();
        let task = DrawTask::new(id)
            .with_on_pre_draw(Some(Box::new(move |device, queue, world, id| {
                let object = world.get(&id).unwrap();
                mesh_resource_cloned.mesh_uniform().write(device, queue, StaticMeshDataLayout {
                    trans: object.world_transform.into(), 
                    ..Default::default()
                });

                Ok(())
            })))
            .add_brush(brush);

        // 게임 세상에 그리기 작업을 등록합니다.
        self.world.regist_draw_task(task).unwrap();
    }


    /// 플레이어를 추가합니다.
    fn insert_player(
        &mut self, 
        data: Player, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        // 모델 파일을 로드합니다.
        let (root_id, clips, nodes) = crate::model::spawn_aris_original_model(
            &self.world,  
            bundle, 
            device, 
            queue
        );

        // 플레이어 게임 오브젝트를 생성합니다.
        let mut desc = GameObjectDescriptor::new()
            .with_name(format!("Player({})", &data.id))
            .with_child(root_id)
            .with_local_transform(
                gmm::Matrix::from_rotation_translation(
                    data.rotation.into(), 
                    data.translation.into()
                )
            )
            .with_element(BulletKind::ArisOriginal)
            .with_element(PlayerState::default())
            .with_element(AnimationSet { clips, index: 0, timer: 0.0 })
            .with_element({
                let mut rigid_body = RigidBody::new(Some(43.0));
                rigid_body.damping = 0.002;
                rigid_body
            });

        
        if self.client_id == data.id {
            desc = desc.with_element(InputController::default())
                .with_element(PlayerFlags::default())
                .with_element(ThirdPersonCamera {
                    target: self.main_camera, 
                    distance: -1.0, 
                    polar: 180f32.to_radians(), 
                    azimuthal:15f32.to_radians()
                })
                .with_element(Weapon {
                    muzzle: nodes.get("fire_01").unwrap().clone()
                });
        }


        // 플레이어를 게임 세상에 추가합니다.
        let id = self.world.spawn(desc);
        self.players.insert(data.id, id);
    }


    /// 게임 오브젝트 계층 구조를 제거합니다.
    fn remove_hierarchy(&self, object_id: &ObjectId) {
        if let Some(object) = self.world.get(object_id) {
            if !object.sibling.is_nil() {
                self.remove_hierarchy(&object.sibling);
            }

            if !object.child.is_nil() {
                self.remove_hierarchy(&object.child);
            }
        }
        self.world.despawn(object_id);
    }


    // 총알을 게임 월드에 추가합니다.
    fn insert_bullet(
        &mut self,
        data: BulletBlob, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bundle: &AssetBundle
    ) {
        // 임시 총알 모델을 로드합니다.
        let root_id = crate::model::spawn_sphere_shape(
            &self.world, 
            bundle, 
            device, 
            queue
        );

        // 게임 오브젝트의 변환 행렬을 생성합니다.
        let z_axis = gmm::Vector::from(data.direction);
        let y_axis = gmm::Vector::Y;
        let x_axis = y_axis.vec3_cross(z_axis);
        let y_axis = z_axis.vec3_cross(x_axis);
        let scale = gmm::Vector::fill(1.0);
        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let translation = gmm::Vector::from(data.translation);

        let desc = GameObjectDescriptor::new()
            .with_name(format!("Bullet({})", &data.id))
            .with_child(root_id)
            .with_local_transform(gmm::Matrix::from_scale_rotation_translation(
                scale, 
                rotation, 
                translation
            ))
            .with_element(Bullet {
                kind: BulletKind::from_id(data.kind), 
                direction: data.direction.into(), 
                translation: data.translation.into(), 
                speed: data.speed, 
                range: data.range, 
            });

        // 게임 월드에 추가합니다.
        let id = self.world.spawn(desc);
        self.bullets.insert(data.id, id);
    }


    /// 메인 카메라를 생성합니다.
    fn create_main_camera(
        &mut self, 
        window: &Window, 
        device: &wgpu::Device, 
    ) {
        // 카메라 오브젝트를 생성합니다.
        let desc = GameObjectDescriptor::new()
            .with_name("Main_Camera")
            .with_element({
                let (width, height): (u32, u32) = window.inner_size().into();
                Projection::perspective(
                    80f32.to_radians(), 
                    width as f32 / height as f32, 
                    0.001, 
                    1000.0
                )
            })
            .with_element(Arc::new(CameraResource::new(
                Some("Main_Camera"), 
                device
            )));

        // 게임 월드에 카메라 오브젝트를 추가합니다.
        let id = self.world.spawn(desc);
        self.main_camera = id;
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
        let position = player.world_transform.get_translation();
        let pivot = position + gmm::Vector::Y * 0.85;
        
        // 최종 카메라의 위치를 계산합니다.
        let translation = offset + pivot;
        let x_axis = right.vec3_normalize();
        let z_axis = (pivot - translation).vec3_normalize();
        let y_axis = z_axis.vec3_cross(x_axis);

        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let transform = gmm::Matrix::from_rotation_translation(rotation, translation);

        // 카메라 오브젝트의 변환 행렬을 설정합니다.
        let mut camera = self.world.get_mut(&self.main_camera).unwrap();
        camera.world_transform = transform;
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
            let mut transform = object.local_transform;
            transform.set_translation(translation);
            object.local_transform = transform;

            self.update_object_hierarchy(id, gmm::Matrix::IDENTITY);
        }
    }

    fn update_object_hierarchy(&self, id: &ObjectId, parent: gmm::Matrix) {
        // 현재 게임 오브젝트를 가져옵니다.
        let mut object = match self.world.get_mut(id) {
            Some(object) => object, 
            None => return
        };

        // 게임 오브젝트의 월드 변환 행렬을 갱신합니다.
        let local_transform = object.local_transform;
        let world_transform = parent * local_transform;
        object.world_transform = world_transform;

        // 형제 게임 오브젝트를 갱신합니다.
        if !object.sibling.is_nil() {
            self.update_object_hierarchy(&object.sibling, parent);
        }

        // 자식 게임 오브젝트를 갱신합니다.
        if !object.child.is_nil() {
            self.update_object_hierarchy(&object.child, world_transform);
        }
    }


    /// 플레이어 데이터를 서버로 전송합니다.
    fn upload_player_data(&self, network: &NetManager) {
        // 플레이어 오브젝트를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();
        let player = self.world.get(player_id).unwrap();
        let player_transform = player.world_transform;
        let animation = player.get::<AnimationSet>().unwrap();

        // 업로드 데이터를 생성합니다.
        let push_data = Player {
            id: self.client_id, 
            hp: self.hp,
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
        app: &dyn AppHandle
    ) {
        let mut next = Vec::with_capacity(self.players.len());
        for data in players {
            if let Some(world_id) = self.players.remove(&data.id) {
                if data.id == self.client_id {
                    self.hp = data.hp;
                    next.push((data.id, world_id));
                    continue;
                }

                // 플레이어 오브젝트의 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
                let mut object = self.world.get_mut(&world_id).unwrap();
                let mut local_transform = object.local_transform;
                local_transform.set_rotation(data.rotation);
                local_transform.set_translation(data.translation);
                object.local_transform = local_transform;

                // 플레이어 오브젝트의 애니메이션을 설정합니다.
                let animation = object.get_mut::<AnimationSet>().unwrap();
                animation.index = data.anim_index as usize;
                animation.timer = data.anim_timer;

                next.push((data.id, world_id));
            } else {
                let device = app.render_device().clone();
                let queue = app.render_queue().clone();
                let bundle = app.bundle().clone();
                rayon::scope(|_| {
                    self.insert_player(data, &device, &queue, &bundle);
                });
    
                // self.insert_player(data, device, queue, bundle);
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
        app: &dyn AppHandle
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
                self.insert_bullet(data, app.render_device(), app.render_queue(), app.bundle());
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
            let device = app.render_device().clone();
            let queue = app.render_queue().clone();
            let bundle = app.bundle().clone();
            rayon::scope(|_| {
                self.insert_player(data, &device, &queue, &bundle);
            });

            // self.insert_player(data, app.render_device(), app.render_queue(), app.bundle());
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
            self.pull_player_objects(packet.players, app);
            self.pull_bullet_objects(packet.bullets, app);
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
            let world_transform = camera_object.world_transform;
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

        // 그리기 작업 목록을 준비합니다.
        for guard in self.world.draw_tasks() {
            guard.on_pre_draw(device, queue, &self.world)?;
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


        let brushes: Vec<_> = self.world.draw_tasks()
            .map(|guard| {
                let brushes: Vec<_> = guard.brushes().into_iter().cloned().collect();
                brushes
            })
            .flatten()
            .collect();

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

            for brush in brushes.iter() {
                brush.bind(&camera, &mut rpass);
                brush.draw(&mut rpass);
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
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        for guard in self.world.draw_tasks() {
            guard.on_post_draw(device, queue, &self.world)?;
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
