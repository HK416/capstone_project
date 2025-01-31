use std::{error::Error, fmt, sync::Arc};

use ahash::{HashMap, HashSet};
use glam::Vec4Swizzles;
use hecs::{Entity, EntityBuilder, With, Without, World};
use mod_app::{app::AppHandle, asset::AssetManager, net::NetManager, scene::GameScene};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, Bullet, BulletKind, CharacterKind, ClientId, Epoch,
        HealthPoint, LatLon, MovementState, MovementStateTimer, ObjectId, Player, ViewState,
        ViewStateTimer,
    },
    protocol::{Packet, PullStagePacket, PushStatusPacket, RawPacket},
};
use mod_render::{
    CameraResource, SamplerPool, ScreenDescriptor, SkyboxDataLayout, SkyboxResource,
    TextureViewPool, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT,
};
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    component::{
        add_child, animate_character, cleanup, draw_character, spawn_player_character,
        spwan_bullet, update_action_state_by_controller_input_flags, update_action_state_timer,
        update_character_direction, update_entity_hierarchy,
        update_movement_state_by_controller_state, update_movement_state_timer,
        update_third_person_camera_hierarchy, update_view_state_by_controller_input_flags,
        update_view_state_timer, BoneCollection, ControllerInputFlags, ControllerInputTimer,
        ControllerState, MoveDirection, Parent, Projection, SkinningAnimation, TerrainTag,
        ThirdPersonCamera, ToParentTrans, WorldTransform,
    },
    config::UserConfig,
    render::{
        clear_render_target_with_skybox, draw_bullet, draw_damage_particle, draw_terrain,
        get_damage_font, prepare_camera_resource, prepare_mesh_resource, spawn_damage_fx, Damage,
        FxDamageDataLayout, FxDamageResource, LifeTime,
    },
    SERVER_ADDR,
};

/// 기본 게임 구조를 테스트하는 공간입니다.
pub struct TestbedInGameScene {
    /// 사용자 설정 구성 데이터
    user_config: Option<Box<UserConfig>>,
    /// 클라이언트 식별자
    client_id: ClientId,
    /// 플레이어의 오브젝트 식별자
    object_id: ObjectId,
    /// 이전 서버의 Epoch
    epoch: Epoch,

    /// 게임 월드
    world: World,
    /// 게임 월드 엔터티 목록
    entities: HashMap<ObjectId, Entity>,
    /// 메인 카메라 엔터티
    main_camera: Entity,

    /// 플레이어 움직임 방향
    move_direction: MoveDirection,
    /// 사용자 입력 상태
    controller_state: ControllerState,
    /// 사용자 입력 상태 지속 시간
    controller_state_timer: ControllerInputTimer,
    /// 사용자 입력 상태 플래그 변수
    controller_input_flags: ControllerInputFlags,

    /// Skybox 쉐이더 리소스
    skybox_resource: Arc<SkyboxResource>,

    /// 공격을 받아 체력이 깎인 플레이어를 저장
    attack_logs: Vec<(Entity, u32)>,

