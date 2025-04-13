use ahash::{HashMap, HashSet};
use hecs::{Entity, ViewBorrow, World};
use mod_app::{app::AppHandle, net::NetManager, scene::GameScene};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, Bullet, CharacterKind, GameInputBits, HealthPoint, LatLon,
        LoginToken, MaxHealthPoint, MovementState, MovementStateTimer, ObjectId, PlayPhasePlayer,
        UserId, ViewState, ViewStateTimer,
    },
    protocol::{Packet, PacketType, PullStagePacket, PushStatusPacket, RawPacket},
};
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        ModelPool, MotionPool, SamplerPool, TexturePool, TextureViewPool, NOTOSANS_REGULAR,
        UI_GAME_LAYOUT_URI,
    },
    component::{
        animate_character, cleanup, prepare_camera_resource, set_weapon_position, spawn_bullet,
        update_character_direction, update_entity_hierarchy, update_third_person_camera,
        update_third_person_camera_hierarchy, update_view_state_by_controller_input_flags,
        update_view_state_timer, BoneCollection, BoneTransformUniform, Child, MoveDirection,
        Projection, ShadowResource, Sibling, SkinningAnimation, ThirdPersonCamera, ToParentTrans,
        TransformDataLayout, TransformUniform, WeightedBlendedOITResource, WorldTransform,
    },
    config::{Locale, UserConfig},
    scenes::BASE_WIDTH,
    SERVER_TCP_ADDR,
};

/// 종합전술시험(점령전)을 진행하는 게임 장면입니다.
pub struct InGameDominationModeScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 사용자의 식별자입니다.
    user_id: UserId,
    /// 현재 사용자의 로그인 토큰입니다.
    token: LoginToken,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 게임 장면의 경과 시간입니다.
    /// 패킷을 보낼 때 사용됩니다.
    elapsed_time_sec: f32,

    /// 엔터티를 관리하는 월드 객체입니다.
    world: World,
    /// 스카이박스 엔터티입니다.
    skybox: Entity,
    /// 메인 카메라 엔터티입니다.
    main_camera: Entity,
    /// 플레이어 엔터티 집합입니다.
    players: HashMap<UserId, Entity>,
    /// 오브젝트 엔터티 집합입니다.
    bullets: HashMap<ObjectId, Entity>,
    /// 지형 엔터티 집합입니다.
    stages: HashMap<ObjectId, Entity>,
    /// 조명 엔터티 집합입니다.
    lights: HashMap<ObjectId, Entity>,

    /// 플레이어 움직임 방향입니다.
    move_direction: MoveDirection,
    /// 사용자 입력 상태 플래그 변수입니다.
    controller_input_flags: GameInputBits,

    /// 그림자 쉐이더 리소스입니다.
    shadow_resource: Option<ShadowResource>,
    /// 알파 블렌딩 쉐이더 리소스입니다.
    alpha_blend_resource: Option<WeightedBlendedOITResource>,

    /// 게임 인터페이스 레이아웃 텍스처 식별자입니다.
    ui_bg_texture: egui::load::SizedTexture,

    /// 모델 풀 객체입니다.
    model_pool: ModelPool,
    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
}

//--------------------------------------------------------------------------------------------
// 초기화 함수만 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 새로운 `InGameDominationModeScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        flip_horizontal: bool,
        flip_vertical: bool,
        world: World,
        skybox: Entity,
        main_camera: Entity,
        players: HashMap<UserId, Entity>,
        bullets: HashMap<ObjectId, Entity>,
        stages: HashMap<ObjectId, Entity>,
        lights: HashMap<ObjectId, Entity>,
        model_pool: ModelPool,
        motion_pool: MotionPool,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            flip_horizontal,
            flip_vertical,
            elapsed_time_sec: 0.0,
            skybox,
            world,
            main_camera,
            players,
            bullets,
            stages,
            lights,
            move_direction: MoveDirection::default(),
            controller_input_flags: GameInputBits::default(),
            shadow_resource: None,
            alpha_blend_resource: None,
            ui_bg_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            model_pool,
            motion_pool,
            texture_pool,
            texture_view_pool,
            sampler_pool,
        }
    }

    /// UI 배경에 사용되는 텍스처를 Ui렌더러에 등록합니다.
    fn register_ui_bg_texture(&mut self, app: &dyn AppHandle) {
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

        self.ui_bg_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 그림자 쉐이더 리소스를 생성합니다.
    fn create_shadow_resource(&mut self, device: &wgpu::Device) {
        let resource =
            ShadowResource::new(1024, 1024, 1, device, wgpu::TextureFormat::Depth32Float);
        self.shadow_resource = resource.into();
    }

    /// 알파 블렌드에 사용되는 쉐이더 리소스를 생성합니다.
    fn create_alpha_blend_resource(&mut self, window: &Window, device: &wgpu::Device) {
        let (width, height): (u32, u32) = window.inner_size().into();
        let resource = WeightedBlendedOITResource::new(width, height, device);
        self.alpha_blend_resource = resource.into();
    }
}

