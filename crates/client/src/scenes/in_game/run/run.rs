use std::{f32::consts::TAU, sync::Arc, time::Instant};

use ahash::HashMap;
use hecs::{Entity, World};
use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        update_action_state, update_action_state_timer, update_movement_state,
        update_movement_state_timer, update_player_translation, ActionState, ActionStateTimer,
        BulletData, CharacterFlags, CharacterKind, GameInputBits, HealthData, InputStateTimer,
        LatLon, LoginToken, MovementState, MovementStateTimer, MovingDirection, NetworkState,
        Permission, SkillCostData, StageAttributes, Team, UserId, Velocity, ViewState,
        ViewStateTimer, MAX_LATITUDE, MIN_LATITUDE,
    },
    protocol::{
        InGamePullPacket, InGamePushNotifyPacket, Packet, PacketType, RawPacket, StateHistory,
        MAX_HISTORIES,
    },
};
use mod_parallelism::collections::Queue;
use mod_physics::object3d::Frustum;
use mod_render::{UiRenderer, SWAPCHAIN_FORMAT};
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        cull_stage_entities, MeshPool, ModelPool, MotionPool, SamplerPool,
        StageBoundingVolumnHierarchy, TextureDataPool, TexturePool, TextureViewPool,
    },
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage,
        clear_render_target_with_skybox, collect_character_resource, collect_stage_resource,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_character,
        draw_character_eye_mouth, draw_character_halo, draw_stage, draw_tree, get_local_transform,
        get_world_transform, lerp_local_transform, set_local_transform,
        update_camera_and_skybox_resource, update_camera_hierarchy, update_camera_param,
        update_character_hierarchy, update_character_resource, update_character_rotation,
        update_stage_hierarchy, update_stage_resource, update_view_state, update_view_state_timer,
        AccumRenderTarget, AlphaBlendPipeline, AnimationQuery, BakeList, BloomPipeline,
        BoneCollection, BrightRenderTarget, Camera, CameraResource, CameraUniform, Child,
        DirectionLight, EntitySnapshot, GaussianBlurPipeline, GlobalLightDataLayout,
        InterpolationManager, LightSetResource, LightTransformDataLayout, MaterialKind,
        MeshRenderer, OpaqueMap, Player0, Player1, Player2, Player3, Player4, Player5, Player6,
        Player7, Player8, Player9, PlayerArchetype, Projection, RenderTask, RevealRenderTarget,
        ShadowMap, Sibling, SkinnedMeshRenderer, Skybox, SnapshotBuffer, ToParentTrans,
        TransparentMap, WeaponQuery, WorldTransform, CAMERA_DEF_FOV_Y, CAMERA_DEF_REL_POS,
        CHARACTER_ATTRIBUTES,
    },
    config::{Locale, UserConfig},
    scenes::{
        FatalErrorSceneLayer, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
    },
    SERVER_TCP_ADDR,
};

/// 게임을 진행하는 장면입니다.
pub struct InGameRunScene {
    /// 현재 시대
    epoch: u32,
    /// 현재 시대 시각
    epoch_time_stamp: Instant,
    /// 이전 패킷을 보낸 후 경과 시간
    packet_elapsed_time_ms: u16,
    /// 플레이어 상태 데이터 기록
    histories: Vec<StateHistory>,

    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 시야 조작 민감도입니다.
    control_sensitivity: f32,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 스테이지 속성 데이터
    stage_attributes: Arc<StageAttributes>,
    /// 최대 게임 플레이 시간
    max_game_play_time_ms: u32,
    /// 남은 게임 진행 시간
    remaining_time_ms: u32,
    /// 첫 번쨰 마우스 눌림 여부 플래그
    first_mouse_pressed: bool,
    /// 사용자 입력 상태 플래그 변수입니다.
    input_bits: GameInputBits,
    /// 플레이어 움직임 속도입니다.
    velocity: Velocity,
    /// 플레이어 움직임 방향입니다.
    direction: MovingDirection,
    /// 플레이어 입력 타이머입니다.
    input_timer: InputStateTimer,

    /// 게임 월드
    world: Option<World>,

    /// 카메라 엔터티
    camera: Entity,
    /// 카메라 Fov-y 각도 (단위: 라디안)
    camera_fov_y: f32,
    /// 카메라 상대 위치
    camera_rel_position: glam::Vec3A,
    /// 카메라 종횡비
    camera_aspect_ratio: f32,

    /// 다른 플레이어 데이터를 관리합니다.
    interpolation: InterpolationManager,
    /// 최근 수신받은 플레이어 데이터입니다.
    latest_snapshot: Option<EntitySnapshot>,
    /// 플레이어 엔터티
    players: HashMap<UserId, (Entity, PlayerArchetype)>,
    /// 스테이지 엔터티
    stage: Option<StageBoundingVolumnHierarchy>,

    /// 누적 값 렌더 타겟
    accum_render_target: Option<AccumRenderTarget>,
    /// 노출 값 렌더 타겟
    reveal_render_target: Option<RevealRenderTarget>,
    /// 발광체 렌더 타겟
    bright_render_target: Option<BrightRenderTarget>,

    /// 알파 블렌딩을 수행하는 파이프라인
    alpha_blend_pipeline: Option<AlphaBlendPipeline>,
    /// 가우시안 블러를 수행하는 파이프라인
    gaussian_blur_pipeline: Option<GaussianBlurPipeline>,
    /// Bloom 효과를 구현하는 파이프라인
    bloom_pipeline: Option<BloomPipeline>,

    /// 스테이지 스카이박스
    skybox: Option<Skybox>,
    /// 스테이지 방향 조명
    direction_light: Option<DirectionLight>,
    /// 조명 쉐이더 리소스
    light_resource: Option<LightSetResource>,

    /// 카메라 뷰 프러스텀 컬링된 엔터티 목록
    culling_stage_entities: Vec<Entity>,
    /// 조명 렌더링 리소스 집합입니다.
    bake_list: BakeList,
    /// 불투명 메쉬 렌더링 리소스 집합입니다.
    opaque_resources: OpaqueMap,
    /// 투명 메쉬 렌더링 리소스 집합입니다.
    transparent_resources: TransparentMap,