    // ----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedInGameScene {
    /// 새로운 `TestbedInGameScene`을 생성합니다.
    ///
    /// # Panics
    /// 주어진 클라이언트 식별자가 유효하지 않는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        user_config: Box<UserConfig>,
        client_id: ClientId,
        object_id: ObjectId,
        epoch: Epoch,
        world: World,
        entities: HashMap<ObjectId, Entity>,
        skybox_resource: Arc<SkyboxResource>,
    ) -> Self {
        assert_ne!(client_id, ClientId::NULL, "invalid client id");
        Self {
            user_config: Some(user_config),
            client_id,
            object_id,
            epoch,
            world,
            entities,
            main_camera: Entity::DANGLING,
            move_direction: MoveDirection::default(),
            controller_state: ControllerState::default(),
            controller_state_timer: ControllerInputTimer::default(),
            controller_input_flags: ControllerInputFlags::default(),
            skybox_resource,
            attack_logs: Vec::default(),
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    /// 사용자 인터페이스 콜백 함수
    #[allow(unused_variables)]
    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();
        let frame_rate = app.timer().frame_rate();

        let connect_server_text = egui::RichText::new(format!("FPS:{}", frame_rate))
            .color(egui::Color32::WHITE)
            .background_color(egui::Color32::from_black_alpha(128))
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.label(connect_server_text);
            });

        Ok(())
    }

    /// 사용자 인터페이스를 그릴 준비를 합니다.
    #[allow(unused_variables)]
    fn prepare_ui(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();

        // 윈도우 창 설명자를 생성합니다.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, window: &Window, device: &wgpu::Device) {
        // 애플리케이션 창의 가로와 세로 크기를 가져옵니다.
        let (w, h): (f32, f32) = window.inner_size().into();

        // 로컬 변환 행렬, 월드 변환 행렬, 투영 변환 행렬 컴포넌트를 추가합니다.
        let mut builder = EntityBuilder::new();
        builder.add(ToParentTrans::default());
        builder.add(WorldTransform::default());
        builder.add(Projection::perspective(
            75f32.to_radians(),
            w / h,
            0.01,
            500.0,
        ));

        // 삼인칭 카메라 데이터와 카메라 쉐이더 리소스 컴포넌트를 추가합니다.
        builder.add(ThirdPersonCamera::default());
        builder.add(Arc::new(CameraResource::uninit(Some("main"), device)));

        // 생성된 메인 카메라 엔터티를 저장합니다.
        self.main_camera = self.world.spawn(builder.build());
    }

    /// 메인 카메라를 회전시킵니다.
    fn rotate_main_camera(&mut self, mut dx: f32, mut dy: f32) {
        // 사용자 설정한 마우스 좌/우, 상/하 반전을 적용합니다.
        let offset = 1.0;
        if let Some(config) = &self.user_config {
            if config.mouse.left_right_reversal {
                dx *= -1.0;
            }

            if config.mouse.up_down_reversal {
                dy *= -1.0;
            }
        }

        // 카메라 엔터티에서 삼인칭 카메라 컴포넌트를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&mut ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 카메라를 회전시킵니다.
        third_person_camera.rotate(dx, dy, offset);
    }

    /// 메인 카메라의 오프셋을 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 카메라 상태와 카메라 상태 타이머를 갱신해야합니다.
    ///
    fn update_main_camera_offset(&mut self) {
        // 플레이어 캐릭터의 종류, 카메라 상태, 카메라 상태 타이머를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, &view_state, &view_state_timer) = self
            .world
            .query_one_mut::<(&CharacterKind, &ViewState, &ViewStateTimer)>(entity)
            .expect("invalid entity or invalid entity component");

        // 메인 카메라의 삼인칭 카메라 요소를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&mut ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 삼인칭 카메라의 위치 오프셋을 갱신합니다.
        third_person_camera.update_offset(character_kind, view_state, view_state_timer);
    }

    /// 메인 카메라의 계층 구조를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 카메라의 회전과 위치 오프셋을 갱신해야합니다.
    ///
    fn update_main_camera_hierarchy(&mut self) {
        // 플레이어 캐릭터의 위치를 가져옵니다.
        let entity = self.get_player_entity();
        let world_transform = self
            .world
            .query_one_mut::<&WorldTransform>(entity)
            .expect("invalid entity or invalid entity component");
        let target_position = world_transform.get_translation();

        // 카메라의 계층 구조를 갱신합니다.
        update_third_person_camera_hierarchy(&mut self.world, self.main_camera, target_position);
    }

    /// 메인 카메라 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 메인 카메라의 월드 변환 행렬이 갱신되어야합니다.
    ///
    fn prepare_main_camera_resource(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let camera_entities = [self.main_camera];
        prepare_camera_resource(&self.world, &camera_entities, device, queue);
    }

    /// Skybox 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 메인 카메라의 월드 변환 행렬이 갱신되어야합니다.
    ///
    fn prepare_skybox_resource(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // 메인 카메라 엔터티의 월드 변환 행렬과 투영 변환 행렬을 가져옵니다.
        let (world_transform, projection) = self
            .world
            .query_one_mut::<(&WorldTransform, &Projection)>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        let proj_view = (projection.0 * world_transform.to_view_trans()).to_cols_array();

        // Skybox 쉐이더 리소스를 갱신합니다.
        self.skybox_resource.skybox_uniform.update(
            device,
            queue,
            SkyboxDataLayout {
                proj_view,
                color: [1.0; 3],
                ..Default::default()
            },
        );
    }

    /// 플레이어 움직임 방향을 갱신합니다.
    fn update_player_move_direction(&mut self) {
        // 카메라 엔터티에서 삼인칭 카메라 컴포넌트를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 플레이어 움직임 방향을 갱신합니다.
        self.move_direction
            .update_from_third_person_camera(self.controller_state, third_person_camera);
    }

    /// 컨트롤러 입력 지속 시간을 갱신합니다.
    fn update_player_controller_input_timer(&mut self, fixed_time_sec: f32) {
        match self.controller_state {
            ControllerState::Idle => self
                .controller_state_timer
                .update_when_controller_released(fixed_time_sec),
            _ => self
                .controller_state_timer
                .update_when_controller_preesed(fixed_time_sec),
        }
    }

    /// 플레이어의 움직임 상태를 갱신합니다.
    fn update_player_movement_state(&mut self) {
        // 플레이어 캐릭터 엔터티에서 `MovementState`, `MovementStateTimer`를 가져옵니다.
        let entity = self.get_player_entity();
        let (movement_state, movement_state_timer) = self
            .world
            .query_one_mut::<(&mut MovementState, &mut MovementStateTimer)>(entity)
            .expect("invalid entity or invalid entity component");

        // 움직임 상태를 갱신합니다.
        update_movement_state_by_controller_state(
            movement_state,
            movement_state_timer,
            self.controller_state,
        );
    }

    /// 현재 클라이언트의 플레이어 캐릭터 엔터티를 가져옵니다.
    ///
    /// # Panics
    /// 엔터티 목록에서 오브젝트 식별자에 해당하는 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
    ///
    fn get_player_entity(&self) -> Entity {
        self.entities
            .get(&self.object_id)
            .cloned()
            .expect("no such entity")
    }

    /// 플레이어 카메라 상태를 갱신합니다.
    fn update_player_view_state(&mut self) {
        // 플레이어 캐릭터 엔터티에서 `ViewState`, `ViewStateTimer`를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, view_state, view_state_timer) = self
            .world
            .query_one_mut::<(&CharacterKind, &mut ViewState, &mut ViewStateTimer)>(entity)
            .expect("invalid entity or invalid entity component");

        update_view_state_by_controller_input_flags(
            character_kind,
            view_state,
            view_state_timer,
            self.controller_input_flags,
        );
    }

    /// 플레이어 카메라 상태 타이머를 갱신합니다.
    fn update_player_view_state_timer(&mut self, elapsed_time_sec: f32) {
        // 플레이어 캐릭터 엔터티에서 `ViewState`, `ViewStateTimer`를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, view_state, view_state_timer) = self
            .world
            .query_one_mut::<(&CharacterKind, &mut ViewState, &mut ViewStateTimer)>(entity)
            .expect("invalid entity or invalid entity component");

        // `ViewState`와 `ViewStateTimer`를 갱신합니다.
        update_view_state_timer(
            character_kind,
            view_state,
            view_state_timer,
            elapsed_time_sec,
        );
    }

    /// 플레이어 행동 상태를 갱신합니다.
    fn update_player_action_state(&mut self) {
        // 플레이어 캐릭터 엔터티에서 `CharacterKind`, `ActionState`, `ActionStateTimer`를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, action_state, action_state_timer) = self
            .world
            .query_one_mut::<(&CharacterKind, &mut ActionState, &mut ActionStateTimer)>(entity)
            .expect("invalid entity or invalid entity component");

        // 행동 상태를 갱신합니다.
        update_action_state_by_controller_input_flags(
            character_kind,
            action_state,
            action_state_timer,
            self.controller_input_flags,
        );
    }

    /// 플레이어 행동 상태 지속 시간을 갱신합니다.
    fn update_player_action_state_timer(&mut self, elapsed_time_sec: f32) {
        // 플레이어 캐릭터 엔터티에서 `CharacterKind`, `ActionState`, `ActionStateTimer`를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, action_state, action_state_timer) = self
            .world
            .query_one_mut::<(&CharacterKind, &mut ActionState, &mut ActionStateTimer)>(entity)
            .expect("invalid entity or invalid entity component");

        update_action_state_timer(
            character_kind,
            action_state,
            action_state_timer,
            elapsed_time_sec,
        );
    }

    /// 플레이어 움직임 상태 지속 시간을 갱신합니다.
    fn update_player_movement_state_timer(&mut self, elapsed_time_sec: f32) {
        // 플레이어 캐릭터 엔터티에서 `CharacterKind`, `ActionState`, `ActionStateTimer`를 가져옵니다.
        type Q<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
        );
        let entity = self.get_player_entity();
        let (&character_kind, &action_state, movement_state, movement_state_timer) = self
            .world
            .query_one_mut::<Q>(entity)
            .expect("invalid entity or invalid entity component");

        update_movement_state_timer(
            character_kind,
            action_state,
            movement_state,
            movement_state_timer,
            elapsed_time_sec,
        );
    }

    /// 플레이어 캐릭터 엔터티의 방향을 갱신합니다.
    ///  
    /// # Panics
    /// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    /// - 주어진 엔터티는 요구되는 컴포넌트를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    ///
    fn update_player_character_direction(&mut self) {
        // 메인 카메라 엔터티에서 삼인칭 카메라 데이터를 가져옵니다.
        let mut query = self
            .world
            .query_one::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity");
        let third_person_camera = query.get().expect("invalid entity component");

        // 플레이어 엔터티에서 `MovementState`, `ViewState`, `ViewStateTimer`, `ToParentTrans` 컴포넌트를 가져옵니다.
        type Components<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a mut ToParentTrans,
        );
        let entity = self.get_player_entity();
        let mut query = self
            .world
            .query_one::<Components>(entity)
            .expect("invalid entity");
        let (&character_kind, &action_state, &action_state_timer, &movement_state, local_transform) =
            query.get().expect("invalid entity component");

        // 플레이어 캐릭터의 방향을 갱신합니다.
        update_character_direction(
            character_kind,
            movement_state,
            action_state,
            action_state_timer,
            &self.move_direction,
            third_person_camera,
            local_transform,
        );
    }

    /// 플레이어들의 캐릭터 엔터티를 반환합니다.
    fn get_character_entities(&self) -> Vec<Entity> {
        type R<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a MovementStateTimer,
            &'a ViewState,
            &'a ViewStateTimer,
        );
        let mut query = self.world.query::<With<(), R>>();
        let entities: Vec<_> = query.iter().map(|(entity, _)| entity).collect();
        log::debug!("num players: {}", entities.len());
        entities
    }

    /// 총알 엔터티를 반환합니다.
    fn get_bullet_entities(&self) -> Vec<Entity> {
        let mut query = self.world.query::<Without<&BulletKind, &Parent>>();
        let entities: Vec<_> = query.iter().map(|(entity, _)| entity).collect();
        log::debug!("num bullets: {}", entities.len());
        entities
    }

    /// 데미지 파티클을 생성합니다.
    fn spawn_damage_particles(
        &mut self,
        asset_manager: &AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), Box<dyn Error + Send>> {
        const WIDTH: f32 = 0.05;
        const HEIGHT: f32 = 0.1;
        const ORIGIN: glam::Vec3A = glam::vec3a(-0.1, 0.25, -0.75);

        // 데미지 폰트 텍스처를 가져옵니다.
        let texture = get_damage_font(asset_manager, device, queue)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        let t_font =
            TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
        let s_font = SamplerPool::get_or_init(device, &wgpu::SamplerDescriptor::default());

        while let Some((entity, damage)) = self.attack_logs.pop() {
            // 엔터티의 스키닝 애니메이션 컴포넌트를 가져옵니다.
            let skinning_animation = self
                .world
                .query_one_mut::<&SkinningAnimation>(entity)
                .expect("invalid entity or invalid entity component");
            let parent = skinning_animation.head;

            let s_damage = format!("{}", damage);
            let length = s_damage.trim().len() as f32;
            for (i, ch) in s_damage.trim().chars().enumerate() {
                // 데미지 폰트를 생성합니다.
                let num = ch.to_digit(10).expect("invalid damage type");
                let mut position_v = ORIGIN;
                position_v.x = position_v.x - WIDTH * length * 0.5 + WIDTH * i as f32 + 0.5 * WIDTH;
                let (entity, mut builder, fx_resource) = spawn_damage_fx(
                    device,
                    queue,
                    &t_font,
                    &s_font,
                    &self.world,
                    parent,
                    1.0,
                    WIDTH,
                    HEIGHT,
                    position_v.to_array(),
                    num,
                );
                self.world
                    .insert(entity, builder.build())
                    .expect("no such entity");
            }
        }

        queue.submit([]);

        Ok(())
    }

    /// 데미지 파티클을 갱신합니다.
    fn update_damage_particles(&mut self, elapsed_time_sec: f32) {
        const SPEED: f32 = 0.2;
        let mut retires = Vec::new();
        let query = self.world.query_mut::<(&mut Damage, &mut LifeTime)>();
        for (entity, (damage, life_time)) in query {
            // 위치를 갱신합니다.
            damage.position_v[1] += SPEED * elapsed_time_sec;

            // 라이프 타임을 갱신합니다.
            life_time.0 -= elapsed_time_sec;

            // 라이프 타임을 모두 소진한 경우 `retires`에 추가합니다.
            if life_time.0 < 0.0 {
                retires.push(entity);
            }
        }

        // `retires`에 포함된 엔터티를 제거합니다.
        for entity in retires {
            cleanup(&mut self.world, entity);
        }
    }

    /// 데미지 파티클 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 캐릭터의 월드 변환 행렬이 갱신되어야합니다.
    ///
    fn prepare_damage_particle_resource(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut query = self
            .world
            .query::<(&Parent, &Arc<FxDamageResource>, &Damage, &LifeTime)>();
        for (_, (parent, fx_resource, damage, life_time)) in query.iter() {
            // 부모의 월드 변환 행렬을 가져옵니다.
            let mut query = self
                .world
                .query_one::<&WorldTransform>(parent.0)
                .expect("invalid entity");
            let world_transform = query.get().expect("invalid entity component");

            // 쉐이더 리소스를 갱신합니다.
            fx_resource.uniform_buffer.update(
                device,
                queue,
                FxDamageDataLayout {
                    trans: world_transform.0.to_cols_array(),
                    position_v: damage.position_v,
                    number: damage.number,
                    width: damage.width,
                    height: damage.height,
                    ..Default::default()
                },
            );
        }
    }

    /// 캐릭터 애니메이션을 재생합니다.
    ///
    /// # Note
    /// 엔터티에 요구되는 컴포넌트 목록
    /// - 캐릭터 종류(`CharacterKind`)
    /// - 스키닝 애니메이션(`SkinningAnimation`)
    /// - 행동 상태(`ActionState`)
    /// - 행동 상태 타이머(`ActionStateTimer`)
    /// - 움직임 상태(`MovementState`)
    /// - 움직임 상태 타이머(`MovementStateTimer`)
    ///
    /// # Panics
    /// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    /// - 주어진 엔터티는 요구되는 컴포넌트를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    ///
    fn animate_characters(&mut self, entities: &[Entity], asset_manager: &AssetManager) {
        type Components<'a> = (
            &'a CharacterKind,
            &'a SkinningAnimation,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a MovementStateTimer,
        );

        // 컴포넌트 뷰를 준비합니다.
        let character_view = self.world.view::<Components>();
        let collection_view = self.world.view::<&BoneCollection>();
        let mut transform_view = self.world.view::<&mut ToParentTrans>();

        // 플레이어 캐릭터의 애니메이션을 재생합니다.
        for &entity in entities {
            let (
                &character_kind,
                skinning_animation,
                &action_state,
                &action_state_timer,
                &movement_state,
                &movement_state_timer,
            ) = character_view
                .get(entity)
                .expect("invalid entity or invalid entity component");

            animate_character(
                asset_manager,
                character_kind,
                action_state,
                action_state_timer,
                movement_state,
                movement_state_timer,
                skinning_animation,
                &collection_view,
                &mut transform_view,
            );
        }
    }

    /// 캐릭터 엔터티의 계층 구조를 갱신합니다.
    ///
    /// # Note
    /// 엔터티에 요구되는 컴포넌트 목록
    /// - 로컬 변환 행렬(`ToParentTrans`)
    /// - 월드 변환 행렬(`WorldTransform`)
    ///
    /// # Panics
    /// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    /// - 주어진 엔터티는 요구되는 컴포넌트를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    ///
    fn update_character_hierarchy(&mut self, entities: &[Entity]) {
        // 캐릭터 엔터티의 계층 구조를 갱신합니다.
        for &entity in entities {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 총알 엔터티의 계층 구조를 갱신합니다.
    ///
    /// # Note
    /// 엔터티에 요구되는 컴포넌트 목록
    /// - 로컬 변환 행렬(`ToParentTrans`)
    /// - 월드 변환 행렬(`WorldTransform`)
    ///
    /// # Panics
    /// - 주어진 엔터티는 유효해야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    /// - 주어진 엔터티는 요구되는 컴포넌트를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
    ///
    fn update_bullet_hierarchy(&mut self, entities: &[Entity]) {
        // 총알 엔터티의 계층 구조를 갱신합니다.
        for &entity in entities {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 캐릭터 엔터티의 메쉬 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 월드 변환 행렬이 갱신되어야합니다.
    ///
    fn prepare_character_mesh_resource(
        &mut self,
        entities: &[Entity],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        prepare_mesh_resource(&self.world, entities, device, queue);
    }

    /// 총알의 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 총알의 월드 변환 행렬이 갱신되어야 합니다.
    ///
    fn prepare_bullet_mesh_resource(
        &mut self,
        entities: &[Entity],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        prepare_mesh_resource(&self.world, entities, device, queue);
    }

    /// 지형 엔터티의 계층 구조를 갱신합니다.
    fn update_stage_hierarchy(&mut self) {
        let query = self.world.query_mut::<Without<&TerrainTag, &Parent>>();
        let entities: Vec<_> = query.into_iter().map(|(entity, _)| entity).collect();
        for entity in entities {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 지형 엔터티의 메쉬 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 월드 변환 행렬이 갱신되어야합니다.
    ///
    fn prepare_stage_resource(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let query = self.world.query_mut::<Without<&TerrainTag, &Parent>>();
        let entities: Vec<_> = query.into_iter().map(|(entity, _)| entity).collect();
        prepare_mesh_resource(&self.world, &entities, device, queue);
    }

    /// 게임 서버에 플레이어 데이터를 전송합니다.
    fn push_player_data(&mut self, net_manager: &NetManager) {
        type Components<'a> = (
            &'a WorldTransform,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a MovementStateTimer,
            &'a ViewState,
            &'a ViewStateTimer,
        );

        // 플레이어 엔터티로부터 필요한 컴포넌트 데이터를 가져옵니다.
        let entity = self.get_player_entity();
        let (
            world_transform,
            &action_state,
            &action_state_timer,
            &movement_state,
            &movement_state_timer,
            &view_state,
            &view_state_timer,
        ) = self
            .world
            .query_one_mut::<Components>(entity)
            .expect("invalid entity or invalid entity component");
        let rotation = world_transform.get_rotation().to_array();
        let direction = self.move_direction.0.xyz().to_array();

        // 메인 카메라 엔터티로부터 카메라 방향 데이터를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        let view_rotation: LatLon = LatLon {
            lat: third_person_camera.pitch_angle,
            lon: third_person_camera.yaw_angle,
        };

        // 패킷을 생성하고, 전송합니다.
        let pakcet = PushStatusPacket {
            epoch: self.epoch,
            client_id: self.client_id,
            rotation,
            direction,
            action_state,
            action_state_timer,
            movement_state,
            movement_state_timer,
            view_state,
            view_state_timer,
            view_rotation,
        };
        let socket = net_manager.get(&SERVER_ADDR).expect("no such socket");
        socket.push_packet(pakcet.as_raw());
    }

    /// 서버 데이터를 게임 월드에 반영합니다.
    fn pull_game_world(
        &mut self,
        packet: PullStagePacket,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 현재 게임 월드에 존재하는 오브젝트의 식별자를 수집합니다.
        let mut objects: HashSet<ObjectId> = self.entities.keys().cloned().collect();

        // 게임 월드에 존재하는 플레이어를 갱신합니다.
        let new = self.update_player_from_packet(&packet.players, &mut objects);
        // 새로운 플레이어를 게임 월드에 추가합니다.
        self.add_player_from_packet(
            new,
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        )?;

        // 게임 월드에 존재하는 총알을 갱신합니다.
        let new = self.update_bullet_from_packet(&packet.bullets, &mut objects);
        // 새로운 총알을 게임 월드에 추가합니다.
        self.add_bullet_from_packet(
            new,
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        )?;

        // 제거된 오브젝트를 게임월드에서 제거합니다.
        self.remove_entity_from_packet(objects.into_iter());

        Ok(())
    }

    /// 서버에서 보낸 플레이어 데이터로 갱신합니다.
    ///
    /// 새로운 플레이어 데이터를 반환합니다.
    ///
    fn update_player_from_packet<'a>(
        &mut self,
        players: &'a [Player],
        objects: &mut HashSet<ObjectId>,
    ) -> Vec<&'a Player> {
        // 컴포넌트 뷰를 준비합니다.
        let mut health_point_view = self.world.view::<&mut HealthPoint>();
        let mut action_state_view = self
            .world
            .view::<(&mut ActionState, &mut ActionStateTimer)>();
        let mut movement_state_view = self
            .world
            .view::<(&mut MovementState, &mut MovementStateTimer)>();
        let mut view_state_view = self.world.view::<(&mut ViewState, &mut ViewStateTimer)>();
        let mut local_transform_view = self.world.view::<&mut ToParentTrans>();

        // 새로운 플레이어 데이터를 수집합니다.
        let mut new = Vec::with_capacity(10);

        for player in players {
            // 현재 플레이어의 경우
            if player.object_id == self.object_id {
                objects.remove(&self.object_id);
                let entity = self.get_player_entity();

                // 플레이어 체력이 변동되었는지 확인합니다.
                let hp = health_point_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");

                let diff_t = (hp.0 - player.health_point.0).abs();
                if diff_t > 0.0 {
                    // 변동되었을 경우 로그에 추가합니다.
                    self.attack_logs.push((entity, diff_t as u32));
                }
                *hp = player.health_point;

                // 플레이어 엔터티의 위치를 갱신합니다.
                let local_transform = local_transform_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.set_translation(glam::Vec3::from_array(player.translation));

                continue;
            }

            // 이미 존재했던 오브젝트인 경우 오브젝트의 데이터를 갱신합니다.
            if objects.remove(&player.object_id) {
                // 오브젝트의 엔터티를 가져옵니다.
                let entity = self
                    .entities
                    .get(&player.object_id)
                    .cloned()
                    .expect("no such entity");

                // 플레이어 체력이 변동되었는지 확인합니다.
                let hp = health_point_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");

                let diff_t = (hp.0 - player.health_point.0).abs();
                if diff_t > 0.0 {
                    // 변동되었을 경우 로그에 추가합니다.
                    self.attack_logs.push((entity, diff_t as u32));
                }
                *hp = player.health_point;

                // 행동 상태, 행동 상태 지속 시간을 갱신합니다.
                let (action_state, action_state_timer) = action_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *action_state = player.action_state;
                *action_state_timer = player.action_state_timer;

                // 움직임 상태, 움직임 상태 지속 시간을 갱신합니다.
                let (movement_state, movement_state_timer) = movement_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *movement_state = player.movement_state;
                *movement_state_timer = player.movement_state_timer;

                // 카메라 상태, 카메라 상태 지속 시간을 갱신합니다.
                let (view_state, view_state_timer) = view_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *view_state = player.view_state;
                *view_state_timer = player.view_state_timer;

                // 위치와 방향을 갱신합니다.
                let local_transform = local_transform_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.set_rotation_translation(
                    glam::Quat::from_array(player.rotation),
                    glam::Vec3::from_array(player.translation),
                );
            } else {
                // 존재하지 않은 오브젝트의 경우 새로운 데이터에 추가합니다.
                new.push(player);
            }
        }

        new
    }

    /// 서버에서 보낸 총알 데이터로 갱신합니다.
    ///
    /// 새로운 총알 데이터를 반환합니다.
    ///
    fn update_bullet_from_packet<'a>(
        &mut self,
        bullet: &'a [Bullet],
        objects: &mut HashSet<ObjectId>,
    ) -> Vec<&'a Bullet> {
        // 컴포넌트 뷰를 준비합니다.
        let mut local_transform_view = self.world.view::<&mut ToParentTrans>();

        // 새로운 총알 데이터를 수집합니다.
        let mut new = Vec::with_capacity(128);

        for bullet in bullet {
            // 이미 존재했던 오브젝트인 경우 오브젝트의 데이터를 갱신합니다.
            if objects.remove(&bullet.object_id) {
                // 오브젝트의 엔터티를 가져옵니다.
                let entity = self
                    .entities
                    .get(&bullet.object_id)
                    .cloned()
                    .expect("no such entity");

                // 위치와 방향을 갱신합니다.
                let local_transform = local_transform_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                local_transform.set_rotation_translation(
                    glam::Quat::from_array(bullet.rotation),
                    glam::Vec3::from_array(bullet.translation),
                );
            } else {
                // 존재하지 않은 오브젝트의 경우 새로운 데이터에 추가합니다.
                new.push(bullet);
            }
        }

        new
    }

    /// 서버에서 보낸 플레이어 데이터 중 새로운 플레이어를 게임 월드에 추가합니다.
    fn add_player_from_packet<'a>(
        &mut self,
        new: Vec<&'a Player>,
        asset_manager: &AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 새로운 플레이어를 추가합니다.
        for player in new {
            // 새로운 플레이어 계층 구조를 생성합니다.
            let (root_entity, batch_commands) =
                spawn_player_character(&player, asset_manager, device, queue, &self.world)
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

            // 명령어를 실행합니다.
            for (entity, mut builder) in batch_commands {
                self.world
                    .insert(entity, builder.build())
                    .expect("no such entity");
            }

            // 엔터티 목록에 새로운 엔터티를 추가합니다.
            self.entities.insert(player.object_id, root_entity);
        }

        Ok(())
    }

    /// 서버에서 보낸 총알 데이터 중 새로운 총알을 게임 월드에 추가합니다.
    fn add_bullet_from_packet<'a>(
        &mut self,
        new: Vec<&'a Bullet>,
        asset_manager: &AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 새로운 플레이어를 추가합니다.
        for bullet in new {
            // 새로운 플레이어 계층 구조를 생성합니다.
            let (root_entity, batch_commands) =
                spwan_bullet(bullet, asset_manager, device, queue, &self.world)
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

            // 명령어를 실행합니다.
            for (entity, mut builder) in batch_commands {
                self.world
                    .insert(entity, builder.build())
                    .expect("no such entity");
            }

            // 엔터티 목록에 새로운 엔터티를 추가합니다.
            self.entities.insert(bullet.object_id, root_entity);
        }

        Ok(())
    }

    /// 제거된 엔터티를 게임 월드에서 제거합니다.
    fn remove_entity_from_packet(&mut self, objects: impl Iterator<Item = ObjectId>) {
        // 제거된 엔터티를 엔터티 목록에서 제거합니다.
        for id in objects {
            let entity = self.entities.remove(&id).expect("no such entity");
            cleanup(&mut self.world, entity);
        }
    }
}

