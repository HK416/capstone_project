use std::sync::Arc;

use ahash::{HashMap, HashSet};
use hecs::{Entity, EntityBuilder, With, Without, World};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    net::{NetManager, NetworkError},
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, Bullet, BulletKind, CharacterKind, DamageLog, GameInputBits,
        HealthPoint, LatLon, LoginToken, MaxHealthPoint, MovementState, MovementStateTimer,
        ObjectId, PlayPhasePlayer, UserId, ViewState, ViewStateTimer,
    },
    protocol::{
        Packet, PacketType, PullStagePacket, PushStatusPacket, RawPacket, UdpDamageLogPacket,
    },
};
use mod_physics::object3d::Frustum;
use mod_render::{DEPTH_FORMAT, SWAPCHAIN_FORMAT,};
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        ModelPool, SamplerPool, TextureDataPool, TexturePool, TextureViewPool, NOTOSANS_REGULAR,
        UI_GAME_LAYOUT_URI,
    },
    component::{
        animate_character, draw_stage, prepare_mesh_resource,
        prepare_skinned_mesh_resource, set_weapon_position, spawn_player_character, spwan_bullet,
        update_character_direction, update_entity_hierarchy, update_third_person_camera,
        update_third_person_camera_hierarchy, update_view_state_by_controller_input_flags,
        update_view_state_timer, BoneCollection, Child, MoveDirection, Parent,
        Projection, Sibling, SkinningAnimation, StageTag, ThirdPersonCamera, ToParentTrans,
        WorldTransform,
    },
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
    SERVER_TCP_ADDR,
};

/// 기본 게임 구조를 테스트하는 공간입니다.
pub struct InGameDominationModeScene {
    /// 애플리케이션 표시 언어
    #[allow(dead_code)]
    locale: Locale,
    /// 현재 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 모델 풀 객체입니다.
    model_pool: ModelPool,
    /// 텍스처 데이터 풀 객체입니다.
    texture_data_pool: TextureDataPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,

    /// 게임 월드
    world: World,
    /// 플레이어 엔터티 목록
    players: HashMap<UserId, Entity>,
    /// 오브젝트 엔터티 목록
    objects: HashMap<ObjectId, Entity>,
    /// 메인 카메라 엔터티
    main_camera: Entity,

    /// 전역 조명 데이터
    directional_light: DirectionLight,

    /// 플레이어 움직임 방향
    move_direction: MoveDirection,
    /// 사용자 입력 상태 플래그 변수
    controller_input_flags: GameInputBits,

    /// Skybox 쉐이더 리소스
    skybox_resource: Arc<SkyboxResource>,

    /// 공격을 받아 체력이 깎인 플레이어를 저장
    damage_logs: Vec<DamageLog>,

    /// ----- Shadow Pass -----
    shadow_resource: Option<ShadowMapResource>,
    /// -----  Composite Pass -----
    composite_resource: Option<CompositeResource>,

    /// 게임 인터페이스 레이아웃 텍스처 식별자입니다.
    ui_game_layout_texture: egui::load::SizedTexture,
}