    /// 메쉬 풀 객체입니다.
    mesh_pool: MeshPool,
    /// 모델 풀 객체입니다.
    model_pool: ModelPool,
    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 데이터 풀 객체입니다.
    texture_data_pool: TextureDataPool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
}

impl InGameRunScene {
    /// 새로운 `InGameRunScene`을 생성합니다.
    pub fn new(
        epoch: u32,
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        stage_attributes: Arc<StageAttributes>,
        remaining_time_ms: u32,
        first_mouse_pressed: bool,
        world: World,
        players: HashMap<UserId, (Entity, PlayerArchetype)>,
        stage: StageBoundingVolumnHierarchy,
        accum_render_target: AccumRenderTarget,
        reveal_render_target: RevealRenderTarget,
        bright_render_target: BrightRenderTarget,
        alpha_blend_pipeline: AlphaBlendPipeline,
        gaussian_blur_pipeline: GaussianBlurPipeline,
        bloom_pipeline: BloomPipeline,
        skybox: Skybox,
        direction_light: DirectionLight,
        light_resource: LightSetResource,
        mesh_pool: MeshPool,
        model_pool: ModelPool,
        motion_pool: MotionPool,
        texture_pool: TexturePool,
        texture_data_pool: TextureDataPool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
    ) -> Self {
        Self {
            epoch,
            epoch_time_stamp: Instant::now(),
            packet_elapsed_time_ms: 0,
            histories: Vec::with_capacity(MAX_HISTORIES),
            locale,
            uid,
            token,
            control_sensitivity: 0.5,
            flip_horizontal: false,
            flip_vertical: false,
            stage_attributes,
            max_game_play_time_ms: remaining_time_ms,
            remaining_time_ms,
            first_mouse_pressed,
            input_bits: GameInputBits::new(),
            velocity: Velocity::new(),
            direction: MovingDirection::new(),
            input_timer: InputStateTimer::new(0),
            world: Some(world),
            camera: Entity::DANGLING,
            camera_fov_y: 45f32.to_radians(),
            camera_rel_position: glam::Vec3A::ZERO,
            camera_aspect_ratio: 1.0,
            interpolation: InterpolationManager::new(),
            latest_snapshot: None,
            players,
            stage: Some(stage),
            accum_render_target: Some(accum_render_target),
            reveal_render_target: Some(reveal_render_target),
            bright_render_target: Some(bright_render_target),
            alpha_blend_pipeline: Some(alpha_blend_pipeline),
            gaussian_blur_pipeline: Some(gaussian_blur_pipeline),
            bloom_pipeline: Some(bloom_pipeline),
            skybox: Some(skybox),
            direction_light: Some(direction_light),
            light_resource: Some(light_resource),
            culling_stage_entities: Vec::default(),
            bake_list: BakeList::default(),
            opaque_resources: OpaqueMap::default(),
            transparent_resources: TransparentMap::default(),
            mesh_pool,
            model_pool,
            motion_pool,
            texture_pool,
            texture_data_pool,
            texture_view_pool,
            sampler_pool,
        }
    }

    /// 플레이어 엔터티를 반환합니다.
    fn player_entity(&self) -> (Entity, PlayerArchetype) {
        self.players
            .get(&self.uid)
            .cloned()
            .expect("no such entity!")
    }

    /// 플레이어 캐릭터 상태를 갱신합니다.
    fn update_player_character_state(&mut self) {
        let (entity, _archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        type Q<'a> = (
            &'a CharacterKind,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut ViewState,
            &'a mut ViewStateTimer,
            &'a HealthData,
            &'a mut BulletData,
            &'a mut SkillCostData,
        );
        let mut view = world.view_mut::<Q>();
        let (
            &character_kind,
            action_state,
            action_state_timer,
            movement_state,
            movement_state_timer,
            view_state,
            view_state_timer,
            health_data,
            bullet_data,
            skill_cost_data,
        ) = view
            .get_mut(entity)
            .expect("invalid entity or invalid entity component!");

        // 캐릭터 속성 데이터를 가져옵니다.
        let i = character_kind as usize;
        let character_attributes = CHARACTER_ATTRIBUTES[i];

        // 시야 상태를 갱신합니다.
        update_view_state(
            view_state,
            view_state_timer,
            character_attributes,
            self.input_bits,
        );

        if health_data.num_maximum_health() != 0 && health_data.remaining == 0 {
            return;
        }

        // 행동 상태를 갱신합니다.
        let mut events = Vec::default();
        update_action_state(
            self.input_bits,
            action_state,
            action_state_timer,
            character_attributes,
            bullet_data,
            skill_cost_data,
            &mut events,
        );

        // 움직임 상태를 갱신합니다.
        update_movement_state(
            self.input_bits,
            *action_state,
            movement_state,
            movement_state_timer,
            &mut events,
        );
    }

    /// 플레이어 캐릭터 타이머를 갱신합니다.
    fn update_player_character_timer(&mut self, elapsed_time_ms: u16) {
        let (entity, _archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        type Q<'a> = (
            &'a CharacterKind,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut ViewState,
            &'a mut ViewStateTimer,
            &'a mut BulletData,
            &'a mut SkillCostData,
        );
        let mut view = world.view::<Q>();

        let (
            &character_kind,
            action_state,
            action_state_timer,
            movement_state,
            movement_state_timer,
            view_state,
            view_state_timer,
            bullet_data,
            skill_cost_data,
        ) = view
            .get_mut(entity)
            .expect("invalid entity or invalid entity component!");

        // 캐릭터 속성 정보를 가져옵니다.
        let i = character_kind as usize;
        let character_attributes = CHARACTER_ATTRIBUTES[i];

        self.input_timer.update(self.input_bits, elapsed_time_ms);
        update_view_state_timer(
            view_state,
            view_state_timer,
            character_attributes,
            elapsed_time_ms,
        );

        let mut events = Vec::new();
        update_action_state_timer(
            self.input_bits,
            bullet_data,
            skill_cost_data,
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
            &mut events,
        );
        update_movement_state_timer(
            *action_state,
            movement_state,
            movement_state_timer,
            character_attributes,
            elapsed_time_ms,
            &mut events,
        );
    }