impl GameScene for TestbedInGameScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 메인 카메라를 생성합니다.
        self.create_main_camera(window, app.render_device());

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_keyboard_pressed(
        &mut self,
        keycode: KeyCode,
        location: KeyLocation,
        modifiers: Modifiers,
        repeat: bool,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        if !repeat && self.user_config.is_some() {
            // 사용자 입력 상태를 갱신합니다.
            let config = unsafe { self.user_config.as_ref().unwrap_unchecked() };
            self.controller_state
                .handle_keyboard_pressed(config, keycode, location);

            // 사용자 입력 플래그를 갱신합니다.
            self.controller_input_flags
                .handle_keyboard_pressed(config, keycode, location);
        }

        // TODO: 사용자 행동 상태를 갱신합니다.

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_keyboard_released(
        &mut self,
        keycode: KeyCode,
        location: KeyLocation,
        modifiers: Modifiers,
        repeat: bool,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 사용자 입력 상태를 갱신합니다.
        if !repeat && self.user_config.is_some() {
            let config = unsafe { self.user_config.as_ref().unwrap_unchecked() };
            self.controller_state
                .handle_keyboard_released(config, keycode, location);

            // 사용자 입력 플래그를 갱신합니다.
            self.controller_input_flags
                .handle_keyboard_released(config, keycode, location);
        }

        // TODO: 사용자 행동 상태를 갱신합니다.

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_mouse_btn_pressed(
        &mut self,
        x: f32,
        y: f32,
        button: MouseButton,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 사용자 입력 플래그를 갱신합니다.
        if self.user_config.is_some() {
            let config = unsafe { self.user_config.as_ref().unwrap_unchecked() };
            self.controller_input_flags
                .handle_mouse_btn_pressed(config, button);
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_mouse_btn_released(
        &mut self,
        x: f32,
        y: f32,
        button: MouseButton,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 사용자 입력 플래그를 갱신합니다.
        if self.user_config.is_some() {
            let config = unsafe { self.user_config.as_ref().unwrap_unchecked() };
            self.controller_input_flags
                .handle_mouse_btn_released(config, button);
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_cursor_moved(
        &mut self,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 메인 카메라를 회전시킵니다.
        self.rotate_main_camera(dx, dy);

        // TODO: 삼인칭 카메라를 회전시킵니다.
        Ok(())
    }

    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let packet = PullStagePacket::try_from_raw(packet).expect("invalid packet");
        // 패킷이 유효한지 확인합니다.
        // 서버에서 보낸 Epoch가 클라이언트의 Epoch보다 작은 경우
        // 수신된 패킷은 유효하지 않습니다.
        //
        if self.epoch <= packet.epoch {
            self.epoch = packet.epoch;
            self.pull_game_world(packet, app)?;
        }

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_pre_update(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 플레이어 움직임 방향을 갱신합니다.
        self.update_player_move_direction();
        // 플레이어 행동 상태를 갱신합니다.
        self.update_player_action_state();
        // 플레이어 움직임 상태를 갱신합니다.
        self.update_player_movement_state();
        // 플레이어 카메라 상태를 갱신합니다.
        self.update_player_view_state();

        // 데미지 파티클을 생성합니다.
        self.spawn_damage_particles(app.asset_manager(), app.render_device(), app.render_queue())?;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 플레이어 행동 상태 지속 시간을 갱신합니다.
        self.update_player_action_state_timer(elapsed_time_sec);
        // 플레이어 움직임 지속 시간을 갱신합니다.
        self.update_player_movement_state_timer(elapsed_time_sec);
        // 플레이어 카메라 상태 지속 시간을 갱신합니다.
        self.update_player_view_state_timer(elapsed_time_sec);

        // 데미지 파티클을 갱신합니다.
        self.update_damage_particles(elapsed_time_sec);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_fixed_update(
        &mut self,
        fixed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 플레이어 컨트롤러 입력 지속 시간을 갱신합니다.
        self.update_player_controller_input_timer(fixed_time_sec);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_post_update(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 플레이어 움직임 방향을 갱신합니다.
        self.update_player_move_direction();
        // 플레이어 행동 상태를 갱신합니다.
        self.update_player_action_state();
        // 플레이어 움직임 상태를 갱신합니다.
        self.update_player_movement_state();
        // 플레이어 카메라 상태를 갱신합니다.
        self.update_player_view_state();
        // 플레이어 캐릭터의 방향을 갱신합니다.
        self.update_player_character_direction();

        // 플레이어 데이터를 서버에 전송합니다.
        self.push_player_data(app.net_manager());

        Ok(())
    }

    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 캐릭터 엔터티들을 가져옵니다.
        let characters_entities = self.get_character_entities();
        // 캐릭터 애니메이션을 재생합니다.
        self.animate_characters(&characters_entities, app.asset_manager());
        // 캐릭터 엔터티의 계층 구조를 갱신합니다.
        self.update_character_hierarchy(&characters_entities);
        // 캐릭터 엔터티의 메쉬 쉐이더 리소스를 갱신합니다.
        self.prepare_character_mesh_resource(
            &characters_entities,
            app.render_device(),
            app.render_queue(),
        );

        // 총알 엔터티들을 가져옵니다.
        let bullet_entities = self.get_bullet_entities();
        // 총알 엔터티의 계층 구조를 갱신합니다.
        self.update_bullet_hierarchy(&bullet_entities);
        // 총알 엔터티의 메쉬 쉐이더 리소스를 갱신합니다.
        self.prepare_bullet_mesh_resource(
            &bullet_entities,
            app.render_device(),
            app.render_queue(),
        );

        // 데미지 파티클 쉐이더 리소스를 갱신합니다.
        self.prepare_damage_particle_resource(app.render_device(), app.render_queue());

        // 메인 카메라의 위치 오프셋을 갱신합니다.
        self.update_main_camera_offset();
        // 메인 카메라의 계층 구조를 갱신합니다.
        self.update_main_camera_hierarchy();
        // 메인 카메라의 쉐이더 리소스를 갱신합니다.
        self.prepare_main_camera_resource(app.render_device(), app.render_queue());

        // 지형의 계층 구조를 갱신합니다.
        self.update_stage_hierarchy();
        // 지형 메쉬의 쉐이더 리소스를 갱신합니다.
        self.prepare_stage_resource(app.render_device(), app.render_queue());

        // Skybox 쉐이더 리소스를 갱신합니다.
        self.prepare_skybox_resource(app.render_device(), app.render_queue());

        // 사용자 인터페이스를 갱신합니다.
        self.prepare_ui(window, egui_renderer, app)?;

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        //! 검은색 화면에 오른쪽 하단에 상태를 출력합니다.
        //!
        let device = app.render_device();
        let queue = app.render_queue();

        // 캐릭터 엔터티 목록을 가져옵니다.
        let character_entities = self.get_character_entities();

        // 카메라 쉐이더 리소스를 가져옵니다.
        let mut query = self
            .world
            .query_one::<&Arc<CameraResource>>(self.main_camera)
            .expect("invalid entity");
        let camera_resource = query.get().expect("invalid entity component");

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TestbedInGameScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            draw_character(
                &self.world,
                &character_entities,
                camera_resource,
                device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            draw_bullet(
                &self.world,
                &camera_resource,
                &device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            draw_terrain(
                &self.world,
                &camera_resource,
                &device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            clear_render_target_with_skybox(
                &self.skybox_resource,
                &device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            draw_damage_particle(
                &self.world,
                &camera_resource,
                &device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            draw_damage_particle(
                &self.world,
                &camera_resource,
                &device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );
            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl fmt::Debug for TestbedInGameScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestbedInGameScene))
    }
}