impl InGameDominationModeScene {
    /// 새로운 `InGameDominationModeScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        model_pool: ModelPool,
        texture_data_pool: TextureDataPool,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
        world: World,
        players: HashMap<UserId, Entity>,
        skybox_resource: Arc<SkyboxResource>,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            model_pool,
            texture_data_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
            world,
            players,
            objects: HashMap::default(),
            main_camera: Entity::DANGLING,
            directional_light: DirectionLight {
                direction: glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    50f32.to_radians(),
                    -30f32.to_radians(),
                    0.0,
                ),
                color: [1.0, 1.0, 1.0],
            },
            move_direction: MoveDirection::default(),
            controller_input_flags: GameInputBits::default(),
            skybox_resource,
            damage_logs: Vec::default(),
            shadow_resource: None,
            composite_resource: None,
            ui_game_layout_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
        }
    }

    /// UI에 사용되는 텍스처를 등록합니다.
    fn register_ui_game_layout_texture(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 게임 인터페이스 레이아웃 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(UI_GAME_LAYOUT_URI)
            .expect("UI_Game_Layout texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let mut egui_renderer = app.egui_renderer_mut();
        let texture_id = egui_renderer.register_native_texture(
            app.render_device(),
            &texture,
            wgpu::FilterMode::Linear,
        );

        self.ui_game_layout_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, window: &Window, device: &wgpu::Device) {
        // 플레이어 캐릭터 종류를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, &view_rotation) = self
            .world
            .query_one_mut::<(&CharacterKind, &LatLon)>(entity)
            .expect("invalid entity or invalid entity component");

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
        builder.add(ThirdPersonCamera::new(character_kind, view_rotation));
        builder.add(Arc::new(CameraResource::uninit(Some("main"), device)));
        builder.add(Frustum::from_mat4(glam::Mat4::IDENTITY));

        // 생성된 메인 카메라 엔터티를 저장합니다.
        self.main_camera = self.world.spawn(builder.build());
    }

    /// 메인 카메라를 회전시킵니다.
    fn rotate_main_camera(&mut self, mut dx: f32, mut dy: f32) {
        // 사용자 설정한 마우스 좌/우, 상/하 반전을 적용합니다.
        let offset = 1.0;

        let config = UserConfig::get();
        if config.flip_horizontal {
            dx *= -1.0;
        }

        if config.flip_vertical {
            dy *= -1.0;
        }

        // 카메라 엔터티에서 카메라 방향 컴포넌트를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&mut ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 카메라를 회전시킵니다.
        if cfg!(not(feature = "print-transform")) {
            third_person_camera.rotate(dx, dy, offset);
        }

        // 플레이어 엔터티에 카메라 방향 컴포넌트에도 적용합니다.
        let rotation = third_person_camera.rotation;
        let entity = self.get_player_entity();
        let view_rotation = self
            .world
            .query_one_mut::<&mut LatLon>(entity)
            .expect("invalid entity or invalid entity component");
        *view_rotation = rotation;
    }

    /// 메인 카메라의 오프셋을 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 카메라 상태와 카메라 상태 타이머를 갱신해야합니다.
    ///
    fn update_main_camera_offset(&mut self) {
        // 플레이어 캐릭터의 종류, 카메라 상태, 카메라 상태 타이머를 가져옵니다.
        let entity = self.get_player_entity();
        let (&character_kind, &action_state, &action_state_timer, &view_state, &view_state_timer) =
            self.world
                .query_one_mut::<(
                    &CharacterKind,
                    &ActionState,
                    &ActionStateTimer,
                    &ViewState,
                    &ViewStateTimer,
                )>(entity)
                .expect("invalid entity or invalid entity component");

        // 메인 카메라의 삼인칭 카메라 요소를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&mut ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 삼인칭 카메라를 갱신합니다.
        update_third_person_camera(
            third_person_camera,
            character_kind,
            action_state,
            action_state_timer,
            view_state,
            view_state_timer,
        );
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

        // 투영 변환 행렬을 갱신합니다.
        for entity in camera_entities {
            let (third_person_camera, projection) = self
                .world
                .query_one_mut::<(&ThirdPersonCamera, &mut Projection)>(entity)
                .expect("invalid entity or invalid entity component");
            projection.0 =
                glam::Mat4::perspective_lh(third_person_camera.fov_y, 16.0 / 9.0, 0.01, 500.0);
        }

        prepare_camera_resource(&self.world, &camera_entities, device, queue);
    }

    /// 그림자 쉐이더 리소스를 갱신합니다.
    fn prepare_shadow_resource(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        // 메인 카메라의 위치를 가져옵니다.
        let transform = self
            .world
            .query_one_mut::<&WorldTransform>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        let camera_pos = transform.get_translation();
        let camera_dir = transform.get_look_vector();

        // 전역 조명의 방향을 가져옵니다.
        let light_dir = self.directional_light.get_look_vector();

        // 그림자 쉐이더 리소스의 변환 행렬을 계산합니다.
        let center = camera_pos + camera_dir * 5.0;
        let eye = center - light_dir * 25.0;
        let view = glam::Mat4::look_at_lh(eye.into(), center.into(), glam::Vec3::Y);
        let proj = glam::Mat4::orthographic_lh(-7.5, 7.5, -7.5, 7.5, -10.0, 50.0);

        // 전역 조명을 갱신합니다.
        GlobalLightUniform::get_or_uninit(device).update(
            device,
            queue,
            GlobalLightDataLayout {
                light_space: (proj * view).to_cols_array(),
                direction_w: light_dir.to_array(),
                color: self.directional_light.color,
                ..Default::default()
            },
        );
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
        let controller_state = self.controller_input_flags.as_state();
        self.move_direction
            .update_from_third_person_camera(controller_state, third_person_camera);
    }

    /// 현재 클라이언트의 플레이어 캐릭터 엔터티를 가져옵니다.
    ///
    /// # Panics
    /// 엔터티 목록에서 오브젝트 식별자에 해당하는 엔터티를 찾을 수 없는 경우 [`panic!`]을 호출합니다.
    ///
    fn get_player_entity(&self) -> Entity {
        self.players
            .get(&self.user_id)
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
    ) {
        const WIDTH: f32 = 0.05;
        const HEIGHT: f32 = 0.1;
        const ORIGIN: glam::Vec3A = glam::vec3a(-0.1, 0.25, -0.75);

        // 데미지 폰트 텍스처를 가져옵니다.
        let texture =
            get_damage_font(asset_manager, device, queue).expect("font texture must exist!");
        let t_font = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
        let s_font = self
            .sampler_pool
            .get_or_init(device, &wgpu::SamplerDescriptor::default());

        while let Some(log) = self.damage_logs.pop() {
            // 엔터티를 가져옵니다.
            let entity = match self.players.get(&log.user_id) {
                Some(&entity) => entity,
                None => continue,
            };

            // 엔터티의 스키닝 애니메이션 컴포넌트를 가져옵니다.
            let skinning_animation = self
                .world
                .query_one_mut::<&SkinningAnimation>(entity)
                .expect("invalid entity or invalid entity component");
            let parent = skinning_animation.head;

            let s_damage = format!("{}", log.damage.0);
            let length = s_damage.trim().len() as f32;
            for (i, ch) in s_damage.trim().chars().enumerate() {
                // 데미지 폰트를 생성합니다.
                let num = ch.to_digit(10).expect("invalid damage type");
                let mut position_v = ORIGIN;
                position_v.x = position_v.x - WIDTH * length * 0.5 + WIDTH * i as f32 + 0.5 * WIDTH;
                let (entity, mut builder) = spawn_damage_fx(
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
            .query::<(&Parent, &Arc<FxDamageResource>, &Damage)>();
        for (_, (parent, fx_resource, damage)) in query.iter() {
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
            &'a LatLon,
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
                &view_rotation,
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
                view_rotation,
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

        let query_view = self
            .world
            .view::<(&CharacterKind, &ActionState, &SkinningAnimation)>();
        let child_view = self.world.view::<&Child>();
        let sibling_view = self.world.view::<&Sibling>();
        let mut transform_view = self.world.view::<(&ToParentTrans, &mut WorldTransform)>();
        for &entity in entities {
            let (&character_kind, &action_state, skinning_animation) = query_view
                .get(entity)
                .expect("invalid entity or invalid entity component");

            set_weapon_position(
                character_kind,
                action_state,
                &skinning_animation,
                &child_view,
                &sibling_view,
                &mut transform_view,
            );
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
    fn prepare_character_mesh_resource(&mut self, entities: &[Entity], device: &wgpu::Device) {
        prepare_skinned_mesh_resource(&self.world, entities, device, 32);
    }

    /// 총알의 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 총알의 월드 변환 행렬이 갱신되어야 합니다.
    ///
    fn prepare_bullet_mesh_resource(&mut self, entities: &[Entity], device: &wgpu::Device) {
        prepare_mesh_resource(&self.world, entities, device, 32);
    }

    /// 스테이지 엔터티의 계층 구조를 갱신합니다.
    fn update_stage_hierarchy(&mut self) {
        // 스테이지 지역 엔터티와 소품 엔터티를 수집합니다.
        let entities: Vec<_> = self
            .world
            .query_mut::<Without<&StageTag, &Parent>>()
            .into_iter()
            .map(|(entity, _)| entity)
            .collect();

        // 엔터티의 계층 구조를 갱신합니다.
        for entity in entities {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 스테이지 엔터티의 메쉬 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 월드 변환 행렬이 갱신되어야합니다.
    ///
    fn prepare_stage_resource(&mut self, device: &wgpu::Device) {
        // 스테이지 지역 엔터티와 소품 엔터티를 수집합니다.
        let entities: Vec<_> = self
            .world
            .query_mut::<Without<&StageTag, &Parent>>()
            .into_iter()
            .map(|(entity, _)| entity)
            .collect();

        // 엔터티의 메쉬 리소스를 갱신합니다.
        prepare_mesh_resource(&self.world, &entities, device, 32);
    }

    /// 게임 서버에 플레이어 데이터를 전송합니다.
    fn push_player_data(&mut self, net_manager: &NetManager) {
        type Components<'a> = (&'a WorldTransform, &'a ViewState, &'a ViewStateTimer);

        // 플레이어 엔터티로부터 필요한 컴포넌트 데이터를 가져옵니다.
        let entity = self.get_player_entity();
        let (world_transform, &view_state, &view_state_timer) = self
            .world
            .query_one_mut::<Components>(entity)
            .expect("invalid entity or invalid entity component");
        let rotation = world_transform.get_rotation().to_array();
        let direction = self.move_direction.0.to_array();

        // 메인 카메라 엔터티로부터 카메라 방향 데이터를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        let view_rotation = third_person_camera.rotation;

        // 패킷을 생성하고, 전송합니다.
        let pakcet = PushStatusPacket {
            user_id: self.user_id,
            token: self.token,
            rotation,
            direction,
            input_flags: self.controller_input_flags,
            view_state,
            view_state_timer,
            view_rotation,
        };
        let socket = net_manager.get(&SERVER_TCP_ADDR).expect("no such socket");
        socket.push_packet(pakcet.as_raw());
    }

    /// 서버 데이터를 게임 월드에 반영합니다.
    fn pull_game_world(&mut self, packet: PullStagePacket, app: &dyn AppHandle) {
        // 현재 게임 월드에 존재하는 플레이어의 식별자를 수집합니다.
        let mut identifiers: HashSet<UserId> = self.players.keys().cloned().collect();
        // 게임 월드에 존재하는 플레이어를 갱신합니다.
        let new = self.update_player_from_packet(&packet.players, &mut identifiers);
        // 새로운 플레이어를 게임 월드에 추가합니다.
        self.add_player_from_packet(new, app.render_device(), app.render_queue());
        self.remove_player_from_packet(identifiers.into_iter());

        // 현재 게임 월드에 존재하는 오브젝트의 식별자를 수집합니다.
        let mut identifiers: HashSet<ObjectId> = self.objects.keys().cloned().collect();
        // 게임 월드에 존재하는 총알을 갱신합니다.
        let new = self.update_bullet_from_packet(&packet.bullets, &mut identifiers);
        // 새로운 총알을 게임 월드에 추가합니다.
        self.add_bullet_from_packet(new, app.render_device(), app.render_queue());
        // 제거된 오브젝트를 게임월드에서 제거합니다.
        self.remove_object_from_packet(identifiers.into_iter());
    }

    /// 서버에서 보낸 플레이어 데이터로 갱신합니다.
    ///
    /// 새로운 플레이어 데이터를 반환합니다.
    ///
    fn update_player_from_packet<'a>(
        &mut self,
        players: &'a [PlayPhasePlayer],
        identifiers: &mut HashSet<UserId>,
    ) -> Vec<&'a PlayPhasePlayer> {
        // 컴포넌트 뷰를 준비합니다.
        let mut health_point_view = self.world.view::<(&mut MaxHealthPoint, &mut HealthPoint)>();
        let mut action_state_view = self
            .world
            .view::<(&mut ActionState, &mut ActionStateTimer)>();
        let mut movement_state_view = self
            .world
            .view::<(&mut MovementState, &mut MovementStateTimer)>();
        let mut view_state_view = self
            .world
            .view::<(&mut ViewState, &mut ViewStateTimer, &mut LatLon)>();
        let mut local_transform_view = self.world.view::<&mut ToParentTrans>();

        // 새로운 플레이어 데이터를 수집합니다.
        let mut new = Vec::with_capacity(10);

        for player in players {
            // 현재 플레이어의 경우
            if player.account.uid == self.user_id {
                identifiers.remove(&self.user_id);
                let entity = self.get_player_entity();

                // 플레이어 체력을 갱신합니다.
                let (max_hp, hp) = health_point_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *max_hp = player.max_health_point;
                *hp = player.health_point;

                // 행동 상태, 행동 상태 지속 시간을 갱신합니다.
                let (action_state, action_state_timer) = action_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *action_state = player.action_state();
                *action_state_timer = player.action_state_timer;

                // 움직임 상태, 움직임 상태 지속 시간을 갱신합니다.
                let (movement_state, movement_state_timer) = movement_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *movement_state = player.movement_state();
                *movement_state_timer = player.movement_state_timer;

                #[cfg(not(feature = "print-transform"))]
                {
                    // 플레이어 엔터티의 위치를 갱신합니다.
                    let local_transform = local_transform_view
                        .get_mut(entity)
                        .expect("invalid entity or invalid entity component");
                    local_transform.set_translation(glam::Vec3::from_array(player.translation));
                }

                continue;
            }

            // 이미 존재했던 오브젝트인 경우 오브젝트의 데이터를 갱신합니다.
            if identifiers.remove(&player.account.uid) {
                // 오브젝트의 엔터티를 가져옵니다.
                let entity = self
                    .players
                    .get(&player.account.uid)
                    .cloned()
                    .expect("no such entity");

                // 플레이어 체력을 갱신합니다.
                let (max_hp, hp) = health_point_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *max_hp = player.max_health_point;
                *hp = player.health_point;

                // 행동 상태, 행동 상태 지속 시간을 갱신합니다.
                let (action_state, action_state_timer) = action_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *action_state = player.action_state();
                *action_state_timer = player.action_state_timer;

                // 움직임 상태, 움직임 상태 지속 시간을 갱신합니다.
                let (movement_state, movement_state_timer) = movement_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *movement_state = player.movement_state();
                *movement_state_timer = player.movement_state_timer;

                // 카메라 상태, 카메라 상태 지속 시간을 갱신합니다.
                let (view_state, view_state_timer, view_rotation) = view_state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");
                *view_state = player.view_state();
                *view_state_timer = player.view_state_timer;
                *view_rotation = player.view_rotation;

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
        identifiers: &mut HashSet<ObjectId>,
    ) -> Vec<&'a Bullet> {
        // 컴포넌트 뷰를 준비합니다.
        let mut local_transform_view = self.world.view::<&mut ToParentTrans>();

        // 새로운 총알 데이터를 수집합니다.
        let mut new = Vec::with_capacity(128);

        for bullet in bullet {
            // 이미 존재했던 오브젝트인 경우 오브젝트의 데이터를 갱신합니다.
            if identifiers.remove(&bullet.object_id) {
                // 오브젝트의 엔터티를 가져옵니다.
                let entity = self
                    .objects
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
        new: Vec<&'a PlayPhasePlayer>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 새로운 플레이어를 추가합니다.
        for player in new {
            // 새로운 플레이어 계층 구조를 생성합니다.
            let (root_entity, batch_commands) = spawn_player_character(
                &self.world,
                &self.model_pool,
                &self.texture_data_pool,
                &self.texture_pool,
                &self.texture_view_pool,
                &self.sampler_pool,
                player,
                device,
                &mut encoder,
                &mut staging_buffers,
            );

            // 명령어를 실행합니다.
            for (entity, mut builder) in batch_commands {
                self.world
                    .insert(entity, builder.build())
                    .expect("no such entity");
            }

            // 플레이어 목록에 새로운 엔터티를 추가합니다.
            self.players.insert(player.account.uid, root_entity);
        }

        queue.submit([encoder.finish()]);
        drop(staging_buffers);
    }

    /// 제거된 플레이어를 게임 월드에서 제거합니다.
    fn remove_player_from_packet(&mut self, identifiers: impl Iterator<Item = UserId>) {
        // 제거된 엔터티를 플레이어 목록에서 제거합니다.
        for id in identifiers {
            let entity = self.players.remove(&id).expect("no such entity");
            cleanup(&mut self.world, entity);
        }
    }
    /// 서버에서 보낸 총알 데이터 중 새로운 총알을 게임 월드에 추가합니다.
    fn add_bullet_from_packet<'a>(
        &mut self,
        new: Vec<&'a Bullet>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 새로운 플레이어를 추가합니다.
        for bullet in new {
            // 새로운 플레이어 계층 구조를 생성합니다.
            let (root_entity, batch_commands) = spwan_bullet(
                &self.world,
                &self.model_pool,
                &self.texture_view_pool,
                &self.sampler_pool,
                bullet,
                device,
                &mut encoder,
                &mut staging_buffers,
            );

            // 명령어를 실행합니다.
            for (entity, mut builder) in batch_commands {
                self.world
                    .insert(entity, builder.build())
                    .expect("no such entity");
            }

            // 오브젝트 목록에 새로운 엔터티를 추가합니다.
            self.objects.insert(bullet.object_id, root_entity);
        }

        queue.submit([encoder.finish()]);
        drop(staging_buffers);
    }

    /// 제거된 엔터티를 오프젝트에서 제거합니다.
    fn remove_object_from_packet(&mut self, objects: impl Iterator<Item = ObjectId>) {
        // 제거된 엔터티를 오브젝트 목록에서 제거합니다.
        for id in objects {
            let entity = self.objects.remove(&id).expect("no such entity");
            cleanup(&mut self.world, entity);
        }
    }
}

impl GameScene for InGameDominationModeScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {
        // Shadow Pass 쉐이더 리소스를 생성합니다.
        self.shadow_resource = Some(ShadowMapResource::new(
            Some("Directional Light"),
            app.render_device(),
            1024,
            1024,
            wgpu::TextureFormat::Depth32Float,
        ));
        // Composite Pass 쉐이더 리소스를 생성합니다.
        self.composite_resource = Some(CompositeResource::uninit(window, app.render_device()));

        self.register_ui_game_layout_texture(window, app);

        // 메인 카메라를 생성합니다.
        self.create_main_camera(window, app.render_device());
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
    ) {
        if !repeat {
            let config = UserConfig::get();
            let flags = config
                .get_keyboard_input(&(keycode, location))
                .map(|input| input.into_bits())
                .unwrap_or(GameInputBits::empty());
            self.controller_input_flags |= flags;
        }
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
    ) {
        // 사용자 입력 상태를 갱신합니다.
        if !repeat {
            let config = UserConfig::get();
            let flags = config
                .get_keyboard_input(&(keycode, location))
                .map(|input| input.into_bits())
                .unwrap_or(GameInputBits::empty());
            self.controller_input_flags &= !flags;
        }
    }

    #[allow(unused_variables)]
    fn on_mouse_btn_pressed(
        &mut self,
        x: f32,
        y: f32,
        button: MouseButton,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        let config = UserConfig::get();
        let flags = config
            .get_mouse_input(&button)
            .map(|input| input.into_bits())
            .unwrap_or(GameInputBits::empty());
        self.controller_input_flags |= flags;
    }

    #[allow(unused_variables)]
    fn on_mouse_btn_released(
        &mut self,
        x: f32,
        y: f32,
        button: MouseButton,
        window: &Window,
        app: &dyn AppHandle,
    ) {
        let config = UserConfig::get();
        let flags = config
            .get_mouse_input(&button)
            .map(|input| input.into_bits())
            .unwrap_or(GameInputBits::empty());
        self.controller_input_flags &= !flags;
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
    ) {
        // 메인 카메라를 회전시킵니다.
        self.rotate_main_camera(dx, dy);
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["네트워크 연결 오류"];
        let title = ERR_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["서버와 연결이 끊어졌습니다!"];
                ERR_MSG_TEXTS[i]
            }
            NetworkError::IO(_) => {
                const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] =
                    ["패킷을 읽는 도중 오류가 발생했습니다!"];
                ERR_MSG_TEXTS[i]
            }
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::SetGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) {
        match packet.packet_type() {
            PacketType::PullStage => {
                let packet = PullStagePacket::from_raw(packet);
                self.pull_game_world(packet, app);
            }
            PacketType::UdpDamageLog => {
                let packet = UdpDamageLogPacket::from_raw(packet);
                // 데미지 로그에 추가합니다.
                self.damage_logs.extend(packet.logs);
            }
            _ => panic!("invalid packet"),
        };
    }

    #[allow(unused_variables)]
    fn on_pre_update(&mut self, window: &Window, app: &dyn AppHandle) {
        // 플레이어 움직임 방향을 갱신합니다.
        self.update_player_move_direction();
        // 플레이어 카메라 상태를 갱신합니다.
        self.update_player_view_state();

        // 데미지 파티클을 생성합니다.
        self.spawn_damage_particles(app.asset_manager(), app.render_device(), app.render_queue());
    }

    #[allow(unused_variables)]
    fn on_update(&mut self, elapsed_time_sec: f32, window: &Window, app: &dyn AppHandle) {
        // 플레이어 카메라 상태 지속 시간을 갱신합니다.
        self.update_player_view_state_timer(elapsed_time_sec);

        // 데미지 파티클을 갱신합니다.
        self.update_damage_particles(elapsed_time_sec);
    }

    #[allow(unused_variables)]
    fn on_post_update(&mut self, window: &Window, app: &dyn AppHandle) {
        // 플레이어 움직임 방향을 갱신합니다.
        self.update_player_move_direction();
        // 플레이어 카메라 상태를 갱신합니다.
        self.update_player_view_state();
        // 플레이어 캐릭터의 방향을 갱신합니다.
        self.update_player_character_direction();

        // 플레이어 데이터를 서버에 전송합니다.
        self.push_player_data(app.net_manager());
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 캐릭터 엔터티들을 가져옵니다.
        let characters_entities = self.get_character_entities();
        // 캐릭터 애니메이션을 재생합니다.
        self.animate_characters(&characters_entities, app.asset_manager());
        // 캐릭터 엔터티의 계층 구조를 갱신합니다.
        self.update_character_hierarchy(&characters_entities);
        // 캐릭터 쉐이더 리소스를 준비합니다.
        self.prepare_character_mesh_resource(&characters_entities, app.render_device());

        // 총알 엔터티들을 가져옵니다.
        let bullet_entities = self.get_bullet_entities();
        // 총알 엔터티의 계층 구조를 갱신합니다.
        self.update_bullet_hierarchy(&bullet_entities);
        // 총알 엔터티의 메쉬 쉐이더 리소스를 갱신합니다.
        self.prepare_bullet_mesh_resource(&bullet_entities, app.render_device());

        // 데미지 파티클 쉐이더 리소스를 갱신합니다.
        self.prepare_damage_particle_resource(app.render_device(), app.render_queue());

        // 메인 카메라의 위치 오프셋을 갱신합니다.
        self.update_main_camera_offset();
        // 메인 카메라의 계층 구조를 갱신합니다.
        self.update_main_camera_hierarchy();
        // 메인 카메라의 쉐이더 리소스를 갱신합니다.
        self.prepare_main_camera_resource(app.render_device(), app.render_queue());

        // 그림자 쉐이더 리소스를 갱신합니다.
        self.prepare_shadow_resource(app.render_device(), app.render_queue());

        // 지형의 계층 구조를 갱신합니다.
        self.update_stage_hierarchy();
        // 지형 메쉬의 쉐이더 리소스를 갱신합니다.
        self.prepare_stage_resource(app.render_device());

        // Skybox 쉐이더 리소스를 갱신합니다.
        self.prepare_skybox_resource(app.render_device(), app.render_queue());

        #[cfg(feature = "print-transform")]
        {
            let entity = self.get_player_entity();
            let skinning_animation = self
                .world
                .query_one_mut::<&SkinningAnimation>(entity)
                .expect("invalid entity or invalid entity component");
            let head = skinning_animation.head;
            let spine = skinning_animation.lower_spine;
            let spine_1 = skinning_animation.uppper_spine;
            let muzzle = skinning_animation.muzzle;
            let weapon = skinning_animation.weapon;
            let right_hand = skinning_animation.right_hand;
            let left_thigh = skinning_animation.left_thigh;
            let right_thigh = skinning_animation.right_thigh;
            let left_calf = skinning_animation.left_calf;
            let right_calf = skinning_animation.right_calf;
            let left_foot = skinning_animation.left_foot;
            let right_foot = skinning_animation.right_foot;

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(head)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            log::debug!("머리의 로컬 좌표계상의 월드 좌표계 X축: {:?}", local_x_axis);

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(spine)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            log::debug!(
                "Spine의 로컬 좌표계상의 월드 좌표계 X축: {:?}",
                local_x_axis
            );

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(spine_1)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            log::debug!(
                "Spine_1의 로컬 좌표계상의 월드 좌표계 X축: {:?}",
                local_x_axis
            );

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(left_thigh)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            let local_z_axis = transform.world_to_model_vector3a(glam::Vec3A::Z);
            log::debug!(
                "Left_Thigh의 로컬 좌표계상의 월드 좌표계 X축:{:?}, Z축:{:?}",
                local_x_axis,
                local_z_axis,
            );

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(right_thigh)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            let local_z_axis = transform.world_to_model_vector3a(glam::Vec3A::Z);
            log::debug!(
                "Right_Thigh의 로컬 좌표계상의 월드 좌표계 X축:{:?}, Z축:{:?}",
                local_x_axis,
                local_z_axis,
            );

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(left_calf)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            let local_z_axis = transform.world_to_model_vector3a(glam::Vec3A::Z);
            log::debug!(
                "Left_Calf의 로컬 좌표계상의 월드 좌표계 X축:{:?}, Z축:{:?}",
                local_x_axis,
                local_z_axis,
            );

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(right_calf)
                .expect("invalid entity or invalid entity component");
            let local_x_axis = transform.world_to_model_vector3a(glam::Vec3A::X);
            let local_z_axis = transform.world_to_model_vector3a(glam::Vec3A::Z);
            log::debug!(
                "Right_Calf의 로컬 좌표계상의 월드 좌표계 X축:{:?}, Z축:{:?}",
                local_x_axis,
                local_z_axis,
            );

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(muzzle)
                .expect("invalid entity or invalid entity component");
            log::debug!("총구의 위치: {:?}", transform.get_translation());
            log::debug!("총구의 z축 방향: {:?}", transform.get_look_vector());

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(right_hand)
                .expect("invalid entity or invalid entity component");
            let inverse_right_hand = transform.0.inverse();

            let transform = self
                .world
                .query_one_mut::<&WorldTransform>(weapon)
                .expect("invalid entity or invalid entity component");
            let weapon_offset = inverse_right_hand * transform.0;
            println!("오프셋 행렬:{}", weapon_offset);

            let transform = self
                .world
                .query_one_mut::<&ToParentTrans>(left_thigh)
                .expect("invalid entity or invalid entity component");
            log::debug!("Left Thigh 로컬 변환 행렬: {}", transform.0);

            let transform = self
                .world
                .query_one_mut::<&ToParentTrans>(right_thigh)
                .expect("invalid entity or invalid entity component");
            log::debug!("Right Thigh 로컬 변환 행렬: {}", transform.0);

            let transform = self
                .world
                .query_one_mut::<&ToParentTrans>(left_calf)
                .expect("invalid entity or invalid entity component");
            log::debug!("Left Calf 로컬 변환 행렬: {}", transform.0);

            let transform = self
                .world
                .query_one_mut::<&ToParentTrans>(right_calf)
                .expect("invalid entity or invalid entity component");
            log::debug!("Right Calf 로컬 변환 행렬: {}", transform.0);

            let transform = self
                .world
                .query_one_mut::<&ToParentTrans>(left_foot)
                .expect("invalid entity or invalid entity component");
            log::debug!("Left Foot 로컬 변환 행렬: {}", transform.0);

            let transform = self
                .world
                .query_one_mut::<&ToParentTrans>(right_foot)
                .expect("invalid entity or invalid entity component");
            log::debug!("Right Foot 로컬 변환 행렬: {}", transform.0);
        }
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        app: &dyn AppHandle,
    ) {
        let device = app.render_device();
        let queue = app.render_queue();

        // 캐릭터 엔터티 목록을 가져와 분류합니다.
        let character_entities = self.get_character_entities();
        let (character_set, character_halo_set) =
            categorize_character_resource(&self.world, &character_entities);

        // 카메라 쉐이더 리소스를 가져옵니다.
        let mut query = self
            .world
            .query_one::<&Arc<CameraResource>>(self.main_camera)
            .expect("invalid entity");
        let camera_resource = query.get().expect("invalid entity component");

        // Shadow Pass 쉐이더 리소스를 가져옵니다.
        let shadow_resource = self
            .shadow_resource
            .as_ref()
            .expect("the shader resource must exist.");

        // Composite Pass 쉐이더 리소스를 가져옵니다.
        let composite_resource = self
            .composite_resource
            .as_ref()
            .expect("the shader resource must exist.");

        encoder.push_debug_group("shadow pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(Shadow)"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_resource.texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            bake_character_shadow(
                &character_set,
                &camera_resource,
                device,
                wgpu::TextureFormat::Depth32Float,
                &mut rpass,
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("opaque pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(OpaquePass)"),
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
                &character_set,
                camera_resource,
                device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            draw_character_halo(
                &character_halo_set,
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

            draw_stage(
                &self.world,
                &camera_resource,
                &shadow_resource,
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
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("transparent pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TransparentPass)"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                a: 0.0,
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        view: &composite_resource.accum_render_target,
                        resolve_target: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                a: 1.0,
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        view: &composite_resource.reveal_render_target,
                        resolve_target: None,
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            draw_damage_particle(
                &self.world,
                &device,
                &camera_resource,
                DEPTH_FORMAT,
                &mut rpass,
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("composite pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(CompositePass)"),
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
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            composite_resource.process(device, SWAPCHAIN_FORMAT, DEPTH_FORMAT, &mut rpass);
        }
        encoder.pop_debug_group();
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 십자선 원
        let reticle_pos = (640.0 * scale, 360.0 * scale);
        let reticle_radius = 4.0 * scale;
        let reticle_color = egui::Color32::from_white_alpha(192);
        let reticle = egui::Shape::circle_filled(reticle_pos.into(), reticle_radius, reticle_color);

        // 프레임 레이트 텍스트
        let fps = app.timer().frame_rate();
        let text = format!("{}FPS", fps);
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let frame_rate_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE)
            .background_color(egui::Color32::from_black_alpha(96));

        // 체력 텍스트
        let entity = self.get_player_entity();
        let (&max_hp, &hp) = self
            .world
            .query_one_mut::<(&MaxHealthPoint, &HealthPoint)>(entity)
            .expect("invalid entity or invalid entity component");
        let percent = (hp.0 as f32 / max_hp.0.get() as f32).min(1.0);

        let text = format!("{}", hp.0.min(9999));
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let health_point_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        // 체력 인터페이스 레이아웃 이미지
        // - 기준 가로 크기: 280
        // - 기준 세로 크기: 94
        // - 기준 시작 위치: (30, 596)
        // - 기준 종료 위치: (310, 690)
        //
        let tex_width = self.ui_game_layout_texture.size.x;
        let tex_height = self.ui_game_layout_texture.size.y;
        let src_front = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: self.ui_game_layout_texture.id,
        };
        let pos_front = egui::Rect::from_min_max(
            egui::pos2(30.0 * scale, 596.0 * scale),
            egui::pos2(66.0 * scale, 690.0 * scale),
        );
        let uv_front = egui::Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(0.59375, 1.0));

        let src_middle = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.1875, tex_height),
            id: self.ui_game_layout_texture.id,
        };
        let pos_middle = egui::Rect::from_min_max(
            egui::pos2(66.0 * scale, 596.0 * scale),
            egui::pos2(274.0 * scale, 690.0 * scale),
        );
        let uv_middle =
            egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(0.40625, 1.0));

        let src_back = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: self.ui_game_layout_texture.id,
        };
        let pos_back = egui::Rect::from_min_max(
            egui::pos2(274.0 * scale, 596.0 * scale),
            egui::pos2(310.0 * scale, 690.0 * scale),
        );
        let uv_back = egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.0, 1.0));

        // 체력 인터페이스 데코레이션
        // - 기준 가로 크기: 210
        // - 기준 세로 크기: 2
        // - 기준 시작 위치: (75, 678)
        // - 기준 종료 위치: (285, 680)
        let deco_pos = egui::Rect::from_min_max(
            egui::pos2(75.0 * scale, 678.0 * scale),
            egui::pos2(285.0 * scale, 680.0 * scale),
        );
        let deco_uv = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(0.40625, 1.0));

        egui::Area::new(egui::Id::new("Reticle_Layout"))
            .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
            .show(app.egui_ctx(), |ui| {
                ui.painter().add(reticle);
            });

        egui::Area::new(egui::Id::new("Health_BG_Layout")).show(app.egui_ctx(), |ui| {
            egui::Image::new(src_front)
                .uv(uv_front)
                .tint(egui::Color32::from_black_alpha(192))
                .paint_at(ui, pos_front);
            egui::Image::new(src_middle)
                .uv(uv_middle)
                .tint(egui::Color32::from_black_alpha(192))
                .paint_at(ui, pos_middle);
            egui::Image::new(src_back)
                .uv(uv_back)
                .tint(egui::Color32::from_black_alpha(192))
                .paint_at(ui, pos_back);

            egui::Image::new(self.ui_game_layout_texture)
                .uv(deco_uv)
                .paint_at(ui, deco_pos);
        });

        egui::Area::new(egui::Id::new("Health_Gauge_Layout")).show(app.egui_ctx(), |ui| {
            // 기준 가로 크기: 39.6
            // 기준 세로 크기: 52
            // 기준 간격 가로 크기: 3
            // 기준 시작 위치: (55, 612)
            // 기준 종료 위치: (280, 647.5)
            // 기준 범위: 225
            let pivot_x = 55.0 * scale;
            let range_x = 225.0 * percent * scale;
            let maximum = 225.0 * scale;
            let mut beg_x = pivot_x;
            let mut end_x: f32;
            let mut rect: egui::Rect;

            while beg_x < pivot_x + range_x {
                end_x = beg_x + 35.5 * scale;
                let x = if end_x > pivot_x + range_x {
                    rect = egui::Rect::from_min_max(
                        egui::pos2(beg_x, 612.0 * scale),
                        egui::pos2(end_x, 647.5 * scale),
                    );
                    let shape =
                        egui::Shape::rect_filled(rect, 1.5 * scale, egui::Color32::DARK_GRAY);
                    ui.painter().add(shape);

                    pivot_x + range_x
                } else {
                    end_x
                };

                let fill_color = match beg_x < pivot_x + maximum * 0.3 {
                    true => egui::Color32::LIGHT_RED,
                    false => egui::Color32::WHITE,
                };
                rect = egui::Rect::from_min_max(
                    egui::pos2(beg_x, 612.0 * scale),
                    egui::pos2(x, 647.5 * scale),
                );
                let shape = egui::Shape::rect_filled(rect, 1.5 * scale, fill_color);
                ui.painter().add(shape);

                beg_x = end_x + 2.4 * scale;
            }

            while beg_x < pivot_x + maximum {
                end_x = beg_x + 36.25 * scale;
                rect = egui::Rect::from_min_max(
                    egui::pos2(beg_x, 612.0 * scale),
                    egui::pos2(end_x, 647.5 * scale),
                );
                let shape = egui::Shape::rect_filled(rect, 1.5 * scale, egui::Color32::DARK_GRAY);
                ui.painter().add(shape);

                beg_x = end_x + 1.5 * scale;
            }
        });

        egui::Area::new(egui::Id::new("Health_Number_Layout"))
            .anchor(egui::Align2::LEFT_BOTTOM, (70.0 * scale, -38.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.set_width(128.0 * scale);
                ui.label(health_point_text).interact(egui::Sense::empty())
            });

        egui::Area::new(egui::Id::new("FrameRate_Layout"))
            .anchor(egui::Align2::LEFT_TOP, (0.0, 0.0))
            .show(app.egui_ctx(), |ui| {
                ui.label(frame_rate_text);
            });
    }
}