    /// 플레이어 캐릭터의 회전 방향을 설정합니다.
    fn update_player_character_rotation(&mut self) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        // 플레이어 카메라 방향을 가져옵니다.
        let &latlon = world
            .query_one_mut::<&LatLon>(entity)
            .expect("invalid entity or invalid entity component!");

        type States<'a> = (&'a ActionState, &'a MovementState);
        let state_view = world.view::<States>();

        // 플레이어 움직임 방향을 갱신합니다
        self.direction.update(self.input_bits, latlon);

        // 캐릭터 상태 데이터를 가져옵니다.
        let (&action_state, &movement_state) = state_view
            .get(entity)
            .expect("invalid entity or invalid entity component!");

        update_character_rotation(
            world,
            entity,
            archetype,
            action_state,
            movement_state,
            self.direction,
            latlon,
        );
    }

    fn update_player_character_translation(&mut self, elapsed_time_sec: f32) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        type Q<'a> = (
            &'a CharacterKind,
            &'a (Team, usize),
            &'a ActionState,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut HealthData,
            &'a mut CharacterFlags,
        );
        let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
        let (
            &character_kind,
            &(team, _),
            &action_state,
            movement_state,
            movement_state_timer,
            health_data,
            character_flags,
        ) = query.get().expect("invalid entity component!");

        // 캐릭터 속성 데이터를 가져옵니다.
        let i = character_kind as usize;
        let character_attributes = CHARACTER_ATTRIBUTES[i];

        // 캐릭터 위치를 가져옵니다.
        let mut transform = get_local_transform(world, entity, archetype);
        let mut translation = transform.get_translation();
        let mut is_grounded = character_flags.is_grounded();
        let mut is_invincible = character_flags.is_invincible();

        update_player_translation(
            &self.stage_attributes,
            character_attributes,
            action_state,
            movement_state,
            movement_state_timer,
            &mut self.velocity,
            &mut translation,
            self.direction,
            self.input_bits,
            team,
            &mut is_grounded,
            &mut is_invincible,
            health_data,
            self.input_timer,
            elapsed_time_sec,
        );

        // 캐릭터 위치를 설정합니다.
        character_flags.set_grounded(is_grounded);
        character_flags.set_invincible(is_invincible);
        transform.set_translation(translation.into());
        set_local_transform(world, entity, archetype, transform);
    }

    /// 다른 플레이어 캐릭터를 갱신합니다.
    fn update_other_characters(&mut self) {
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        type Q<'a> = (
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut LatLon,
        );
        let time_stamp_ms = self
            .max_game_play_time_ms
            .saturating_sub(self.remaining_time_ms);
        let mut state_view = world.view::<Q>();
        for (entity, archetype) in self.players.values().cloned() {
            let result = self.interpolation.get_interpolated(entity, time_stamp_ms);
            if let Some((
                transform,
                action_state,
                action_state_timer,
                movement_state,
                movement_state_timer,
                latlon,
            )) = result
            {
                let (
                    old_action_state,
                    old_action_state_timer,
                    old_movement_state,
                    old_movement_state_timer,
                    old_latlon,
                ) = state_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component!");
                *old_action_state = action_state;
                *old_action_state_timer = action_state_timer;
                *old_movement_state = movement_state;
                *old_movement_state_timer = movement_state_timer;
                *old_latlon = latlon;

                match archetype {
                    PlayerArchetype::Player0 => {
                        let mut query = world
                            .query_one::<&mut (Player0, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player1 => {
                        let mut query = world
                            .query_one::<&mut (Player1, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player2 => {
                        let mut query = world
                            .query_one::<&mut (Player2, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player3 => {
                        let mut query = world
                            .query_one::<&mut (Player3, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player4 => {
                        let mut query = world
                            .query_one::<&mut (Player4, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player5 => {
                        let mut query = world
                            .query_one::<&mut (Player5, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player6 => {
                        let mut query = world
                            .query_one::<&mut (Player6, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player7 => {
                        let mut query = world
                            .query_one::<&mut (Player7, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player8 => {
                        let mut query = world
                            .query_one::<&mut (Player8, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                    PlayerArchetype::Player9 => {
                        let mut query = world
                            .query_one::<&mut (Player9, ToParentTrans)>(entity)
                            .expect("invalid entity");
                        let (_, local_transform) = query.get().expect("invalid entity component!");
                        *local_transform = ToParentTrans(transform);
                    }
                }
            }
        }
    }

    /// 카메라 엔터티를 생성합니다.
    fn create_camera(&mut self, device: &wgpu::Device) {
        // 플레이어 캐릭터의 종류를 가져옵니다.
        let (entity, _archetype) = self.player_entity();
        let world = self.world.as_mut().expect("the world must be exists!");
        let character_kind = world
            .query_one_mut::<&CharacterKind>(entity)
            .cloned()
            .expect("invalid entity or invalid entity component!");

        let i = character_kind as usize;
        self.camera_fov_y = CAMERA_DEF_FOV_Y[i];
        self.camera_rel_position = CAMERA_DEF_REL_POS[i];

        // 카메라 컴포넌트 데이터를 생성합니다.
        let local_transform = ToParentTrans::default();
        let world_transform = WorldTransform::default();
        let projection =
            Projection::perspective(self.camera_fov_y, self.camera_aspect_ratio, 0.1, 200.0);
        let proj_view = projection.0 * world_transform.to_view_trans();
        let frustum = Frustum::from_mat4(proj_view);

        // 카메라 쉐이더 리소스를 생성합니다.
        let label = format!("InGameRunScene(Camera)");
        let camera_uniform = CameraUniform::uninit(Some(&label), device);
        let camera_resource = CameraResource::new(Some(&label), device, &camera_uniform);

        // 엔터티를 생성합니다.
        self.camera = world.spawn((
            (Camera, local_transform),
            (Camera, world_transform),
            projection,
            frustum,
            camera_uniform,
            camera_resource,
        ));
    }

    /// 카메라 파라미터 데이터를 갱신합니다.
    fn update_camera_param(&mut self) {
        let (entity, _archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        // 플레이어 엔터티의 컴포넌트를 가져옵니다.
        type Q<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a ViewState,
            &'a ViewStateTimer,
            &'a LatLon,
        );
        let (
            &character_kind,
            &action_state,
            &action_state_timer,
            &view_state,
            &view_state_timer,
            &latlon,
        ) = world
            .query_one_mut::<Q>(entity)
            .expect("invalid entity or invalid entity component!");

        // 카메라 파라미터를 갱신합니다.
        update_camera_param(
            &mut self.camera_rel_position,
            &mut self.camera_fov_y,
            character_kind,
            action_state,
            view_state,
            action_state_timer,
            view_state_timer,
        );

        // 카메라 변환 행렬을 생성합니다.
        let distance = self.camera_rel_position * glam::Vec3A::NEG_Z;
        let mut transform = glam::Mat4::from_translation(distance.into());
        let rotation = glam::Mat4::from_rotation_y(latlon.lon);
        transform = rotation * transform;

        let forward = glam::Vec3A::from_vec4(transform.z_axis);
        let forward = forward.normalize_or(glam::Vec3A::Z);
        let axis = glam::Vec3A::Y.cross(forward);
        let rotation = glam::Mat4::from_axis_angle(axis.into(), latlon.lat);
        transform = rotation * transform;

        let offset = self.camera_rel_position.with_z(0.0);
        let offset = glam::Mat4::from_translation(offset.into());
        transform = transform * offset;

        // 카메라의 로컬 변환 행렬, 투영 변환 행렬을 설정합니다.
        let ((_, local_transform), projection) = world
            .query_one_mut::<(&mut (Camera, ToParentTrans), &mut Projection)>(self.camera)
            .expect("invalid entity or invalid entity component!");
        local_transform.0 = transform;
        *projection =
            Projection::perspective(self.camera_fov_y, self.camera_aspect_ratio, 0.1, 200.0);
    }

    /// 카메라 변환 행렬을 갱신합니다.
    ///
    /// # Note
    /// 이 함수는 캐릭터의 월드 변환 행렬을 갱신한 후 호출되어야 합니다.
    ///
    fn update_camera_transform(&mut self) {
        // 플레이어 캐릭터의 월드 변환 행렬을 가져옵니다.
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        // 카메라 엔터티 계층 구조를 갱신합니다.
        let translation = get_world_transform(world, entity, archetype).get_translation();
        let parent = glam::Mat4::from_translation(translation.into());
        let entity = self.camera;
        update_camera_hierarchy(world, entity, parent);

        // 카메라 엔터티의 위치를 조정합니다.
        type Q<'a> = &'a mut (Camera, WorldTransform);
        let (_, world_transform) = world
            .query_one_mut::<Q>(entity)
            .expect("invalid entity or invalid entity component!");
    }

    /// Weighted-Blended OIT에 사용되는 렌더 타겟과 파이프라인을 생성합니다.
    fn create_weighted_blend_oit_resource(&mut self, size: WindowSize, device: &wgpu::Device) {
        if self.world.is_none() {
            return;
        }

        // 해상도의 크기를 가져옵니다.
        let (width, height): (u32, u32) = size.size().into();

        // 렌더 타겟을 생성합니다.
        let accum_render_target = AccumRenderTarget::new(width, height, device);
        let reveal_render_target = RevealRenderTarget::new(width, height, device);

        // 알파 블렌드 파이프라인을 생성합니다.
        let alpha_blend_pipeline = match self.alpha_blend_pipeline.take() {
            Some(pipeline) => pipeline.renew(device, &accum_render_target, &reveal_render_target),
            None => AlphaBlendPipeline::new(
                device,
                &accum_render_target,
                &reveal_render_target,
                SWAPCHAIN_FORMAT,
            ),
        };

        // 저장
        self.accum_render_target = Some(accum_render_target);
        self.reveal_render_target = Some(reveal_render_target);
        self.alpha_blend_pipeline = Some(alpha_blend_pipeline);
    }

    /// Bloom에 사용되는 렌더 타겟과 렌더/컴퓨트 파이프라인을 생성합니다.
    fn create_bloom_resource(&mut self, size: WindowSize, device: &wgpu::Device) {
        if self.world.is_none() {
            return;
        }

        // 해상도의 크기를 가져옵니다.
        let (width, height): (u32, u32) = size.size().into();

        // 렌더 타겟과 파이프라인을 생성합니다.
        let zip = self
            .gaussian_blur_pipeline
            .take()
            .zip(self.bloom_pipeline.take());
        let (gaussian_blur_pipeline, bright_render_target, bloom_pipeline) = match zip {
            Some((gaussian_blur_pipeline, bloom_pipeline)) => {
                gaussian_blur_pipeline.renew(width, height, device, bloom_pipeline)
            }
            None => GaussianBlurPipeline::new(width, height, device, SWAPCHAIN_FORMAT),
        };

        // 저장
        self.bright_render_target = Some(bright_render_target);
        self.gaussian_blur_pipeline = Some(gaussian_blur_pipeline);
        self.bloom_pipeline = Some(bloom_pipeline);
    }

    /// 서버로부터 전달받은 데이터로 플레이어를 갱신합니다.
    fn pull_world_data(&mut self, time_stamp: Instant, packet: InGamePullPacket) {
        // 게임 월드를 빌려옵니다.
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        type Always<'a> = (
            &'a CharacterKind,
            &'a mut Permission,
            &'a mut NetworkState,
            &'a mut HealthData,
            &'a mut BulletData,
            &'a mut SkillCostData,
            &'a mut CharacterFlags,
        );
        let mut always_change_view = world.view::<Always>();

        // 남은 시간을 계산합니다.
        let delta = Instant::now()
            .saturating_duration_since(time_stamp)
            .as_millis()
            .min(self.max_game_play_time_ms as u128) as u32;
        let server_remaining_time_ms = packet.remaining_time_ms.saturating_sub(delta);
        let server_time_stamp_ms = self
            .max_game_play_time_ms
            .saturating_sub(server_remaining_time_ms);

        // 플레이어 상태를 갱신합니다.
        for data in packet.players {
            let (entity, _archetype) = self
                .players
                .get(&data.uid)
                .cloned()
                .expect("the player data must be exists!");

            // 캐릭터 데이터를 가져옵니다.
            let (
                &character_kind,
                permission,
                network_state,
                health_data,
                bullet_data,
                skill_cost_data,
                flags,
            ) = always_change_view
                .get_mut(entity)
                .expect("invalid entity or invalid entity component!");

            // 서버 데이터를 저장합니다.
            *permission = data.permission();
            *network_state = data.network_state();
            flags.set_connected(data.is_connected());
            flags.set_invincible(data.is_invincible());
            health_data.shield = data.guard_health;
            health_data.remaining = data.current_health;
            bullet_data.remaining = data.current_bullet;
            skill_cost_data.remaining = data.current_skill_cost;

            if data.uid != self.uid {
                // 다른 플레이어 캐릭터의 스냅샷을 생성합니다.
                let snapshot = EntitySnapshot::new(
                    server_time_stamp_ms,
                    data.velocity,
                    data.rotation,
                    data.translation,
                    data.player_states.action_state(),
                    data.action_state_timer,
                    data.player_states.movement_state(),
                    data.movement_state_timer,
                    data.latlon,
                );

                match self.interpolation.buffers.get_mut(&entity) {
                    Some((_, buffers)) => {
                        buffers.insert(snapshot);
                    }
                    None => {
                        let mut buffers = SnapshotBuffer::new();
                        buffers.insert(snapshot);

                        self.interpolation
                            .buffers
                            .insert(entity, (character_kind, buffers));
                    }
                };
            } else {
                self.latest_snapshot = Some(EntitySnapshot::new(
                    server_time_stamp_ms,
                    data.velocity,
                    data.rotation,
                    data.translation,
                    data.player_states.action_state(),
                    data.action_state_timer,
                    data.player_states.movement_state(),
                    data.movement_state_timer,
                    data.latlon,
                ));
            }
        }

        let diff_t = (packet.remaining_time_ms as i32 - self.remaining_time_ms as i32).abs();
        if diff_t > 100 {
            self.remaining_time_ms = packet.remaining_time_ms;
        }
    }

    /// 플레이어 데이터를 보정합니다.
    fn correct_player_data(&mut self) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        type Q<'a> = (
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut LatLon,
        );
        let mut optional_change_view = world.view::<Q>();

        if let Some(snapshot) = self.latest_snapshot.as_ref() {
            // 플레이어 캐릭터 데이터를 가져옵니다.
            let (action_state, action_state_timer, movement_state, movement_state_timer, latlon) =
                optional_change_view
                    .get_mut(entity)
                    .expect("invalid entity or invalid entity component!");

            // 클라이언트의 시각을 계산합니다.
            let client_time_stamp = self
                .max_game_play_time_ms
                .saturating_sub(self.remaining_time_ms);

            let diff_t = (client_time_stamp as i32 - snapshot.time_stamp_ms as i32).abs();
            if *action_state == snapshot.action_state {
                // 차이가 큰 경우에만 서버 데이터를 덮어씁니다.
                if diff_t > 100 {
                    action_state_timer.0 = snapshot.action_state_timer.0;
                }
            } else {
                // 행동 상태가 다른 경우 서버 데이터를 덮어씁니다.
                *action_state = snapshot.action_state;
                action_state_timer.0 = snapshot.action_state_timer.0;
            }

            if *movement_state == snapshot.movement_state {
                // 차이가 큰 경우에만 서버 데이터를 덮어씁니다.
                if diff_t > 100 {
                    movement_state_timer.0 = snapshot.movement_state_timer.0;
                }
            } else {
                // 움직임 상태가 다른 경우 서버 데이터를 덮어씁니다.
                *movement_state = snapshot.movement_state;
                movement_state_timer.0 = snapshot.movement_state_timer.0;
            }

            // 카메라 방향 데이터를 덮어씁니다.
            let min = snapshot.latlon.lat - 3f32.to_radians().max(MIN_LATITUDE);
            let max = snapshot.latlon.lat + 3f32.to_radians().min(MAX_LATITUDE);
            latlon.lat = latlon.lat.clamp(min, max);
            let min = snapshot.latlon.lon - 5f32.to_radians();
            let max = snapshot.latlon.lon + 5f32.to_radians();
            latlon.lon = latlon.lon.clamp(min, max) % TAU;

            let p0 = get_local_transform(world, entity, archetype).get_translation();
            let p1 = snapshot.transform.w_axis.truncate().into();
            let d = p0.distance_squared(p1);
            let s = d.min(0.5) / 0.5;
            self.velocity.0 = self.velocity.0.lerp(snapshot.velocity.0, 0.5);
            lerp_local_transform(
                world,
                entity,
                archetype,
                ToParentTrans(snapshot.transform),
                s,
            );
        }
    }
}

impl GameScene for InGameRunScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let size = app.window_size();
        let device = app.render_device();

        let (width, height): (f32, f32) = size.size().into();
        self.camera_aspect_ratio = width / height;
        self.create_camera(device);
    }

    fn on_enter_background(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
        self.first_mouse_pressed = false;
    }

    fn on_window_resized(&mut self, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_some() {
            let size = app.window_size();
            let (width, height): (f32, f32) = size.size().into();
            self.camera_aspect_ratio = width / height;

            let device = app.render_device();
            self.create_weighted_blend_oit_resource(size, device);
            self.create_bloom_resource(size, device);
            // self.resize_ui(window, app);
        }
    }

    fn on_mouse_btn_pressed(
        &mut self,
        button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if !self.first_mouse_pressed {
            return true;
        }

        {
            let config = UserConfig::get();
            let flags = config
                .get_mouse_input(&button)
                .map(|input| input.into_bits())
                .unwrap_or_default();
            self.input_bits |= flags;
        }

        self.update_player_character_state();
        true
    }

    fn on_mouse_btn_released(
        &mut self,
        button: MouseButton,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !self.first_mouse_pressed {
            let event = AppEvent::CursorDisable;
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
            self.first_mouse_pressed = true;
            return true;
        }

        {
            let config = UserConfig::get();
            let flags = config
                .get_mouse_input(&button)
                .map(|input| input.into_bits())
                .unwrap_or_default();
            self.input_bits &= !flags;
        }

        self.update_player_character_state();
        true
    }

    fn on_cursor_moved(
        &mut self,
        _x: f32,
        _y: f32,
        mut dx: f32,
        mut dy: f32,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if !self.first_mouse_pressed {
            return true;
        }

        // 플레이어 카메라 방향을 가져옵니다.
        let (entity, _) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return true,
        };
        let latlon = world
            .query_one_mut::<&mut LatLon>(entity)
            .expect("invalid entity or invalid entity component!");

        // FOV-y 값에 따른 카메라 이동 속도 오프셋
        const MAX_CAM_FOV_Y: f32 = 90f32.to_radians();
        let s = self.camera_fov_y.min(MAX_CAM_FOV_Y) / MAX_CAM_FOV_Y;
        let offset = 0.05 + 0.95 * s;

        dx *= match self.flip_horizontal {
            true => -self.control_sensitivity,
            false => self.control_sensitivity,
        };

        dy *= match self.flip_vertical {
            true => -self.control_sensitivity,
            false => self.control_sensitivity,
        };

        let lat = latlon.lat + dy.to_radians() * offset;
        latlon.lat = lat.clamp(MIN_LATITUDE, MAX_LATITUDE);

        let lon = latlon.lon + dx.to_radians() * offset;
        latlon.lon = lon % TAU;

        true
    }

    fn on_keyboard_pressed(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if !repeat {
            {
                let config = UserConfig::get();
                let flags = config
                    .get_keyboard_input(&(code, location))
                    .map(|input| input.into_bits())
                    .unwrap_or_default();
                self.input_bits |= flags;
            }

            self.update_player_character_state();
        }

        true
    }

    fn on_keyboard_released(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if !repeat {
            {
                let config = UserConfig::get();
                let flags = config
                    .get_keyboard_input(&(code, location))
                    .map(|input| input.into_bits())
                    .unwrap_or_default();
                self.input_bits &= !flags;
            }

            self.update_player_character_state();
        }

        true
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(
        &mut self,
        time_stamp: Instant,
        packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::InGamePull => {
                let packet = InGamePullPacket::from_raw(packet);
                self.epoch = packet.epoch;
                self.epoch_time_stamp = Instant::now();
                self.pull_world_data(time_stamp, packet);
            }
            _ => {
                log::warn!(
                    "ignored >> invalid packet received! (TYPE:{:?})",
                    packet_type,
                );
            }
        }
        None
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        let elapsed_time_ms = (elapsed_time_sec * 1000.0) as u32;
        let now = Instant::now();
        let epoch_elapsed = now
            .saturating_duration_since(self.epoch_time_stamp)
            .as_millis()
            .min(u16::MAX as u128) as u16;
        if epoch_elapsed > 500 {
            return;
        }

        self.remaining_time_ms = self.remaining_time_ms.saturating_sub(elapsed_time_ms);
        let elapsed_time_ms = elapsed_time_ms.min(u16::MAX as u32) as u16;
        self.packet_elapsed_time_ms = self.packet_elapsed_time_ms.saturating_add(elapsed_time_ms);
        self.update_other_characters();
        self.update_player_character_timer(elapsed_time_ms);
        self.update_player_character_rotation();
        self.update_player_character_translation(elapsed_time_sec);
        self.update_player_character_state();
    }

    fn on_post_update(&mut self, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        const TICK: u16 = 11;
        if self.packet_elapsed_time_ms >= TICK {
            self.packet_elapsed_time_ms = 0;

            let net = app.net_manager();
            let socket = net.get(&SERVER_TCP_ADDR).unwrap();
            loop {
                let now = Instant::now();
                let elapsed_time_ms = now
                    .saturating_duration_since(self.epoch_time_stamp)
                    .as_millis()
                    .min(u16::MAX as u128) as u16;
                let count = self.histories.len().min(MAX_HISTORIES);
                let iter: std::vec::Drain<'_, StateHistory> = self.histories.drain(..count);
                let packet = InGamePushNotifyPacket::from_iter(
                    self.uid,
                    self.token,
                    self.epoch,
                    elapsed_time_ms,
                    self.input_bits,
                    iter,
                );
                socket.push_packet(packet.as_raw());

                if self.histories.is_empty() {
                    break;
                }
            }
        }
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 플레이어 데이터를 보정합니다.
        self.correct_player_data();

        // 변환 행렬을 갱신합니다.
        {
            let world = match self.world.as_ref() {
                Some(world) => world,
                None => return,
            };

            let child_view = &world.view::<&Child>();
            let sibling_view = &world.view::<&Sibling>();
            let flag_view = &world.view::<&CharacterFlags>();
            let weapon_view = &world.view::<WeaponQuery>();
            let animation_view = &world.view::<AnimationQuery>();
            let collection_view = &world.view::<&BoneCollection>();
            let motion_pool = &self.motion_pool;

            rayon::in_place_scope(|scope| {
                // 각 캐릭터의 애니메이션을 재생합니다.
                for (entity, archetype) in self.players.values().cloned() {
                    let flags = flag_view
                        .get(entity)
                        .expect("invalid entity or invalid entity component!");
                    if !flags.is_connected() {
                        continue;
                    }

                    scope.spawn(move |_| {
                        // 애니메이션을 재생합니다.
                        animate_character(
                            world,
                            entity,
                            archetype,
                            motion_pool,
                            animation_view,
                            &collection_view,
                        );

                        // 캐릭터 계층 구조를 갱신합니다.
                        update_character_hierarchy(
                            world,
                            entity,
                            archetype,
                            child_view,
                            sibling_view,
                            weapon_view,
                        );
                    });
                }
            });
        }

        // 카메라 파라미터를 갱신합니다.
        self.update_camera_param();
        self.update_camera_transform();

        let draw_tasks: &Arc<Queue<_>> = &Arc::new(Queue::new());
        let bake_tasks: &Arc<Queue<_>> = &Arc::new(Queue::new());
        {
            let device = app.render_device();
            let queue = app.render_queue();
            let world = self.world.as_ref().expect("the world must be exists!");
            let skybox = self.skybox.as_ref().expect("the skybox must be exists!");
            let hierarchy = self.stage.as_ref();
            let camera_entity = self.camera;

            let child_view = &world.view::<&Child>();
            let sibling_view = &world.view::<&Sibling>();
            let flag_view = &world.view::<&CharacterFlags>();
            let mesh_filter_view = &world.view::<MeshRenderer>();
            let skinned_mesh_filter_view = &world.view::<SkinnedMeshRenderer>();

            rayon::in_place_scope(|scope| {
                // 카메라 쉐이더 리소스와 스카이박스 쉐이더 리소스를 갱신합니다.
                scope.spawn(move |_| {
                    let mut staging_buffers = Vec::default();
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                    update_camera_and_skybox_resource(
                        world,
                        camera_entity,
                        skybox,
                        device,
                        &mut encoder,
                        &mut staging_buffers,
                    );

                    queue.submit(Some(encoder.finish()));
                    drop(staging_buffers);
                });

                // 캐릭터 엔터티의 쉐이더 리소스를 갱신합니다.
                for (entity, archetype) in self.players.values().cloned() {
                    let flags = flag_view
                        .get(entity)
                        .expect("invalid entity or invalid entity component!");
                    if !flags.is_connected() {
                        continue;
                    }

                    scope.spawn(move |_| {
                        let mut staging_buffers = Vec::default();
                        let mut encoder = device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        update_character_resource(
                            world,
                            entity,
                            archetype,
                            &device,
                            &mut encoder,
                            &mut staging_buffers,
                            child_view,
                            sibling_view,
                            mesh_filter_view,
                            skinned_mesh_filter_view,
                            draw_tasks,
                        );

                        queue.submit(Some(encoder.finish()));
                        drop(staging_buffers);
                    });
                }

                // 스테이지 엔터티 갱신
                if let Some(hierarchy) = hierarchy {
                    scope.spawn(move |_| {
                        // 스테이지 엔터티에 대해 카메라 뷰 프러스텀 컬링을 수행합니다.
                        type Q<'a> = (&'a (Camera, WorldTransform), &'a Projection);
                        let mut query =
                            world.query_one::<Q>(camera_entity).expect("invalid entity");
                        let ((_, world_transform), projection) = query
                            .get()
                            .expect("invalid entity or invalid entity component");
                        let frustum =
                            Frustum::from_mat4(projection.0 * world_transform.to_view_trans());
                        let entities = cull_stage_entities(&frustum, hierarchy);

                        // 컬링된 스테이지 엔터티의 계층 구조를 갱신합니다.
                        update_stage_hierarchy(world, &entities, child_view, sibling_view);

                        // 스테이지 쉐이더 리소스를 갱신합니다.
                        let mut staging_buffers = Vec::default();
                        let mut encoder = device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        update_stage_resource(
                            world,
                            &entities,
                            device,
                            &mut encoder,
                            &mut staging_buffers,
                            child_view,
                            sibling_view,
                            mesh_filter_view,
                            skinned_mesh_filter_view,
                            draw_tasks,
                        );

                        queue.submit(Some(encoder.finish()));
                        drop(staging_buffers);
                    });
                }
            });

            let light_resources = self
                .light_resource
                .as_ref()
                .expect("the light shader resource must be exists!");
            let direction_light = self.direction_light.as_ref();
            let player_entities: Vec<_> = self.players.values().cloned().collect();
            let (screen_width, screen_height): (f32, f32) = app.window_size().size().into();
            let aspect_ratio = screen_width / screen_height;
            let fov_y = self.camera_fov_y;

            rayon::in_place_scope(|scope| {
                // 방향성 조명의 쉐이더 리소스를 갱신합니다.
                if let Some(direction_light) = direction_light {
                    scope.spawn(move |_| {
                        type Q<'a> = &'a (Camera, WorldTransform);
                        let mut query =
                            world.query_one::<Q>(camera_entity).expect("invalid entity");
                        let (_, world_transform) = query.get().expect("invalid entity component!");

                        // 카메라의 뷰 프러스텀 모서리 위치를 계산합니다.
                        let frustum_corners = compute_frustum_corners_no_inverse(
                            world_transform,
                            fov_y,
                            aspect_ratio,
                            0.1,
                            15.0,
                        );

                        // 전역 조명 유니폼 버퍼를 갱신합니다.
                        let mut staging_buffers = Vec::default();
                        let mut encoder = device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        const MARGIN: f32 = 5.0;
                        let color = direction_light.color;
                        let light_dir = direction_light.direction_w;
                        let light_proj_view =
                            compute_light_view_proj_matrix(&frustum_corners, light_dir, MARGIN);
                        let data = GlobalLightDataLayout {
                            static_light_proj_view: direction_light.light_proj_view.to_cols_array(),
                            light_proj_view: light_proj_view.to_cols_array(),
                            direction_w: light_dir.to_array(),
                            color: color.to_array(),
                            intensity: 1.0,
                            ..Default::default()
                        };
                        light_resources.global_light_uniform.update(
                            device,
                            &mut encoder,
                            &mut staging_buffers,
                            data,
                        );

                        // 전역 조명 그림자 쉐이더 리소스를 갱신합니다.
                        let shadow_resource = light_resources.get_global();
                        let data = LightTransformDataLayout {
                            proj_view: light_proj_view.to_cols_array(),
                        };
                        shadow_resource.uniform.update(
                            device,
                            &mut encoder,
                            &mut staging_buffers,
                            data,
                        );

                        queue.submit(Some(encoder.finish()));
                        drop(staging_buffers);

                        // 조명이 비추는 영역과 교차하는 엔터티를 수집합니다.
                        let frustum = Frustum::from_mat4(light_proj_view);
                        let mut transform_resources = ShadowMap::default();
                        for (entity, archetype) in player_entities {
                            let flags = flag_view
                                .get(entity)
                                .expect("invalid entity or invalid entity component!");
                            if !flags.is_connected() {
                                continue;
                            }

                            collect_character_resource(
                                world,
                                entity,
                                archetype,
                                child_view,
                                sibling_view,
                                mesh_filter_view,
                                skinned_mesh_filter_view,
                                &mut transform_resources,
                            );
                        }

                        if let Some(hierarchy) = hierarchy {
                            let entities = cull_stage_entities(&frustum, hierarchy);
                            collect_stage_resource(
                                world,
                                &entities,
                                child_view,
                                sibling_view,
                                mesh_filter_view,
                                skinned_mesh_filter_view,
                                &mut transform_resources,
                            );
                        }

                        bake_tasks.push((shadow_resource.clone(), transform_resources));
                    });
                }
            });
        }

        while let Some(task) = draw_tasks.pop() {
            let RenderTask {
                mesh,
                mesh_resource,
                material_index,
                material_resource,
            } = task;

            let material_kind = material_resource.kind();
            if material_kind.is_opaque() {
                // 불투명 작업 집합에 추가합니다.
                let key = (mesh.clone(), material_kind);
                let sub_key = (material_index, material_resource.clone());
                match self.opaque_resources.get_mut(&key) {
                    Some(resource_map) => match resource_map.get_mut(&sub_key) {
                        Some(list) => {
                            list.push(mesh_resource.clone());
                        }
                        None => {
                            resource_map.insert(sub_key, vec![mesh_resource.clone()]);
                        }
                    },
                    None => {
                        self.opaque_resources.insert(
                            key,
                            HashMap::from_iter([(sub_key, vec![mesh_resource.clone()])]),
                        );
                    }
                }
            } else {
                // 투명 작업 집합에 추가합니다.
                let key = (mesh.clone(), material_kind);
                let sub_key = (material_index, material_resource.clone());
                match self.transparent_resources.get_mut(&key) {
                    Some(resource_map) => match resource_map.get_mut(&sub_key) {
                        Some(list) => {
                            list.push(mesh_resource);
                        }
                        None => {
                            resource_map.insert(sub_key, vec![mesh_resource]);
                        }
                    },
                    None => {
                        self.transparent_resources
                            .insert(key, HashMap::from_iter([(sub_key, vec![mesh_resource])]));
                    }
                }
            }
        }

        while let Some(task) = bake_tasks.pop() {
            self.bake_list.push(task);
        }
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        app: &dyn AppHandle,
    ) {
        let device = app.render_device();
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        // 카메라 쉐이더 리소스를 가져옵니다.
        let mut query = world
            .query_one::<&CameraResource>(self.camera)
            .expect("invalid entity!");
        let camera_resource = query.get().expect("invalid entity component!");

        // 쉐이더 리소스를 가져옵니다.
        let skybox = self.skybox.as_ref().expect("the skybox must be exists!");
        let light_resource = self
            .light_resource
            .as_ref()
            .expect("the light shader resource must be exists!");
        let accum_render_target = self
            .accum_render_target
            .as_ref()
            .expect("the accumulate render target must be exists!");
        let reveal_render_target = self
            .reveal_render_target
            .as_ref()
            .expect("the revealage render target must be exists!");
        let bright_render_target = self
            .bright_render_target
            .as_ref()
            .expect("the brightness render target must be exists!");
        let alpha_blend_pipeline = self
            .alpha_blend_pipeline
            .as_ref()
            .expect("the alpha blending render pipeline must be exists!");
        let gaussian_blur_pipeline = self
            .gaussian_blur_pipeline
            .as_ref()
            .expect("the gaussian blur compute pipeline must be exists!");
        let bloom_pipeline = self
            .bloom_pipeline
            .as_ref()
            .expect("the bloom render pipeline must be exists!");

        encoder.push_debug_group("shadow pass");
        for (shadow_resource, shadow_map) in self.bake_list.iter() {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(ShadowPass))"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_resource.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for ((mesh, kind), transform_resources) in shadow_map.iter() {
                match kind {
                    MaterialKind::Character => bake_character(
                        mesh,
                        device,
                        shadow_resource,
                        transform_resources,
                        &mut rpass,
                    ),
                    MaterialKind::CharacterEyeMouth => bake_character_eye_mouth(
                        mesh,
                        device,
                        shadow_resource,
                        transform_resources,
                        &mut rpass,
                    ),
                    MaterialKind::Stage | MaterialKind::Tree => bake_stage(
                        mesh,
                        device,
                        shadow_resource,
                        transform_resources,
                        &mut rpass,
                    ),
                    _ => {}
                };
            }
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("opaque pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(OpaquePass))"),
                color_attachments: &[
                    // 0번 렌더 타겟: 색상
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        view: render_target_view,
                        resolve_target: None,
                    }),
                    // 1번 렌더 타겟: bloom
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        view: bright_render_target.view(),
                        resolve_target: None,
                    }),
                ],
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

            for ((mesh, kind), material_resources) in self.opaque_resources.iter() {
                match kind {
                    MaterialKind::Character => {
                        draw_character(
                            mesh,
                            device,
                            camera_resource,
                            light_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::CharacterEyeMouth => {
                        draw_character_eye_mouth(
                            mesh,
                            device,
                            camera_resource,
                            light_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::CharacterHalo => {
                        draw_character_halo(
                            mesh,
                            device,
                            camera_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::Stage => {
                        draw_stage(
                            mesh,
                            device,
                            camera_resource,
                            light_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::Tree => {
                        draw_tree(
                            mesh,
                            device,
                            camera_resource,
                            light_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    _ => {}
                }
            }

            clear_render_target_with_skybox(&skybox, device, &mut rpass);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("transparent pass");
        {
            let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(TransparentPass))"),
                color_attachments: &[
                    // 0번 렌더 타겟: 누적 값
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear({
                                wgpu::Color {
                                    a: 0.0,
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                }
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        view: accum_render_target.view(),
                        resolve_target: None,
                    }),
                    // 1번 렌더 타겟: 노출 값
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear({
                                wgpu::Color {
                                    a: 1.0,
                                    r: 1.0,
                                    g: 1.0,
                                    b: 1.0,
                                }
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        view: reveal_render_target.view(),
                        resolve_target: None,
                    }),
                    // 2번 렌더 타겟: bloom
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        view: bright_render_target.view(),
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
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("compute pass");
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ComputePass(InGame)"),
                timestamp_writes: None,
            });
            gaussian_blur_pipeline.process(&mut cpass);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("composite pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(CompositePass))"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            alpha_blend_pipeline.process(&mut rpass);
            bloom_pipeline.process(&mut rpass);
        }
        encoder.pop_debug_group();
    }

    fn on_finish_draw(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.bake_list.clear();
        self.opaque_resources.clear();
        self.transparent_resources.clear();
    }
}