//--------------------------------------------------------------------------------------------
// 플레이어 조작과 관련된 함수만 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 플레이어 엔터티를 반환합니다.
    fn get_player_entity(&self) -> Entity {
        self.players
            .get(&self.user_id)
            .cloned()
            .expect("the player entity must exist!")
    }

    /// 플레이어 움직임 방향을 갱신합니다.
    fn update_move_direction(&mut self) {
        // 삼인칭 카메라 요소를 가져옵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 삼인칭 카메라의 방향을 기준으로 플레이어 움직임 방향을 갱신합니다.
        let controller = self.controller_input_flags.as_state();
        self.move_direction
            .update_from_third_person_camera(controller, third_person_camera);
    }

    /// 캐릭터가 바라보는 방향을 갱신합니다.
    fn update_character_direction(&mut self) {
        // 삼인칭 카메라 요소를 가져옵니다.
        let mut query = self
            .world
            .query_one::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity");
        let third_person_camera = query.get().expect("invalid entity component");

        // 행동 상태, 행동 상태 타이머, 움직임 상태, 로컬 변환 행렬 요소를 가져옵니다.
        type Query<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a mut ToParentTrans,
        );
        let entity = self.get_player_entity();
        let mut query = self
            .world
            .query_one::<Query>(entity)
            .expect("invalid entity");
        let (&character_kind, &action_state, &action_state_timer, &movement_state, local_transform) =
            query.get().expect("invalid entity component");

        // 캐릭터가 바라보는 방향을 갱신합니다.
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

    /// 플레이어 카메라 상태를 갱신합니다.
    fn pre_update_view_state(&mut self) {
        // 캐릭터 종류, 카메라 상태, 카메라 상태 타이머 요소를 가져옵니다.
        type Query<'a> = (&'a CharacterKind, &'a mut ViewState, &'a mut ViewStateTimer);
        let entity = self.get_player_entity();
        let (&character_kind, view_state, view_state_timer) = self
            .world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");

        // 현재 입력 상태에 따라 카메라 상태를 갱신합니다.
        update_view_state_by_controller_input_flags(
            character_kind,
            view_state,
            view_state_timer,
            self.controller_input_flags,
        );
    }

    /// 플레이어 카메라 상태를 갱신합니다.
    fn update_view_state(&mut self, elapsed_time_sec: f32) {
        // 캐릭터 종류, 카메라 상태, 카메라 상태 타이머 요소를 가져옵니다.
        type Query<'a> = (&'a CharacterKind, &'a mut ViewState, &'a mut ViewStateTimer);
        let entity = self.get_player_entity();
        let (&character_kind, view_state, view_state_timer) = self
            .world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");

        // 현재 입력 상태에 따라 카메라 상태를 갱신합니다.
        update_view_state_timer(
            character_kind,
            view_state,
            view_state_timer,
            elapsed_time_sec,
        );
    }
}

//--------------------------------------------------------------------------------------------
// 네트워크 통신과 관련된 함수만 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 게임 서버에 플레이어 데이터를 전송합니다.
    fn push_player_data(&mut self, net_manager: &NetManager) {
        // 패킷 지연 시간 이후 패킷을 전송합니다.
        const DEALY: f32 = 1.0 / 120.0;
        if self.elapsed_time_sec < DEALY {
            return;
        }
        self.elapsed_time_sec = 0.0;

        // 플레이어 데이터를 수집합니다.
        type Query<'a> = (&'a WorldTransform, &'a ViewState, &'a ViewStateTimer);
        let entity = self.get_player_entity();
        let (world_transform, &view_state, &view_state_timer) = self
            .world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");
        let rotation = world_transform.get_rotation().to_array();
        let direction = self.move_direction.0.to_array();
        let input_flags = self.controller_input_flags;

        let third_person_camera = self
            .world
            .query_one_mut::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        let view_rotation = third_person_camera.rotation;

        // 패킷을 생성 후 전송합니다.
        let packet = PushStatusPacket {
            user_id: self.user_id,
            token: self.token,
            rotation,
            direction,
            input_flags,
            view_state,
            view_state_timer,
            view_rotation,
        };
        let socket = net_manager.get(&SERVER_TCP_ADDR).expect("no such socket");
        socket.push_packet(packet.as_raw());
    }

    /// 서버의 게임 데이터를 반영합니다.
    fn pull_game_data(&mut self, packet: PullStagePacket, app: &dyn AppHandle) {
        let device = app.render_device();
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.update_player_from_packet(&packet.players);
        self.update_bullet_from_packet(&packet.bullets, device, &mut encoder, &mut staging_buffers);

        app.render_queue().submit(Some(encoder.finish()));
        drop(staging_buffers);
    }

    /// 패킷 데이터로 플레이어를 갱신합니다.
    fn update_player_from_packet<'a>(&mut self, players: &'a [PlayPhasePlayer]) {
        type Query<'a> = (
            &'a mut MaxHealthPoint,
            &'a mut HealthPoint,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut ViewState,
            &'a mut ViewStateTimer,
            &'a mut LatLon,
            &'a mut ToParentTrans,
        );
        let mut component_view = self.world.view_mut::<Query>();

        // 플레이어 데이터를 수정합니다.
        let mut ids: HashSet<UserId> = self.players.keys().cloned().collect();
        for data in players {
            ids.remove(&data.account.uid);
            if let Some(entity) = self.players.get(&data.account.uid).cloned() {
                let (
                    max_health_point,
                    health_point,
                    action_state,
                    action_state_timer,
                    movement_state,
                    movement_state_timer,
                    view_state,
                    view_state_timer,
                    view_rotation,
                    local_transform,
                ) = component_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component");

                *max_health_point = data.max_health_point;
                *health_point = data.health_point;
                *action_state = data.action_state();
                *action_state_timer = data.action_state_timer;
                *movement_state = data.movement_state();
                *movement_state_timer = data.movement_state_timer;

                if data.account.uid == self.user_id {
                    local_transform.set_translation(data.translation.into());
                } else {
                    *view_state = data.view_state();
                    *view_state_timer = data.view_state_timer;
                    *view_rotation = data.view_rotation;
                    local_transform.set_rotation_translation(
                        glam::Quat::from_array(data.rotation),
                        data.translation.into(),
                    );
                }
            } else {
                log::warn!("Unknown game player (UID:{})", data.account.uid);
            }
        }
        drop(component_view);

        // 제거된 플레이어를 게임 월드에서 제거합니다.
        for id in ids {
            let entity = self.players.remove(&id).expect("no such entity");
            cleanup(&mut self.world, entity);
        }
    }

    /// 패킷 데이터로 총알을 갱신합니다.
    fn update_bullet_from_packet<'a>(
        &mut self,
        bullets: &'a [Bullet],
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        type Query<'a> = &'a mut ToParentTrans;
        let mut component_view = self.world.view::<Query>();

        let mut ids: HashSet<ObjectId> = self.bullets.keys().cloned().collect();
        let mut batch_commands = Vec::new();
        for data in bullets {
            ids.remove(&data.object_id);
            if let Some(entity) = self.bullets.get(&data.object_id).cloned() {
                let local_transform = component_view
                    .get_mut(entity)
                    .expect("invalid entity of invalid entity component");

                local_transform.set_rotation_translation(
                    glam::Quat::from_array(data.rotation),
                    data.translation.into(),
                );
            } else {
                let (entity, mut batch_command) = spawn_bullet(
                    &self.world,
                    &self.model_pool,
                    &self.texture_view_pool,
                    &self.sampler_pool,
                    data,
                    device,
                    encoder,
                    staging_buffers,
                );

                self.bullets.insert(data.object_id, entity);
                batch_commands.append(&mut batch_command);
            }
        }
        drop(component_view);

        // 엔터티 생성 명령어를 실행합니다.
        for (entity, mut builder) in batch_commands {
            self.world
                .insert(entity, builder.build())
                .expect("no such entity");
        }

        // 제거된 총알을 게임 월드에서 제거합니다.
        for id in ids {
            let entity = self.bullets.remove(&id).expect("no such entity");
            cleanup(&mut self.world, entity);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 엔터티 계층 구조 갱신과 관련된 함수만 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 카메라를 갱신합니다.
    fn update_camera(&mut self) {
        // 삼인칭 카메라 대상의 요소를 가져옵니다.
        type Query<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a ViewState,
            &'a ViewStateTimer,
            &'a WorldTransform,
        );
        let entity = self.get_player_entity();
        let (
            &character_kind,
            &action_state,
            &action_state_timer,
            &view_state,
            &view_state_timer,
            world_transform,
        ) = self
            .world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");
        let target_pos = world_transform.get_translation();

        // 삼인칭 카메라 요소를 가져옵니다.
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

        // 삼인칭 카메라의 계층 구조를 갱신합니다.
        update_third_person_camera_hierarchy(&mut self.world, self.main_camera, target_pos);
    }

    /// 캐릭터 애니메이션을 재생합니다.
    fn animate_character(&mut self) {
        type Query<'a> = (
            &'a CharacterKind,
            &'a SkinningAnimation,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a MovementStateTimer,
            &'a LatLon,
        );
        let element_view = self.world.view::<Query>();
        let collection_view = self.world.view::<&BoneCollection>();
        let mut transform_view = self.world.view::<&mut ToParentTrans>();

        for entity in self.players.values().cloned() {
            let (
                &character_kind,
                skinning_animation,
                &action_state,
                &action_state_timer,
                &movement_state,
                &movement_state_timer,
                &view_rotation,
            ) = element_view
                .get(entity)
                .expect("invalid entity or invalid entity component");

            animate_character(
                &self.motion_pool,
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

    /// 캐릭터의 무기를 갱신합니다.
    fn update_character_weapon(&mut self) {
        type Query<'a> = (&'a CharacterKind, &'a ActionState, &'a SkinningAnimation);
        let element_view = self.world.view::<Query>();
        let child_view = self.world.view::<&Child>();
        let sibling_view = self.world.view::<&Sibling>();
        let mut transform_view = self.world.view::<(&ToParentTrans, &mut WorldTransform)>();

        for entity in self.players.values().cloned() {
            let (&character_kind, &action_state, skinning_animation) = element_view
                .get(entity)
                .expect("invalid entity or invalid entity component");

            set_weapon_position(
                character_kind,
                action_state,
                skinning_animation,
                &child_view,
                &sibling_view,
                &mut transform_view,
            );
        }
    }

    /// 캐릭터 엔터티의 계층 구조를 갱신합니다.
    fn update_character(&mut self) {
        self.animate_character();

        // 캐릭터의 계층 구조를 갱신합니다.
        for entity in self.players.values().cloned() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }

        self.update_character_weapon();
    }

    /// 총알 엔터티의 계층 구조를 갱신합니다.
    fn update_bullet(&mut self) {
        for entity in self.bullets.values().cloned() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 지형 엔터티의 계층 구조를 갱신합니다.
    fn update_stage(&mut self) {
        for entity in self.stages.values().cloned() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 쉐이더 리소스 갱신과 관련된 함수만 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 카메라 쉐이더 리소스를 준비합니다.
    fn prepare_camera_resource(
        &mut self,
        device: &wgpu::Device,
    ) -> (Vec<wgpu::CommandBuffer>, Vec<wgpu::Buffer>) {
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 카메라의 투영 변환 행렬을 갱신합니다.
        type Query<'a> = (&'a ThirdPersonCamera, &'a mut Projection);
        let (third_person_camera, projection) = self
            .world
            .query_one_mut::<Query>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        *projection = Projection::perspective(third_person_camera.fov_y, 16.0 / 9.0, 0.01, 500.0);

        // 카메라의 쉐이더 리소스를 갱신합니다.
        prepare_camera_resource(
            &self.world,
            self.main_camera,
            device,
            &mut encoder,
            &mut staging_buffers,
        );

        (vec![encoder.finish()], staging_buffers)
    }

    /// 엔터티의 쉐이더 리소스를 갱신하는 재귀함수입니다.
    fn prepare_entity_resource_recursion(
        entity: Entity,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        transform_view: &ViewBorrow<'_, &WorldTransform>,
        mesh_resource_view: &ViewBorrow<'_, &TransformUniform>,
        skinned_mesh_resource_view: &ViewBorrow<'_, (&BoneCollection, &BoneTransformUniform)>,
    ) {
        // 형제 엔터티가 존재하는 경우 형제 엔터티를 탐색합니다.
        if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
            Self::prepare_entity_resource_recursion(
                *sibling_entity,
                device,
                encoder,
                staging_buffers,
                child_view,
                sibling_view,
                transform_view,
                mesh_resource_view,
                skinned_mesh_resource_view,
            );
        }

        // 자식 엔터티가 존재하는 경우 자식 엔터티를 탐색합니다.
        if let Some(child_entity) = child_view.get(entity).cloned() {
            Self::prepare_entity_resource_recursion(
                *child_entity,
                device,
                encoder,
                staging_buffers,
                child_view,
                sibling_view,
                transform_view,
                mesh_resource_view,
                skinned_mesh_resource_view,
            );
        }

        // 현재 엔터티가 조건에 부합하는지 확인합니다.
        if let Some(uniform) = mesh_resource_view.get(entity) {
            let transform = transform_view
                .get(entity)
                .expect("invalid entity or invalid entity component");
            uniform.update(
                device,
                encoder,
                staging_buffers,
                TransformDataLayout {
                    trans: transform.0.to_cols_array(),
                    ..Default::default()
                },
            );
        } else if let Some((collection, uniform)) = skinned_mesh_resource_view.get(entity) {
            let data = collection
                .bones
                .iter()
                .map(|&entity| {
                    transform_view
                        .get(entity)
                        .expect("invalid entity or invalid entity component")
                })
                .map(|transform| transform.0.to_cols_array())
                .collect();
            uniform.update(device, encoder, staging_buffers, data);
        };
    }

    /// 캐릭터의 쉐이더 리소스를 갱신합니다.
    fn prepare_character_resource(
        &mut self,
        device: &wgpu::Device,
    ) -> (Vec<wgpu::CommandBuffer>, Vec<wgpu::Buffer>) {
        let child_view = self.world.view::<&Child>();
        let sibling_view = self.world.view::<&Sibling>();
        let transform_view = self.world.view::<&WorldTransform>();
        let mesh_resource_view = self.world.view::<&TransformUniform>();
        let skinned_mesh_resource_view = self
            .world
            .view::<(&BoneCollection, &BoneTransformUniform)>();

        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        for entity in self.players.values().cloned() {
            Self::prepare_entity_resource_recursion(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
                &child_view,
                &sibling_view,
                &transform_view,
                &mesh_resource_view,
                &skinned_mesh_resource_view,
            );
        }

        (vec![encoder.finish()], staging_buffers)
    }

    /// 총알의 쉐이더 리소스를 갱신합니다.
    fn prepare_bullet_resource(
        &mut self,
        device: &wgpu::Device,
    ) -> (Vec<wgpu::CommandBuffer>, Vec<wgpu::Buffer>) {
        let child_view = self.world.view::<&Child>();
        let sibling_view = self.world.view::<&Sibling>();
        let transform_view = self.world.view::<&WorldTransform>();
        let mesh_resource_view = self.world.view::<&TransformUniform>();
        let skinned_mesh_resource_view = self
            .world
            .view::<(&BoneCollection, &BoneTransformUniform)>();

        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        for entity in self.bullets.values().cloned() {
            Self::prepare_entity_resource_recursion(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
                &child_view,
                &sibling_view,
                &transform_view,
                &mesh_resource_view,
                &skinned_mesh_resource_view,
            );
        }

        (vec![encoder.finish()], staging_buffers)
    }

    /// 지형의 쉐이더 리소스를 갱신합니다.
    fn prepare_stage_resource(
        &mut self,
        device: &wgpu::Device,
    ) -> (Vec<wgpu::CommandBuffer>, Vec<wgpu::Buffer>) {
        let child_view = self.world.view::<&Child>();
        let sibling_view = self.world.view::<&Sibling>();
        let transform_view = self.world.view::<&WorldTransform>();
        let mesh_resource_view = self.world.view::<&TransformUniform>();
        let skinned_mesh_resource_view = self
            .world
            .view::<(&BoneCollection, &BoneTransformUniform)>();

        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        for entity in self.stages.values().cloned() {
            Self::prepare_entity_resource_recursion(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
                &child_view,
                &sibling_view,
                &transform_view,
                &mesh_resource_view,
                &skinned_mesh_resource_view,
            );
        }

        (vec![encoder.finish()], staging_buffers)
    }
}

//--------------------------------------------------------------------------------------------
// 렌더링과 관련된 함수만 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {}

impl GameScene for InGameDominationModeScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {
        self.register_ui_bg_texture(app);
        self.create_shadow_resource(app.render_device());
        self.create_alpha_blend_resource(window, app.render_device());
        self.update_stage(); // 정적인 지형은 매번 계층 구조를 갱신할 필요가 없다.
    }

    fn on_keyboard_pressed(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) {
        if !repeat {
            let config = UserConfig::get();
            let flags = config
                .get_keyboard_input(&(code, location))
                .map(|input| input.into_bits())
                .unwrap_or_default();
            self.controller_input_flags |= flags;
        }
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) {
        if !repeat {
            let config = UserConfig::get();
            let flags = config
                .get_keyboard_input(&(code, location))
                .map(|input| input.into_bits())
                .unwrap_or_default();
            self.controller_input_flags &= !flags;
        }
    }

    fn on_cursor_moved(
        &mut self,
        _x: f32,
        _y: f32,
        mut dx: f32,
        mut dy: f32,
        _window: &Window,
        _app: &dyn AppHandle,
    ) {
        if self.flip_horizontal {
            dx *= -1.0;
        }

        if self.flip_vertical {
            dy *= -1.0;
        }

        // 삼인칭 카메라를 회전시킵니다.
        let third_person_camera = self
            .world
            .query_one_mut::<&mut ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        third_person_camera.rotate(dx, dy, 1.0);
        let rotation = third_person_camera.rotation;

        // 플레이어에 적용합니다.
        let entity = self.get_player_entity();
        let view_rotation = self
            .world
            .query_one_mut::<&mut LatLon>(entity)
            .expect("invalid entity or invalid entity component");
        *view_rotation = rotation;
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) {
        match packet.packet_type() {
            PacketType::PullStage => {
                let packet = PullStagePacket::from_raw(packet);
                self.pull_game_data(packet, app);
            }
            PacketType::UdpDamageLog => {}
            _ => panic!("invalid packet"),
        };
    }

    fn on_pre_update(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.pre_update_view_state();
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _pp: &dyn AppHandle) {
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        self.update_move_direction();
        self.update_character_direction();
        self.update_view_state(elapsed_time_sec);
    }

    fn on_post_update(&mut self, _window: &Window, app: &dyn AppHandle) {
        self.push_player_data(app.net_manager());
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        self.update_camera();
        self.update_bullet();
        self.update_character();

        let mut staging_buffers = Vec::new();
        let mut command_buffers = Vec::new();
        let (mut command, mut buffers) = self.prepare_camera_resource(app.render_device());
        command_buffers.append(&mut command);
        staging_buffers.append(&mut buffers);

        let (mut command, mut buffers) = self.prepare_character_resource(app.render_device());
        command_buffers.append(&mut command);
        staging_buffers.append(&mut buffers);

        let (mut command, mut buffers) = self.prepare_bullet_resource(app.render_device());
        command_buffers.append(&mut command);
        staging_buffers.append(&mut buffers);

        let (mut command, mut buffers) = self.prepare_stage_resource(app.render_device());
        command_buffers.append(&mut command);
        staging_buffers.append(&mut buffers);

        app.render_queue().submit(command_buffers);
        drop(staging_buffers);
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
        let tex_width = self.ui_bg_texture.size.x;
        let tex_height = self.ui_bg_texture.size.y;
        let src_front = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: self.ui_bg_texture.id,
        };
        let pos_front = egui::Rect::from_min_max(
            egui::pos2(30.0 * scale, 596.0 * scale),
            egui::pos2(66.0 * scale, 690.0 * scale),
        );
        let uv_front = egui::Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(0.59375, 1.0));

        let src_middle = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.1875, tex_height),
            id: self.ui_bg_texture.id,
        };
        let pos_middle = egui::Rect::from_min_max(
            egui::pos2(66.0 * scale, 596.0 * scale),
            egui::pos2(274.0 * scale, 690.0 * scale),
        );
        let uv_middle =
            egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(0.40625, 1.0));

        let src_back = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: self.ui_bg_texture.id,
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

            egui::Image::new(self.ui_bg_texture)
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
