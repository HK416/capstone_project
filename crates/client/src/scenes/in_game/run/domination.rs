use std::{collections::VecDeque, ptr::NonNull, sync::Arc};

use ahash::{HashMap, HashSet};
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::{NetManager, NetworkError},
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, Bullet, CapturePoint, CharacterKind, DamageLog, ExSkillCost,
        GameInputBits, GamePlayData, HealthPoint, LatLon, LoginToken, MovementState,
        MovementStateTimer, ObjectId, PlayPhasePlayer, RemainingBullet, Team, UserAccount, UserId,
        ViewState, ViewStateTimer, MAX_CAPTURE_SCORE, MAX_IN_GAME_PLAYERS,
    },
    protocol::{
        FinishStagePacket, Packet, PacketType, PullStagePacket, PushStatusPacket, RawPacket,
        UdpDamageLogPacket,
    },
};
use mod_physics::object3d::Frustum;
use winit::{
    event::{Modifiers, MouseButton},
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, TextureDataPool, TexturePool,
        TextureViewPool, DAMAGE_FONT_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR, SCHALE_ICON_URI,
        TIMER_ICON_URI, UI_GAME_LAYOUT_URI, WEAPON_ICON_MASK_URI, WEAPON_ICON_URI,
    },
    component::{
        animate_character, cleanup, set_weapon_position, spawn_bullet, update_character_direction,
        update_entity_hierarchy, update_third_person_camera, update_third_person_camera_hierarchy,
        update_view_state_by_controller_input_flags, update_view_state_timer, AttributeKind,
        BoneCollection, BulletRenderPipeline, CameraDataLayout, CameraResource, CameraUniform,
        CaptureZoneRenderPipeline, CharacterRenderPipeline, Child, DamageFontDataLayout,
        DamageFontRenderPipeline, DamageFontResource, DamageFontUniform, DamageParticle,
        EnergyBulletRenderPipeline, EyeMouthRenderPipeline, HaloRenderPipeline, MaterialKind,
        MaterialResource, MaterialUniform, Mesh, MeshFilter, MeshRenderer, MoveDirection,
        OpaqueMap, Parent, Projection, ShadowMap, ShadowResource, Sibling, SkinnedMeshRenderer,
        SkinningAnimation, Skybox, SkyboxDataLayout, SkyboxRenderPipeline, StageRenderPipeline,
        ThirdPersonCamera, ToParentTrans, TransformDataLayout, TransparentMap,
        WeightedBlendedOITRenderPipeline, WeightedBlendedOITResource, WorldTransform,
        NUM_CUBE_VERTICES,
    },
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, InGameResultEnterScene, BASE_WIDTH},
    SERVER_TCP_ADDR,
};

use super::{InGameDominationModeStatusLayer, InGamePauseLayer};

const UI_BG_COLOR: egui::Color32 = egui::Color32::from_black_alpha(128);

/// 팀의 색상입니다.
pub const TEAM_COLOR: [egui::Color32; 2] = [
    egui::Color32::from_rgb(0, 150, 255), // 블루팀 색상
    egui::Color32::from_rgb(255, 68, 51), // 레드 팀 색상
];

/// 플레이어 데이터입니다.
pub struct PlayerData {
    /// 플레이어의 접속 여부입니다.
    pub connected: bool,
    /// 사용자 계정 데이터입니다.
    pub account: UserAccount,
    /// 플레이어가 속한 팀입니다.
    pub team: Team,
    /// 플레이어가 속한 팀의 인덱스입니다.
    pub index: usize,
    /// 플레이어의 사망 여부입니다.
    pub alive: bool,
    /// 플레이어 캐릭터 종류입니다.
    pub character_kind: CharacterKind,
    /// 플레이어 캐릭터 체력입니다.
    pub health_point: HealthPoint,
}

/// 종합전술시험(점령전)을 진행하는 게임 장면입니다.
pub struct InGameDominationModeScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 사용자의 식별자입니다.
    user_id: UserId,
    /// 현재 사용자의 로그인 토큰입니다.
    token: LoginToken,
    /// 시야 조작 민감도입니다.
    control_sensitivity: f32,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 플레이어가 상태 대화상자를 보는 여부를 나타냅니다.
    show_status: bool,

    /// 게임 장면의 경과 시간입니다.
    /// 패킷을 보낼 때 사용됩니다.
    elapsed_time_sec: f32,
    /// 파티클의 타이머입니다.
    particle_timer: f32,

    /// 현재 게임 진행 상황입니다.
    capture_point: CapturePoint,
    /// 남은 게임 시간입니다.
    remaining_time_sec: f32,

    /// 엔터티를 관리하는 월드 객체입니다.
    world: Option<World>,
    /// 스카이박스입니다.
    skybox: Option<Skybox>,
    /// 메인 카메라 엔터티입니다.
    main_camera: Entity,
    /// 플레이어 엔터티 집합입니다.
    players: HashMap<UserId, Entity>,
    /// 연결이 끊어진 플레이어 엔터티 집합입니다.
    disconnected_players: Vec<Entity>,
    /// 오브젝트 엔터티 집합입니다.
    bullets: HashMap<ObjectId, Entity>,
    /// 지형 엔터티 집합입니다.
    stages: Vec<Entity>,

    /// 데미지 파티클 엔터티입니다.
    damage_particles: VecDeque<Entity>,

    /// 플레이어 움직임 방향입니다.
    move_direction: MoveDirection,
    /// 사용자 입력 상태 플래그 변수입니다.
    controller_input_flags: GameInputBits,

    /// 그림자 쉐이더 리소스입니다.
    shadow_resource: Option<ShadowResource>,
    /// 알파 블렌딩 쉐이더 리소스입니다.
    alpha_blend_resource: Option<WeightedBlendedOITResource>,

    /// 게임 인터페이스 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,

    /// 그림자 렌더링 리소스 집합입니다.
    shadow_map: ShadowMap,
    /// 불투명 메쉬 렌더링 리소스 집합입니다.
    opaque_map: OpaqueMap,
    /// 투명 메쉬 렌더링 리소스 집합입니다.
    transparent_map: TransparentMap,

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

//--------------------------------------------------------------------------------------------
// InGameDominationModeStatusLayer에서 사용되는 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 현재 상태창을 보고있는지 여부를 설정합니다.
    pub fn set_show_status(&mut self, show: bool) {
        self.show_status = show;
    }

    /// 주어진 uri에 해당하는 UI 텍스처를 가져옵니다.   
    /// 등록된 UI 텍스처가 없는 경우 `None`을 반환합니다.
    pub fn get_ui_texture<Uri>(&self, uri: Uri) -> Option<egui::load::SizedTexture>
    where
        Uri: AsRef<str>,
    {
        self.ui_textures.get(uri.as_ref()).cloned()
    }

    /// 플레이어 데이터를 반환합니다.  
    /// 첫 번째 요소는 블루 팀 플레이어, 두 번째 요소는 레드 팀 플레이어 집합입니다.
    pub fn get_player_data(&mut self) -> (Vec<PlayerData>, Vec<PlayerData>) {
        type Query<'a> = (
            &'a UserAccount,
            &'a ActionState,
            &'a (Team, usize),
            &'a CharacterKind,
            &'a HealthPoint,
        );

        // Safe: 상태 창 장면에서 게임 월드는 제거되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };
        let mut blue = Vec::with_capacity(MAX_IN_GAME_PLAYERS / 2);
        let mut red = Vec::with_capacity(MAX_IN_GAME_PLAYERS / 2);

        // 현재 접속 중인 플레이어를 대상으로 Blue팀과 Red팀으로 나눈다.
        for entity in self.players.values().cloned() {
            let (&account, &state, &(team, index), &character_kind, &health_point) = world
                .query_one_mut::<Query>(entity)
                .expect("invalid entity or invalid entity component");

            if team == Team::Blue {
                blue.push(PlayerData {
                    connected: true,
                    account,
                    team,
                    index,
                    alive: state != ActionState::Dead,
                    character_kind,
                    health_point,
                });
            } else {
                red.push(PlayerData {
                    connected: true,
                    account,
                    team,
                    index,
                    alive: state != ActionState::Dead,
                    character_kind,
                    health_point,
                });
            }
        }

        // 연결이 끊어진 플레이어를 대상으로 Blue팀과 Red팀으로 나눈다.
        for entity in self.disconnected_players.iter().cloned() {
            let (&account, &state, &(team, index), &character_kind, &health_point) = world
                .query_one_mut::<Query>(entity)
                .expect("invalid entity or invalid entity component");

            if team == Team::Blue {
                blue.push(PlayerData {
                    connected: false,
                    account,
                    team,
                    index,
                    alive: state != ActionState::Dead,
                    character_kind,
                    health_point,
                });
            } else {
                red.push(PlayerData {
                    connected: false,
                    account,
                    team,
                    index,
                    alive: state != ActionState::Dead,
                    character_kind,
                    health_point,
                });
            }
        }

        blue.sort_by_key(|it| it.index);
        red.sort_by_key(|it| it.index);

        (blue, red)
    }
}

//--------------------------------------------------------------------------------------------
// 초기화 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 새로운 `InGameDominationModeScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        world: World,
        skybox: Skybox,
        players: HashMap<UserId, Entity>,
        disconnected_players: Vec<Entity>,
        stages: Vec<Entity>,
        shadow_resource: ShadowResource,
        alpha_blend_resource: WeightedBlendedOITResource,
        ui_textures: HashMap<String, egui::load::SizedTexture>,
        mesh_pool: MeshPool,
        model_pool: ModelPool,
        motion_pool: MotionPool,
        texture_pool: TexturePool,
        texture_data_pool: TextureDataPool,
        texture_view_pool: TextureViewPool,
        sampler_pool: SamplerPool,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            control_sensitivity: 0.5,
            flip_horizontal: false,
            flip_vertical: false,
            show_status: false,
            elapsed_time_sec: 0.0,
            particle_timer: 0.0,
            capture_point: CapturePoint::default(),
            remaining_time_sec: 0.0,
            skybox: Some(skybox),
            world: Some(world),
            main_camera: Entity::DANGLING,
            players,
            disconnected_players,
            bullets: HashMap::default(),
            stages,
            damage_particles: VecDeque::default(),
            move_direction: MoveDirection::default(),
            controller_input_flags: GameInputBits::default(),
            shadow_resource: Some(shadow_resource),
            alpha_blend_resource: Some(alpha_blend_resource),
            ui_textures,
            shadow_map: HashMap::default(),
            opaque_map: HashMap::default(),
            transparent_map: HashMap::default(),
            mesh_pool,
            model_pool,
            motion_pool,
            texture_pool,
            texture_data_pool,
            texture_view_pool,
            sampler_pool,
        }
    }

    /// 진행도를 설정합니다.
    pub fn setup_progress(&mut self, capture_point: CapturePoint, remaining_time_sec: f32) {
        self.capture_point = capture_point;
        self.remaining_time_sec = remaining_time_sec;
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, device: &wgpu::Device) {
        // 플레이어 캐릭터 종류를 가져옵니다.
        let entity = self.get_player_entity();
        // Safe: 게임 월드는 `on_enter`를 호출한 시점에서 제거되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };
        let (&character_kind, &view_rotation) = world
            .query_one_mut::<(&CharacterKind, &LatLon)>(entity)
            .expect("invalid entity or invalid entity component");

        let third_person_camera = ThirdPersonCamera::new(character_kind, view_rotation);
        let camera_uniform = CameraUniform::uninit(Some("Main"), device);
        let camera_resource = CameraResource::new(Some("Main"), device, &camera_uniform);

        // 로컬 변환 행렬, 월드 변환 행렬, 투영 변환 행렬 컴포넌트를 추가합니다.
        let mut builder = EntityBuilder::new();
        builder.add_bundle((
            ToParentTrans::default(),
            WorldTransform::default(),
            Projection::perspective(75f32.to_radians(), 16.0 / 9.0, 0.01, 500.0),
            third_person_camera,
            camera_uniform,
            camera_resource,
            Frustum::from_mat4(glam::Mat4::IDENTITY),
        ));

        // 생성된 메인 카메라 엔터티를 저장합니다.
        self.main_camera = world.spawn(builder.build());
    }

    /// 데미지 파티클을 생성합니다.
    fn create_damage_particles(&mut self, device: &wgpu::Device, logs: Vec<DamageLog>) {
        // 데미지 파티클 메쉬를 가져옵니다.
        let (mesh, _) = self
            .mesh_pool
            .get(DAMAGE_FONT_URI)
            .expect("the damage particle mesh must exist!");

        // 데미지 폰트 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(DAMAGE_FONT_URI)
            .expect("the damage font texture must exist!");
        let view = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
        let sampler = self
            .sampler_pool
            .get_or_init(device, &wgpu::SamplerDescriptor::default());

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        for log in logs {
            // 플레이어 엔터티를 가져옵니다.
            let user_id = log.user_id;
            let entity = match self.players.get(&user_id).cloned() {
                Some(entity) => entity,
                None => continue,
            };

            // 플레이어 엔터티의 머리 노드 엔터티를 가져옵니다.
            let head = world
                .query_one_mut::<&SkinningAnimation>(entity)
                .expect("invalid entity or invalid entity component")
                .head;

            let str = log.damage.to_string();
            let length = str.trim().len() as f32;
            for (i, ch) in str.trim().chars().enumerate() {
                let number = ch.to_digit(10).expect("invalid data");

                // 파티클 위치를 계산합니다.
                const ORIGIN: f32 = -0.1;
                const WIDTH: f32 = 0.05;
                const HALF_WIDTH: f32 = WIDTH * 0.5;
                let x = ORIGIN - HALF_WIDTH * length + WIDTH * i as f32 + HALF_WIDTH;

                // 엔터티 요소를 생성합니다.
                let parent = Parent(head);
                let particle = DamageParticle {
                    elapsed_time_sec: 0.0,
                    duration_sec: 2.0,
                    begin_offset: glam::vec3a(x, 0.0, -0.6),
                    end_offset: glam::vec3a(x, 0.5, -0.4),
                    number,
                };
                let label = format!("DamageLog({})", user_id);
                let damage_uniform = DamageFontUniform::uninit(Some(&label), device);
                let damage_resource =
                    DamageFontResource::new(Some(&label), device, &view, &sampler, &damage_uniform);

                // 새로운 엔터티를 생성합니다.
                let entity = world.spawn((
                    mesh.clone(),
                    parent,
                    particle,
                    damage_uniform,
                    damage_resource,
                ));
                self.damage_particles.push_back(entity);
            }
        }
    }
}

//--------------------------------------------------------------------------------------------
// 플레이어 조작과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 플레이어 엔터티를 반환합니다.
    pub(super) fn get_player_entity(&self) -> Entity {
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
            .as_mut()
            .expect("the world must exist!")
            .query_one_mut::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 삼인칭 카메라의 방향을 기준으로 플레이어 움직임 방향을 갱신합니다.
        let controller = self.controller_input_flags.as_state();
        self.move_direction
            .update_from_third_person_camera(controller, third_person_camera);
    }

    /// 캐릭터가 바라보는 방향을 갱신합니다.
    fn update_character_direction(&mut self) {
        type Query<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a mut ToParentTrans,
        );
        let entity = self.get_player_entity();

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 삼인칭 카메라 요소를 가져옵니다.
        let mut query = world
            .query_one::<&ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity");
        let third_person_camera = query.get().expect("invalid entity component");

        // 행동 상태, 행동 상태 타이머, 움직임 상태, 로컬 변환 행렬 요소를 가져옵니다.
        let mut query = world.query_one::<Query>(entity).expect("invalid entity");
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
    fn update_view_state(&mut self) {
        type Query<'a> = (&'a CharacterKind, &'a mut ViewState, &'a mut ViewStateTimer);
        let entity = self.get_player_entity();

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 캐릭터 종류, 카메라 상태, 카메라 상태 타이머 요소를 가져옵니다.
        let (&character_kind, view_state, view_state_timer) = world
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

    /// 플레이어 카메라 상태 타이머를 갱신합니다.
    fn update_view_state_timer(&mut self, elapsed_time_sec: f32) {
        // 캐릭터 종류, 카메라 상태, 카메라 상태 타이머 요소를 가져옵니다.
        type Query<'a> = (&'a CharacterKind, &'a mut ViewState, &'a mut ViewStateTimer);
        let entity = self.get_player_entity();

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        let (&character_kind, view_state, view_state_timer) = world
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
// 네트워크 통신과 관련된 코드를 작성합니다.
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

        type Query<'a> = (&'a WorldTransform, &'a ViewState, &'a ViewStateTimer);
        let entity = self.get_player_entity();

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 플레이어 데이터를 수집합니다.
        let (world_transform, &view_state, &view_state_timer) = world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");
        let rotation = world_transform.get_rotation().to_array();
        let direction = self.move_direction.0.to_array();
        let input_flags = self.controller_input_flags;

        let third_person_camera = world
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
        let queue = app.render_queue();
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.capture_point = packet.capture_point;
        self.remaining_time_sec = packet.remaining_time_sec;

        self.update_player_from_pull_packet(&packet.players);
        self.update_bullet_from_pull_packet(
            &packet.bullets,
            device,
            &mut encoder,
            &mut staging_buffers,
        );

        queue.submit(Some(encoder.finish()));
        drop(staging_buffers);
    }

    /// 패킷 데이터로 플레이어를 갱신합니다.
    fn update_player_from_pull_packet<'a>(&mut self, players: &'a [PlayPhasePlayer]) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        type Query<'a> = (
            &'a mut GamePlayData,
            &'a mut HealthPoint,
            &'a mut RemainingBullet,
            &'a mut ExSkillCost,
            // &'a mut SkillKind,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
            &'a mut ViewState,
            &'a mut ViewStateTimer,
            &'a mut LatLon,
            &'a mut ToParentTrans,
        );
        let mut component_view = world.view_mut::<Query>();

        // 플레이어 데이터를 수정합니다.
        let mut removed = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for player in players {
            let user_id = player.account.uid;
            let entity = match self.players.get(&user_id).cloned() {
                Some(entity) => entity,
                None => continue,
            };

            // 플레이어가 연결 중이 아닌 경우 플레이어 집합에서 제거합니다.
            if !player.connected() {
                removed.push(user_id);
                continue;
            }

            let (
                play_data,
                health_point,
                remaining_bullet,
                ex_skill_cost,
                // skill_cool_time,
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

            *play_data = player.play_data;
            *remaining_bullet = player.remaining_bullet;
            *ex_skill_cost = player.ex_skill_cost;
            *health_point = player.health_point;
            *action_state = player.action_state();
            *action_state_timer = player.action_state_timer;
            *movement_state = player.movement_state();
            *movement_state_timer = player.movement_state_timer;

            if user_id == self.user_id {
                local_transform.set_translation(player.translation.into());
            } else {
                *view_state = player.view_state();
                *view_state_timer = player.view_state_timer;
                *view_rotation = player.view_rotation;
                local_transform.set_rotation_translation(
                    glam::Quat::from_array(player.rotation),
                    player.translation.into(),
                );
            }
        }
        drop(component_view);

        // 제거된 플레이어를 연결이 끊긴 플레이어 목록에 추가합니다.
        for user_id in removed {
            let entity = self.players.remove(&user_id).expect("no such entity");
            self.disconnected_players.push(entity);
        }
    }

    /// 패킷 데이터로 총알을 갱신합니다.
    fn update_bullet_from_pull_packet<'a>(
        &mut self,
        bullets: &'a [Bullet],
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        type Query<'a> = &'a mut ToParentTrans;
        let mut component_view = world.view::<Query>();

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
                    world,
                    &self.model_pool,
                    &self.texture_data_pool,
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
            world
                .insert(entity, builder.build())
                .expect("no such entity");
        }

        // 제거된 총알을 게임 월드에서 제거합니다.
        for id in ids {
            let entity = self.bullets.remove(&id).expect("no such entity");
            cleanup(world, entity);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 엔터티 계층 구조 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 카메라를 갱신합니다.
    fn update_camera(&mut self) {
        type Query<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a ViewState,
            &'a ViewStateTimer,
            &'a WorldTransform,
        );
        let entity = self.get_player_entity();

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 삼인칭 카메라 대상의 요소를 가져옵니다.
        let (
            &character_kind,
            &action_state,
            &action_state_timer,
            &view_state,
            &view_state_timer,
            world_transform,
        ) = world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");
        let target_pos = world_transform.get_translation();

        // 삼인칭 카메라 요소를 가져옵니다.
        let third_person_camera = world
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
        update_third_person_camera_hierarchy(world, self.main_camera, target_pos);
    }

    /// 캐릭터 애니메이션을 재생합니다.
    fn animate_character(&mut self) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        type Query<'a> = (
            &'a CharacterKind,
            &'a SkinningAnimation,
            &'a ActionState,
            &'a ActionStateTimer,
            &'a MovementState,
            &'a MovementStateTimer,
            &'a LatLon,
        );
        let element_view = world.view::<Query>();
        let collection_view = world.view::<&BoneCollection>();
        let mut transform_view = world.view::<&mut ToParentTrans>();

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
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        type Query<'a> = (&'a CharacterKind, &'a ActionState, &'a SkinningAnimation);
        let element_view = world.view::<Query>();
        let child_view = world.view::<&Child>();
        let sibling_view = world.view::<&Sibling>();
        let mut transform_view = world.view::<(&ToParentTrans, &mut WorldTransform)>();

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

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 캐릭터의 계층 구조를 갱신합니다.
        for entity in self.players.values().cloned() {
            update_entity_hierarchy(world, entity, glam::Mat4::IDENTITY);
        }

        self.update_character_weapon();
    }

    /// 총알 엔터티의 계층 구조를 갱신합니다.
    fn update_bullet(&mut self) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        for entity in self.bullets.values().cloned() {
            update_entity_hierarchy(world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 지형 엔터티의 계층 구조를 갱신합니다.
    fn update_stage(&mut self) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        for entity in self.stages.iter().cloned() {
            update_entity_hierarchy(world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 데미지 파티클을 갱신합니다.
    fn update_damage_particles(&mut self, elapsed_time_sec: f32) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 데미지 파티클을 갱신합니다.
        let mut damage_particles = VecDeque::with_capacity(self.damage_particles.len());
        while let Some(entity) = self.damage_particles.pop_front() {
            // 파티클의 요소를 가져옵니다.
            let particle = world
                .query_one_mut::<&mut DamageParticle>(entity)
                .expect("invalid entity or invalid entity component");

            // 파티클의 경과 시간을 갱신합니다.
            particle.elapsed_time_sec += elapsed_time_sec;

            // 파티클의 지속시간을 초과할 경우 엔터티를 제거합니다.
            if particle.elapsed_time_sec >= particle.duration_sec {
                let _ = world.despawn(entity);
                continue;
            }

            damage_particles.push_back(entity);
        }

        self.damage_particles = damage_particles;
    }
}

//--------------------------------------------------------------------------------------------
// 쉐이더 리소스 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 카메라 쉐이더 리소스를 갱신합니다.
    fn update_camera_and_skybox_resource(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        type Query<'a> = (
            &'a ThirdPersonCamera,
            &'a CameraUniform,
            &'a WorldTransform,
            &'a mut Projection,
            &'a mut Frustum,
        );

        let (third_person_camera, uniform, transform, projection, frustum) = world
            .query_one_mut::<Query>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 투영 변환 행렬을 갱신합니다.
        let fov_y_radians = third_person_camera.fov_y;
        *projection = Projection::perspective(fov_y_radians, 16.0 / 9.0, 0.01, 500.0);

        // 카메라 데이터 유니폼 버퍼를 갱신합니다.
        let position_w = transform.get_translation();
        let view = transform.to_view_trans();
        let proj_view = projection.0 * view;

        uniform.update(
            device,
            encoder,
            staging_buffers,
            CameraDataLayout {
                proj_view: proj_view.to_cols_array(),
                position_w: position_w.to_array(),
                ..Default::default()
            },
        );

        // 카메라 절두체를 갱신합니다.
        *frustum = Frustum::from_mat4(proj_view);

        // 스카이박스 데이터 유니폼 버퍼를 갱신합니다.
        let skybox = self.skybox.as_ref().unwrap();
        skybox.uniform.update(
            device,
            encoder,
            staging_buffers,
            SkyboxDataLayout {
                proj_view: proj_view.to_cols_array(),
                color: [1.0; 3],
                ..Default::default()
            },
        );
    }

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 캐릭터 엔터티를 수집합니다.
    fn culling_character(&self) -> Vec<Entity> {
        // FIXME: 현재는 모든 엔터티를 전부 렌더링함
        self.players.values().cloned().collect()
    }

    /// 캐릭터의 쉐이더 리소스를 갱신합니다.
    fn update_character_resource(
        &self,
        entity: Entity,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        shadow_map: &mut ShadowMap,
        opaque_map: &mut OpaqueMap,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        transform_view: &ViewBorrow<'_, &WorldTransform>,
        mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    ) {
        // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
        if let Some(child_entity) = child_view.get(entity).cloned() {
            self.update_character_resource(
                *child_entity,
                device,
                encoder,
                staging_buffers,
                shadow_map,
                opaque_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
        if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
            self.update_character_resource(
                *sibling_entity,
                device,
                encoder,
                staging_buffers,
                shadow_map,
                opaque_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        let result = mesh_filter_view.get(entity);
        if let Some((mesh, mesh_resource, uniform, materials)) = result {
            // 유니폼 버퍼를 갱신합니다.
            let transform = transform_view
                .get(entity)
                .expect("invalid entity component");
            uniform.update(
                device,
                encoder,
                staging_buffers,
                TransformDataLayout {
                    trans: transform.0.to_cols_array(),
                },
            );

            // 렌더 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                let key = (mesh.clone(), material.kind());
                let value = (
                    index,
                    MeshFilter::Mesh(mesh_resource.clone()),
                    material.clone(),
                );
                if let Some(resources) = opaque_map.get_mut(&key) {
                    resources.push(value);
                } else {
                    opaque_map.insert(key, vec![value]);
                }
            }

            // 그림자 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                if material.kind() == MaterialKind::Character
                    || material.kind() == MaterialKind::CharacterEyeMouth
                {
                    let key = (mesh.clone(), material.kind());
                    let value = (index, MeshFilter::Mesh(mesh_resource.clone()));
                    if let Some(resources) = shadow_map.get_mut(&key) {
                        resources.push(value);
                    } else {
                        shadow_map.insert(key, vec![value]);
                    }
                }
            }

            return;
        }

        let result = skinned_mesh_filter_view.get(entity);
        if let Some((mesh, mesh_resource, collection, uniform, materials)) = result {
            // 유니폼 버퍼를 갱신합니다.
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

            // 렌더 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                let key = (mesh.clone(), material.kind());
                let value = (
                    index,
                    MeshFilter::SkinnedMesh(mesh_resource.clone()),
                    material.clone(),
                );
                if let Some(resources) = opaque_map.get_mut(&key) {
                    resources.push(value);
                } else {
                    opaque_map.insert(key, vec![value]);
                }
            }

            // 그림자 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                if material.kind() == MaterialKind::Character
                    || material.kind() == MaterialKind::CharacterEyeMouth
                {
                    let key = (mesh.clone(), material.kind());
                    let value = (index, MeshFilter::SkinnedMesh(mesh_resource.clone()));
                    if let Some(resources) = shadow_map.get_mut(&key) {
                        resources.push(value);
                    } else {
                        shadow_map.insert(key, vec![value]);
                    }
                }
            }

            return;
        }
    }

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 총알 엔터티를 수집합니다.
    fn culling_bullets(&self) -> Vec<Entity> {
        // FIXME: 현재는 모든 엔터티를 전부 렌더링함
        self.bullets.values().cloned().collect()
    }

    /// 총알의 쉐이더 리소스를 갱신합니다.
    fn update_bullet_resource(
        &self,
        entity: Entity,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        shadow_map: &mut ShadowMap,
        opaque_map: &mut OpaqueMap,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        transform_view: &ViewBorrow<'_, &WorldTransform>,
        mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    ) {
        // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
        if let Some(child_entity) = child_view.get(entity).cloned() {
            self.update_bullet_resource(
                *child_entity,
                device,
                encoder,
                staging_buffers,
                shadow_map,
                opaque_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
        if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
            self.update_bullet_resource(
                *sibling_entity,
                device,
                encoder,
                staging_buffers,
                shadow_map,
                opaque_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        let result = mesh_filter_view.get(entity);
        if let Some((mesh, mesh_resource, uniform, materials)) = result {
            // 유니폼 버퍼를 갱신합니다.
            let transform = transform_view
                .get(entity)
                .expect("invalid entity component");
            uniform.update(
                device,
                encoder,
                staging_buffers,
                TransformDataLayout {
                    trans: transform.0.to_cols_array(),
                },
            );

            // 렌더 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                let key = (mesh.clone(), material.kind());
                let value = (
                    index,
                    MeshFilter::Mesh(mesh_resource.clone()),
                    material.clone(),
                );
                if let Some(resources) = opaque_map.get_mut(&key) {
                    resources.push(value);
                } else {
                    opaque_map.insert(key, vec![value]);
                }
            }

            // 그림자 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                if material.kind() == MaterialKind::Bullet {
                    let key = (mesh.clone(), material.kind());
                    let value = (index, MeshFilter::Mesh(mesh_resource.clone()));
                    if let Some(resources) = shadow_map.get_mut(&key) {
                        resources.push(value);
                    } else {
                        shadow_map.insert(key, vec![value]);
                    }
                }
            }

            return;
        }

        let result = skinned_mesh_filter_view.get(entity);
        if let Some((mesh, mesh_resource, collection, uniform, materials)) = result {
            // 유니폼 버퍼를 갱신합니다.
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

            // 렌더 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                let key = (mesh.clone(), material.kind());
                let value = (
                    index,
                    MeshFilter::SkinnedMesh(mesh_resource.clone()),
                    material.clone(),
                );
                if let Some(resources) = opaque_map.get_mut(&key) {
                    resources.push(value);
                } else {
                    opaque_map.insert(key, vec![value]);
                }
            }

            // 그림자 집합에 추가합니다.
            for (index, material) in materials.iter().enumerate() {
                if material.kind() == MaterialKind::Bullet {
                    let key = (mesh.clone(), material.kind());
                    let value = (index, MeshFilter::SkinnedMesh(mesh_resource.clone()));
                    if let Some(resources) = shadow_map.get_mut(&key) {
                        resources.push(value);
                    } else {
                        shadow_map.insert(key, vec![value]);
                    }
                }
            }

            return;
        }
    }

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 지형 엔터티를 수집합니다.
    fn culling_stages(&self) -> Vec<Entity> {
        // FIXME: 현재는 모든 엔터티를 전부 렌더링함
        self.stages.iter().cloned().collect()
    }

    /// 지형의 쉐이더 리소스를 갱신합니다.
    fn update_stage_resource(
        &self,
        entity: Entity,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        shadow_map: &mut ShadowMap,
        opaque_map: &mut OpaqueMap,
        transparent_map: &mut TransparentMap,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        transform_view: &ViewBorrow<'_, &WorldTransform>,
        mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
    ) {
        // 자식 엔터티가 존재하는 경우 자식 엔터티를 갱신합니다.
        if let Some(child_entity) = child_view.get(entity).cloned() {
            self.update_stage_resource(
                *child_entity,
                device,
                encoder,
                staging_buffers,
                shadow_map,
                opaque_map,
                transparent_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // 형제 엔터티가 존재하는 경우 형제 엔터티를 갱신합니다.
        if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
            self.update_stage_resource(
                *sibling_entity,
                device,
                encoder,
                staging_buffers,
                shadow_map,
                opaque_map,
                transparent_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        let result = mesh_filter_view.get(entity);
        if let Some((mesh, mesh_resource, uniform, materials)) = result {
            // 유니폼 버퍼를 갱신합니다.
            let transform = transform_view
                .get(entity)
                .expect("invalid entity component");
            uniform.update(
                device,
                encoder,
                staging_buffers,
                TransformDataLayout {
                    trans: transform.0.to_cols_array(),
                },
            );

            for (index, material) in materials.iter().enumerate() {
                match material.kind() {
                    MaterialKind::Stage => {
                        // 불투명 렌더 집합에 추가합니다.
                        let key = (mesh.clone(), material.kind());
                        let value = (
                            index,
                            MeshFilter::Mesh(mesh_resource.clone()),
                            material.clone(),
                        );
                        if let Some(resources) = opaque_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            opaque_map.insert(key, vec![value]);
                        }

                        // 그림자 집합에 추가합니다.
                        let key = (mesh.clone(), material.kind());
                        let value = (index, MeshFilter::Mesh(mesh_resource.clone()));
                        if let Some(resources) = shadow_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            shadow_map.insert(key, vec![value]);
                        }
                    }
                    MaterialKind::CaptureZone => {
                        // 투명 렌더 집합에 추가합니다.
                        let key = (mesh.clone(), material.kind());
                        let value = (
                            index,
                            MeshFilter::Mesh(mesh_resource.clone()),
                            material.clone(),
                        );
                        if let Some(resources) = transparent_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            transparent_map.insert(key, vec![value]);
                        }
                    }
                    _ => {}
                }
            }

            return;
        }

        let result = skinned_mesh_filter_view.get(entity);
        if let Some((mesh, mesh_resource, collection, uniform, materials)) = result {
            // 유니폼 버퍼를 갱신합니다.
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

            for (index, material) in materials.iter().enumerate() {
                match material.kind() {
                    MaterialKind::Stage => {
                        let key = (mesh.clone(), material.kind());
                        let value = (
                            index,
                            MeshFilter::SkinnedMesh(mesh_resource.clone()),
                            material.clone(),
                        );
                        // 불투명 렌더 집합에 추가합니다.
                        if let Some(resources) = opaque_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            opaque_map.insert(key, vec![value]);
                        }

                        let key = (mesh.clone(), material.kind());
                        let shadow_value = (index, MeshFilter::SkinnedMesh(mesh_resource.clone()));
                        // 그림자 집합에 추가합니다.
                        if let Some(resources) = shadow_map.get_mut(&key) {
                            resources.push(shadow_value);
                        } else {
                            shadow_map.insert(key, vec![shadow_value]);
                        }
                    }
                    MaterialKind::CaptureZone => {
                        let key = (mesh.clone(), material.kind());
                        let value = (
                            index,
                            MeshFilter::SkinnedMesh(mesh_resource.clone()),
                            material.clone(),
                        );

                        // 투명 렌더 집합에 추가합니다.
                        if let Some(resources) = transparent_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            transparent_map.insert(key, vec![value]);
                        }
                    }
                    _ => {}
                }
            }

            return;
        }
    }

    /// 데미지 파티클 쉐이더 리소스를 갱신합니다.
    fn update_damage_particle_resources(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_ref().unwrap_unchecked() };

        // 카메라의 위치를 가져옵니다.
        let mut query = world
            .query_one::<&WorldTransform>(self.main_camera)
            .expect("invalid entity");
        let camera_position = query
            .get()
            .expect("invalid entity component")
            .get_translation();

        for entity in self.damage_particles.iter().cloned() {
            // 파티클 요소를 가져옵니다.
            type Query<'a> = (&'a Parent, &'a DamageParticle, &'a DamageFontUniform);
            let mut query = world.query_one::<Query>(entity).expect("invalid entity");
            let (&parent, particle, uniform) = query.get().expect("invalid entity component");

            // 부모의 위치를 가져옵니다.
            let mut query = world
                .query_one::<&WorldTransform>(*parent)
                .expect("invalid entity");
            let head_position = query
                .get()
                .expect("invalid entity component")
                .get_translation();

            // 현재 파티클의 월드 변환 행렬을 계산합니다.
            let t = (particle.elapsed_time_sec / particle.duration_sec).min(1.0);
            let offset = particle.begin_offset * (1.0 - t) + particle.end_offset * t;
            let look = (head_position - camera_position).normalize_or(glam::Vec3A::Z);
            let right = glam::Vec3A::Y.cross(look);
            let up = look.cross(right);
            let position = offset.x * right + offset.y * up + offset.z * look + head_position;

            // 유니폼 버퍼를 갱신합니다.
            uniform.update(
                device,
                encoder,
                staging_buffers,
                DamageFontDataLayout {
                    trans: glam::mat4(
                        glam::vec4(right.x, right.y, right.z, 0.0),
                        glam::vec4(up.x, up.y, up.z, 0.0),
                        glam::vec4(look.x, look.y, look.z, 0.0),
                        glam::vec4(position.x, position.y, position.z, 1.0),
                    )
                    .to_cols_array(),
                    number: particle.number,
                    ..Default::default()
                },
            );
        }
    }

    /// 점령 지역의 재질 데이터 유니폼 버퍼를 갱신합니다.
    fn update_capture_zone_material_resource(
        &self,
        entity: Entity,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_ref().unwrap_unchecked() };

        // 점령 지역 재질 요소를 가져옵니다.
        let mut query = world
            .query_one::<&Vec<MaterialUniform>>(entity)
            .expect("invalid entity");
        if let Some(uniform_buffers) = query.get() {
            for uniform_buffer in uniform_buffers {
                match uniform_buffer {
                    MaterialUniform::CaptureZone { data, buffer } => {
                        let mut data = *data;
                        data.timer = self.particle_timer;
                        buffer.update(device, encoder, staging_buffers, data);
                    }
                    _ => {}
                }
            }
        }
    }
}

//--------------------------------------------------------------------------------------------
// 렌더링과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 캐릭터를 그립니다.
    fn draw_character<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);

        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
        rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }

    /// 캐릭터의 눈과 입을 그립니다.
    fn draw_character_eye_mouth<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);

        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
        rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }

    /// 캐릭터의 헤일로를 그립니다.
    fn draw_character_halo<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);

        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = &mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }

    /// 총알을 그립니다.
    fn draw_bullet<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);

        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = &mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }

    /// 에너지 볼 형태의 총알을 그립니다.
    fn draw_energy_bullet<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);

        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = &mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }

    /// 지형을 그립니다.
    fn draw_stage<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);

        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }

    /// 스카이박스로 렌더 타겟을 초기화합니다.
    ///
    /// # Note
    /// 이 함수는 그리기 마지막에 호출하는 것이 가장 성능이 좋습니다.
    ///
    fn clear_render_target_with_skybox<'a>(
        skybox: &'a Skybox,
        pipeline: Arc<wgpu::RenderPipeline>,
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);
        rpass.set_vertex_buffer(0, skybox.vertex.slice(..));
        rpass.set_bind_group(0, skybox.resource.bind_group(), &[]);
        rpass.draw(0..NUM_CUBE_VERTICES as u32, 0..1);
    }

    /// 데미지 파티클을 그립니다.
    fn draw_damage_particle<'a>(
        &'a self,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_ref().unwrap_unchecked() };

        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);

        type Query<'a> = (&'a Arc<Mesh>, &'a DamageFontResource);
        for entity in self.damage_particles.iter().cloned() {
            let mut query = world.query_one::<Query>(entity).expect("invalid entity");
            let (mesh, resource) = query.get().expect("invalid entity component");
            rpass.set_bind_group(1, resource.bind_group(), &[]);
            rpass.set_vertex_buffer(0, mesh.vertex(..));
            rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
            rpass.draw(0..mesh.num_vertices(), 0..1);
        }
    }

    /// 점령 지역을 그립니다.
    fn draw_capture_zone<'a>(
        mesh: &'a Mesh,
        pipeline: Arc<wgpu::RenderPipeline>,
        camera_resource: &'a CameraResource,
        material_resources: &'a [(usize, MeshFilter, MaterialResource)],
        rpass: &mut wgpu::RenderPass<'a>,
    ) {
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, camera_resource.bind_group(), &[]);
        rpass.set_vertex_buffer(0, mesh.vertex(..));

        for (index, mesh_resource, material) in material_resources {
            let index_buffer = mesh.submeshes().get(*index).unwrap();
            rpass.set_index_buffer(index_buffer.slice(..), index_buffer.format());
            rpass.set_bind_group(1, mesh_resource.bind_group(), &[]);
            rpass.set_bind_group(2, material.bind_group(), &[]);
            rpass.draw_indexed(0..index_buffer.count(), 0, 0..1);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 사용자 인터페이스와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 체력 인터페이스 배경 레이아웃이미지입니다.
    fn health_point_bg_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // 체력 인터페이스 레이아웃 이미지
        // - 기준 가로 크기: 280
        // - 기준 세로 크기: 94
        // - 기준 시작 위치: (30, 596)
        // - 기준 종료 위치: (310, 690)
        //
        let ui_game_layout = self
            .ui_textures
            .get(UI_GAME_LAYOUT_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        let tex_width = ui_game_layout.size.x;
        let tex_height = ui_game_layout.size.y;
        let src_front = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_front = egui::Rect::from_min_max(
            egui::pos2(30.0 * scale, 596.0 * scale),
            egui::pos2(63.0 * scale, 690.0 * scale),
        );
        let uv_front = egui::Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(0.59375, 1.0));

        let src_middle = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.1875, tex_height),
            id: ui_game_layout.id,
        };
        let pos_middle = egui::Rect::from_min_max(
            egui::pos2(63.0 * scale, 596.0 * scale),
            egui::pos2(277.0 * scale, 690.0 * scale),
        );
        let uv_middle =
            egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(0.40625, 1.0));

        let src_back = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_back = egui::Rect::from_min_max(
            egui::pos2(277.0 * scale, 596.0 * scale),
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

        egui::Area::new(egui::Id::new("Health_BG_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(src_front)
                .uv(uv_front)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_front);
            egui::Image::new(src_middle)
                .uv(uv_middle)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_middle);
            egui::Image::new(src_back)
                .uv(uv_back)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_back);

            egui::Image::new(ui_game_layout)
                .uv(deco_uv)
                .paint_at(ui, deco_pos);
        });
    }

    /// 체력 게이지 인터페이스 레이아웃입니다.
    fn health_point_gauge_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let entity = self.get_player_entity();
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 폰트 속성
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());

        // 체력 텍스트
        let &health_point = world
            .query_one_mut::<&HealthPoint>(entity)
            .expect("invalid entity or invalid entity component");
        let percent = health_point.percent();

        let text = format!("{}", health_point.current.min(9999));
        let font_id = egui::FontId::new(28.0 * scale, main_font_family.clone());
        let health_point_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        egui::Area::new(egui::Id::new("Health_Gauge_Layout")).show(egui_ctx, |ui| {
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
            .show(egui_ctx, |ui| {
                ui.set_width(128.0 * scale);
                ui.label(health_point_text).interact(egui::Sense::empty())
            });
    }

    /// 팀 점수 게이지 인터페이스 레이아웃입니다.
    fn score_gauge_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // schale 아이콘
        let schale_icon = self
            .ui_textures
            .get(SCHALE_ICON_URI)
            .cloned()
            .expect("the Schale_Icon must exist!");
        let icon_area = egui::Rect::from_min_max(
            egui::pos2(610.0 * scale, 24.0 * scale),
            egui::pos2(670.0 * scale, 84.0 * scale),
        );

        // 전체 배경
        // - 가로 기준 길이: 520
        // - 세로 기준 길이: 12
        //
        let rect = egui::Rect::from_min_max(
            egui::pos2(380.0 * scale, 48.0 * scale),
            egui::pos2(900.0 * scale, 60.0 * scale),
        );
        let frame_bg = egui::epaint::RectShape::new(
            rect,
            16.0,
            egui::Color32::WHITE,
            egui::Stroke::NONE,
            egui::StrokeKind::Middle,
        );

        // 플레이어 팀 데코
        let team_bg_deco = egui::epaint::CircleShape::stroke(
            egui::pos2(640.0 * scale, 54.0 * scale),
            40.0 * scale,
            egui::Stroke::new(3.0 * scale, egui::Color32::WHITE),
        );
        let team_bg_shadow = egui::epaint::CircleShape::filled(
            egui::pos2(640.0 * scale, 54.0 * scale),
            33.0 * scale,
            egui::Color32::from_black_alpha(192),
        );

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let entity = self.get_player_entity();
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 플레이어 팀 배경
        let (team, _) = world
            .query_one_mut::<&(Team, usize)>(entity)
            .cloned()
            .expect("invalid entity or invalid entity component");
        let team_bg = egui::epaint::CircleShape::filled(
            egui::pos2(640.0 * scale, 54.0 * scale),
            32.0 * scale,
            TEAM_COLOR[team as usize],
        );

        // 블루 팀 게이지 배경
        // - 가로 기준 길이: 200
        // - 세로 기준 길이: 8
        //
        let rect = egui::Rect::from_min_max(
            egui::pos2(382.0 * scale, 50.0 * scale),
            egui::pos2(582.0 * scale, 58.0 * scale),
        );
        let blue_guage_bg = egui::epaint::RectShape::new(
            rect,
            16.0,
            egui::Color32::DARK_GRAY,
            egui::Stroke::NONE,
            egui::StrokeKind::Middle,
        );

        // 레드 팀 게이지 배경
        // - 가로 기준 길이: 200
        // - 세로 기준 길이: 8
        //
        let rect = egui::Rect::from_min_max(
            egui::pos2(698.0 * scale, 50.0 * scale),
            egui::pos2(898.0 * scale, 58.0 * scale),
        );
        let red_guage_bg = egui::epaint::RectShape::new(
            rect,
            16.0,
            egui::Color32::DARK_GRAY,
            egui::Stroke::NONE,
            egui::StrokeKind::Middle,
        );

        // 블루 팀 게이지
        let score = self.capture_point.capture_score[Team::Blue as usize];
        let percent = (score / MAX_CAPTURE_SCORE * 100.0).floor() / 100.0;
        let width = 200.0 * scale * percent;
        let rect = egui::Rect::from_min_max(
            egui::pos2(582.0 * scale - width, 50.0 * scale),
            egui::pos2(582.0 * scale, 58.0 * scale),
        );
        let blue_guage = egui::epaint::RectShape::new(
            rect,
            16.0,
            TEAM_COLOR[Team::Blue as usize],
            egui::Stroke::NONE,
            egui::StrokeKind::Middle,
        );

        // 레드 팀 게이지
        let score = self.capture_point.capture_score[Team::Red as usize];
        let percent = (score / MAX_CAPTURE_SCORE * 100.0).floor() / 100.0;
        let width = 200.0 * scale * percent;
        let rect = egui::Rect::from_min_max(
            egui::pos2(698.0 * scale, 50.0 * scale),
            egui::pos2(698.0 * scale + width, 58.0 * scale),
        );
        let red_guage = egui::epaint::RectShape::new(
            rect,
            16.0,
            TEAM_COLOR[Team::Red as usize],
            egui::Stroke::NONE,
            egui::StrokeKind::Middle,
        );

        // 점령도 게이지
        let progress_guage = match self.capture_point.capture_team {
            Some(team) => {
                const GUAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
                const GUAGE_BLUR_COLOR: egui::Color32 = egui::Color32::from_rgb(218, 247, 166);
                let percent = self.capture_point.capture_progress.floor() / 100.0;
                let width = 204.0 * scale * percent;
                match team {
                    Team::Blue => {
                        let rect = egui::Rect::from_min_max(
                            egui::pos2(380.0 * scale, 48.0 * scale),
                            egui::pos2(380.0 * scale + width, 60.0 * scale),
                        );
                        Some(egui::epaint::RectShape::new(
                            rect,
                            16.0,
                            GUAGE_COLOR,
                            egui::Stroke::new(1.0 * scale, GUAGE_BLUR_COLOR),
                            egui::StrokeKind::Middle,
                        ))
                    }
                    Team::Red => {
                        let rect = egui::Rect::from_min_max(
                            egui::pos2(900.0 * scale - width, 48.0 * scale),
                            egui::pos2(900.0 * scale, 60.0 * scale),
                        );
                        Some(egui::epaint::RectShape::new(
                            rect,
                            16.0,
                            GUAGE_COLOR,
                            egui::Stroke::new(1.0 * scale, GUAGE_BLUR_COLOR),
                            egui::StrokeKind::Middle,
                        ))
                    }
                }
            }
            None => None,
        };

        egui::Area::new(egui::Id::new("Score_Gauge_Layout")).show(egui_ctx, |ui| {
            ui.painter().add(frame_bg);
            ui.painter().add(team_bg_deco);
            ui.painter().add(team_bg_shadow);
            ui.painter().add(team_bg);
            egui::Image::new(schale_icon)
                .tint(egui::Color32::from_white_alpha(128))
                .paint_at(ui, icon_area);
            if let Some(progress_guage) = progress_guage {
                ui.painter().add(progress_guage);
            }
            ui.painter().add(blue_guage_bg);
            ui.painter().add(blue_guage);
            ui.painter().add(red_guage_bg);
            ui.painter().add(red_guage);
        });
    }

    /// 남은 시간 인터페이스 레이아웃입니다.
    fn remaining_timer_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // 남은 시간 인터페이스 레이아웃
        // - 기준 가로 크기: 144
        // - 기준 세로 크기: 36
        // - 기준 시작 위치: (1144, 12)
        // - 기준 종료 위치: (1264, 42)
        //
        let ui_game_layout = self
            .ui_textures
            .get(UI_GAME_LAYOUT_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        let ui_timer_icon = self
            .ui_textures
            .get(TIMER_ICON_URI)
            .cloned()
            .expect("the UI_Timer_Icon must exist!");

        let tex_width = ui_game_layout.size.x;
        let tex_height = ui_game_layout.size.y;
        let src_front = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_front = egui::Rect::from_min_max(
            egui::pos2(1120.0 * scale, 12.0 * scale),
            egui::pos2(1133.0 * scale, 48.0 * scale),
        );
        let uv_front = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.40625, 1.0));

        let src_middle = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.1875, tex_height),
            id: ui_game_layout.id,
        };
        let pos_middle = egui::Rect::from_min_max(
            egui::pos2(1133.0 * scale, 12.0 * scale),
            egui::pos2(1253.0 * scale, 48.0 * scale),
        );
        let uv_middle =
            egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.59375, 1.0));

        let src_back = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_back = egui::Rect::from_min_max(
            egui::pos2(1253.0 * scale, 12.0 * scale),
            egui::pos2(1264.0 * scale, 48.0 * scale),
        );
        let uv_back = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(1.0, 1.0));

        // 타이머 아이콘
        let timer_icon_rect = egui::Rect::from_min_max(
            egui::pos2(1140.0 * scale, 16.0 * scale),
            egui::pos2(1168.0 * scale, 44.0 * scale),
        );

        // 남은 시간 폰트
        let font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(18.0 * scale, font_family);
        let minute = (self.remaining_time_sec / 60.0).floor();
        let seconds = (self.remaining_time_sec % 60.0).floor();
        let text = format!("{:0>2}:{:0>2}", minute, seconds);
        let remaining_time_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let text_area_rect = egui::Rect::from_min_max(
            egui::pos2(1164.0 * scale, 14.0 * scale),
            egui::pos2(1252.0 * scale, 46.0 * scale),
        );

        egui::Area::new(egui::Id::new("Timer_BG_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(src_front)
                .uv(uv_front)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_front);
            egui::Image::new(src_middle)
                .uv(uv_middle)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_middle);
            egui::Image::new(src_back)
                .uv(uv_back)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_back);

            egui::Image::new(ui_timer_icon).paint_at(ui, timer_icon_rect);

            ui.put(text_area_rect, egui::Label::new(remaining_time_text));
        });
    }

    // 남은 총알 인터페이스 레이아웃입니다.
    fn remaining_bullet_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // 총알 인터페이스 레이아웃
        // - 기준 가로 크기: 220
        // - 기준 세로 크기: 110
        // - 기준 시작 위치: (1040, 580)
        // - 기준 종료 위치: (1250, 690)
        //
        let ui_game_layout = self
            .ui_textures
            .get(UI_GAME_LAYOUT_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        let icon = self
            .ui_textures
            .get(WEAPON_ICON_MASK_URI)
            .cloned()
            .expect("the Weapon Icon must exist!");

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let entity = self.get_player_entity();
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 남은 총알 텍스트
        let remaining_bullet = world
            .query_one_mut::<&RemainingBullet>(entity)
            .expect("invalid entity or invalid entity component");
        let text = format!("{}/{}", remaining_bullet.current, remaining_bullet.maximum);
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(18.0 * scale, family);
        let remaining_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let text_area = egui::Rect::from_min_max(
            egui::pos2(1040.0 * scale, 650.0 * scale),
            egui::pos2(1240.0 * scale, 670.0 * scale),
        );

        // 인터페이스 배경
        let tex_width = ui_game_layout.size.x;
        let tex_height = ui_game_layout.size.y;
        let src_front = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_front = egui::Rect::from_min_max(
            egui::pos2(1030.0 * scale, 580.0 * scale),
            egui::pos2(1063.0 * scale, 690.0 * scale),
        );
        let uv_front = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.40625, 1.0));

        let src_middle = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.1875, tex_height),
            id: ui_game_layout.id,
        };
        let pos_middle = egui::Rect::from_min_max(
            egui::pos2(1063.0 * scale, 580.0 * scale),
            egui::pos2(1217.0 * scale, 690.0 * scale),
        );
        let uv_middle =
            egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.59375, 1.0));

        let src_back = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_back = egui::Rect::from_min_max(
            egui::pos2(1217.0 * scale, 580.0 * scale),
            egui::pos2(1250.0 * scale, 690.0 * scale),
        );
        let uv_back = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(1.0, 1.0));

        // 무기 아이콘
        // 가로 길이: 200
        let ratio = icon.size.x / icon.size.y;
        let icon_width = 200.0;
        let icon_height = icon_width / ratio;
        let icon_area = egui::Rect::from_min_max(
            egui::pos2(1040.0 * scale, 590.0 * scale),
            egui::pos2(1240.0 * scale, (590.0 + icon_height) * scale),
        );

        egui::Area::new(egui::Id::new("Bullet_BG_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(src_front)
                .uv(uv_front)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_front);
            egui::Image::new(src_middle)
                .uv(uv_middle)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_middle);
            egui::Image::new(src_back)
                .uv(uv_back)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_back);

            egui::Image::new(icon)
                .tint(egui::Color32::DARK_GRAY)
                .paint_at(ui, icon_area);

            ui.put(text_area, egui::Label::new(remaining_text));
        });
    }

    /// 일반 스킬 인터페이스 레이아웃입니다.
    fn skill_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // 일반스킬 인터페이스 레이아웃
        // - 기준 가로 크기: 74
        // - 기준 세로 크기: 74
        // - 기준 시작 위치: (940, 612)
        // - 기준 종료 위치: (1032, 690)
        //
        let ui_game_layout = self
            .ui_textures
            .get(UI_GAME_LAYOUT_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        // 인터페이스 배경
        let tex_width = ui_game_layout.size.x;
        let tex_height = ui_game_layout.size.y;
        let src_front = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_front = egui::Rect::from_min_max(
            egui::pos2(940.0 * scale, 612.0 * scale),
            egui::pos2(962.0 * scale, 690.0 * scale),
        );
        let uv_front = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.40625, 1.0));

        let src_middle = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.1875, tex_height),
            id: ui_game_layout.id,
        };
        let pos_middle = egui::Rect::from_min_max(
            egui::pos2(962.0 * scale, 612.0 * scale),
            egui::pos2(1010.0 * scale, 690.0 * scale),
        );
        let uv_middle =
            egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.59375, 1.0));

        let src_back = egui::load::SizedTexture {
            size: egui::vec2(tex_width * 0.40625, tex_height),
            id: ui_game_layout.id,
        };
        let pos_back = egui::Rect::from_min_max(
            egui::pos2(1010.0 * scale, 612.0 * scale),
            egui::pos2(1032.0 * scale, 690.0 * scale),
        );
        let uv_back = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(1.0, 1.0));

        egui::Area::new(egui::Id::new("Skill_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(src_front)
                .uv(uv_front)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_front);
            egui::Image::new(src_middle)
                .uv(uv_middle)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_middle);
            egui::Image::new(src_back)
                .uv(uv_back)
                .tint(UI_BG_COLOR)
                .paint_at(ui, pos_back);
        });
    }

    /// Ex 스킬 게이지 인터페이스 레이아웃입니다.
    fn ex_skill_guage_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        let icon = self
            .ui_textures
            .get(WEAPON_ICON_URI)
            .cloned()
            .expect("the Weapon Icon must exist!");

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let entity = self.get_player_entity();
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 현재 Ex스킬 코스트를 가져옵니다.
        let ex_skill_cost = world
            .query_one_mut::<&ExSkillCost>(entity)
            .expect("invalid entity or invalid entity component");
        let percent = ex_skill_cost.percent();

        // 무기 아이콘
        // 가로 길이: 200
        let ratio = icon.size.x / icon.size.y;
        let icon_width = 200.0;
        let icon_height = icon_width / ratio;
        let icon_area = egui::Rect::from_min_max(
            egui::pos2(1040.0 * scale, 590.0 * scale),
            egui::pos2(
                (1040.0 + icon_width * percent) * scale,
                (590.0 + icon_height) * scale,
            ),
        );
        let icon_uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(percent, 1.0));

        egui::Area::new(egui::Id::new("ExSkill_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(icon).uv(icon_uv).paint_at(ui, icon_area);
        });
    }
}

//--------------------------------------------------------------------------------------------
// 시스템 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModeScene {
    /// 마우스 커서를 활성화합니다.
    fn enable_cursor(&self, window: &Window) {
        #[cfg(not(target_os = "windows"))]
        {
            use winit::window::CursorGrabMode;
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
        }
        #[cfg(target_os = "windows")]
        {
            use mod_app::ext::AppWindowExt;
            window.confine_cursor_to_window(false);
        }

        window.set_cursor_visible(true);
    }

    /// 마우스 커서를 비활성화합니다.
    fn disable_cursor(&self, window: &Window) {
        #[cfg(not(target_os = "windows"))]
        {
            use winit::window::CursorGrabMode;
            window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
        }
        #[cfg(target_os = "windows")]
        {
            use mod_app::ext::AppWindowExt;
            window.confine_cursor_to_window(true);
        }

        window.set_cursor_visible(false);
    }

    /// 커서 위치를 애플리케이션 창 중앙으로 초기화합니다.
    #[allow(unused_variables)]
    fn reset_cursor_position_at_center(&self, window: &Window) {
        #[cfg(target_os = "windows")]
        {
            use winit::dpi::PhysicalPosition;
            let (width, height): (u32, u32) = window.inner_size().into();
            let position = PhysicalPosition::new(width / 2, height / 2);
            window.set_cursor_position(position).unwrap();
        }
    }
}

//--------------------------------------------------------------------------------------------

impl GameScene for InGameDominationModeScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
        let device = app.render_device();
        self.create_main_camera(&device);
        self.update_stage(); // 정적인 지형은 매번 계층 구조를 갱신할 필요가 없다.
    }

    fn on_enter_foreground(&mut self, app: &dyn AppHandle) {
        if let Some(window) = app.window() {
            self.disable_cursor(window);
        }
    }

    fn on_enter_background(&mut self, app: &dyn AppHandle) {
        if let Some(window) = app.window() {
            self.enable_cursor(window);
        }
    }

    fn on_exit(&mut self, window: Option<&Window>, _app: &dyn AppHandle) {
        if let Some(window) = window {
            self.enable_cursor(window);
        }
    }

    fn on_keyboard_pressed(
        &mut self,
        code: KeyCode,
        location: KeyLocation,
        _modifiers: Modifiers,
        repeat: bool,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if self.world.is_none() {
            return false;
        }

        if !repeat {
            let config = UserConfig::get();
            let flags = config
                .get_keyboard_input(&(code, location))
                .map(|input| input.into_bits())
                .unwrap_or_default();

            if flags == GameInputBits::Status {
                // 인게임 상태창 장면으로 전환합니다.
                // Safe: self는 null이 아님.
                let prev_scene =
                    unsafe { NonNull::new_unchecked(self as *mut InGameDominationModeScene) };
                let scene = InGameDominationModeStatusLayer::new(self.locale, prev_scene);
                let scene_flow = GameSceneFlow::Push(Box::new(scene));
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }

            self.controller_input_flags |= flags;
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
        app: &dyn AppHandle,
    ) -> bool {
        if self.world.is_none() {
            return false;
        }

        if !repeat {
            if !self.show_status && code == KeyCode::Escape {
                // 인게임 일시정지 장면으로 전환합니다.
                let scene = InGamePauseLayer::new(self.locale);
                let scene_flow = GameSceneFlow::Push(Box::new(scene));
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
                return true;
            }

            let config = UserConfig::get();
            let flags = config
                .get_keyboard_input(&(code, location))
                .map(|input| input.into_bits())
                .unwrap_or_default();
            self.controller_input_flags &= !flags;
        }

        true
    }

    fn on_mouse_btn_pressed(
        &mut self,
        _x: f32,
        _y: f32,
        button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if self.world.is_none() {
            return false;
        }

        let config = UserConfig::get();
        let flags = config
            .get_mouse_input(&button)
            .map(|input| input.into_bits())
            .unwrap_or_default();
        self.controller_input_flags |= flags;

        true
    }

    fn on_mouse_btn_released(
        &mut self,
        _x: f32,
        _y: f32,
        button: MouseButton,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if self.world.is_none() {
            return false;
        }

        let config = UserConfig::get();
        let flags = config
            .get_mouse_input(&button)
            .map(|input| input.into_bits())
            .unwrap_or_default();
        self.controller_input_flags &= !flags;

        true
    }

    fn on_cursor_moved(
        &mut self,
        _x: f32,
        _y: f32,
        mut dx: f32,
        mut dy: f32,
        window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if self.world.is_none() {
            return false;
        }

        self.reset_cursor_position_at_center(window);

        dx *= match self.flip_horizontal {
            true => -self.control_sensitivity,
            false => self.control_sensitivity,
        };

        dy *= match self.flip_vertical {
            true => -self.control_sensitivity,
            false => self.control_sensitivity,
        };

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let entity = self.get_player_entity();
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 삼인칭 카메라를 회전시킵니다.
        let third_person_camera = world
            .query_one_mut::<&mut ThirdPersonCamera>(self.main_camera)
            .expect("invalid entity or invalid entity component");
        third_person_camera.rotate(dx, dy, 1.0);
        let rotation = third_person_camera.rotation;

        // 플레이어에 적용합니다.
        let view_rotation = world
            .query_one_mut::<&mut LatLon>(entity)
            .expect("invalid entity or invalid entity component");
        *view_rotation = rotation;

        true
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

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        if self.world.is_none() {
            return Some(packet);
        }

        match packet.packet_type() {
            PacketType::PullStage => {
                let packet = PullStagePacket::from_raw(packet);
                self.pull_game_data(packet, app);
            }
            PacketType::UdpDamageLog => {
                let packet = UdpDamageLogPacket::from_raw(packet);
                self.create_damage_particles(app.render_device(), packet.logs);
            }
            PacketType::FinishStage => {
                let packet = FinishStagePacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let world = self.world.take().unwrap();
                let skybox = self.skybox.take().unwrap();
                let players = self.players.to_owned();
                let stages = self.stages.to_owned();
                let next_scene = InGameResultEnterScene::new(
                    self.locale,
                    self.user_id,
                    self.token,
                    packet.winner_team(),
                    packet.stage_kind(),
                    world,
                    skybox,
                    players,
                    packet.players,
                    stages,
                    self.ui_textures.clone(),
                    self.motion_pool.clone(),
                );
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            _ => {}
        };

        None
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _pp: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;
        self.particle_timer = (self.particle_timer + elapsed_time_sec) % 1.0;

        self.update_view_state();
        self.update_view_state_timer(elapsed_time_sec);
        self.update_move_direction();
        self.update_character_direction();

        self.update_damage_particles(elapsed_time_sec);
    }

    fn on_post_update(&mut self, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        self.push_player_data(app.net_manager());
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        self.update_bullet();
        self.update_character();
        self.update_camera();

        let device = app.render_device();
        let queue = app.render_queue();
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 카메라 쉐이더 리소스를 갱신합니다.
        self.update_camera_and_skybox_resource(device, &mut encoder, &mut staging_buffers);
        // 데미지 파티클 쉐이더 리소스를 갱신합니다.
        self.update_damage_particle_resources(device, &mut encoder, &mut staging_buffers);

        let mut shadow_map = HashMap::default();
        let mut opaque_map = HashMap::default();
        let mut transparent_map = HashMap::default();

        let world = self.world.as_ref().unwrap();
        let child_view = &world.view::<&Child>();
        let sibling_view = &world.view::<&Sibling>();
        let transform_view = &world.view::<&WorldTransform>();
        let mesh_filter_view = &world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &world.view::<SkinnedMeshRenderer>();

        // 캐릭터 쉐이더 리소스를 갱신합니다.
        let entities = self.culling_character();
        for entity in entities {
            self.update_character_resource(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
                &mut shadow_map,
                &mut opaque_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // 총알 쉐이더 리소스를 갱신합니다.
        let entities = self.culling_bullets();
        for entity in entities {
            self.update_bullet_resource(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
                &mut shadow_map,
                &mut opaque_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // 지형 쉐이더 리소스를 갱신합니다.
        let entities = self.culling_stages();
        for entity in entities {
            self.update_stage_resource(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
                &mut shadow_map,
                &mut opaque_map,
                &mut transparent_map,
                child_view,
                sibling_view,
                transform_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
            self.update_capture_zone_material_resource(
                entity,
                device,
                &mut encoder,
                &mut staging_buffers,
            );
        }

        queue.submit(Some(encoder.finish()));
        drop(staging_buffers);

        self.shadow_map = shadow_map;
        self.opaque_map = opaque_map;
        self.transparent_map = transparent_map;
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) {
        if self.world.is_none() {
            return;
        }

        // 카메라 쉐이더 리소스를 가져옵니다.
        let world = self.world.as_mut().unwrap();
        let camera_resource = world
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

        // Weighted Blended OIT 쉐이더 리소스를 가져옵니다.
        let alpha_blend_resource = self.alpha_blend_resource.as_ref().unwrap();
        let skybox = self.skybox.as_ref().unwrap();

        encoder.push_debug_group("opaque pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(OpaquePass))"),
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

            for ((mesh, kind), resources) in self.opaque_map.iter() {
                let func = match kind {
                    MaterialKind::Bullet => Self::draw_bullet,
                    MaterialKind::EnergyBullet => Self::draw_energy_bullet,
                    MaterialKind::Character => Self::draw_character,
                    MaterialKind::CharacterEyeMouth => Self::draw_character_eye_mouth,
                    MaterialKind::CharacterHalo => Self::draw_character_halo,
                    MaterialKind::Stage => Self::draw_stage,
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::Bullet => BulletRenderPipeline::get(),
                    MaterialKind::EnergyBullet => EnergyBulletRenderPipeline::get(),
                    MaterialKind::Character => CharacterRenderPipeline::get(),
                    MaterialKind::CharacterEyeMouth => EyeMouthRenderPipeline::get(),
                    MaterialKind::CharacterHalo => HaloRenderPipeline::get(),
                    MaterialKind::Stage => StageRenderPipeline::get(),
                    _ => continue,
                }
                .unwrap();

                func(&mesh, pipeline, &camera_resource, &resources, &mut rpass);
            }

            self.draw_damage_particle(
                DamageFontRenderPipeline::get().unwrap(),
                &camera_resource,
                &mut rpass,
            );

            Self::clear_render_target_with_skybox(
                skybox,
                SkyboxRenderPipeline::get().unwrap(),
                &mut rpass,
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("transparent pass");
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(TransparentPass))"),
                color_attachments: &[
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
                        view: &alpha_blend_resource.accum_render_target,
                        resolve_target: None,
                    }),
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
                        view: &alpha_blend_resource.reveal_render_target,
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

            for ((mesh, kind), resources) in self.transparent_map.iter() {
                let func = match kind {
                    MaterialKind::CaptureZone => Self::draw_capture_zone,
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::CaptureZone => CaptureZoneRenderPipeline::get(),
                    _ => continue,
                }
                .unwrap();

                func(&mesh, pipeline, &camera_resource, &resources, &mut rpass);
            }
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

            // 그래픽스 파이프라인을 가져옵니다.
            let pipeline = WeightedBlendedOITRenderPipeline::get().unwrap();
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &alpha_blend_resource.bind_group, &[]);
            rpass.draw(0..4, 0..1);
        }
        encoder.pop_debug_group();
    }

    fn on_finish_draw(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.shadow_map.clear();
        self.opaque_map.clear();
        self.transparent_map.clear();
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

        egui::Area::new(egui::Id::new("Reticle_Layout"))
            .anchor(egui::Align2::CENTER_CENTER, (0.0, 0.0))
            .show(app.egui_ctx(), |ui| {
                ui.painter().add(reticle);
            });

        self.score_gauge_layout(app.egui_ctx(), scale);

        self.health_point_bg_layout(app.egui_ctx(), scale);
        self.health_point_gauge_layout(app.egui_ctx(), scale);

        self.remaining_timer_layout(app.egui_ctx(), scale);
        self.remaining_bullet_layout(app.egui_ctx(), scale);

        self.skill_layout(app.egui_ctx(), scale);
        self.ex_skill_guage_layout(app.egui_ctx(), scale);

        egui::Area::new(egui::Id::new("FrameRate_Layout"))
            .anchor(egui::Align2::LEFT_TOP, (0.0, 0.0))
            .show(app.egui_ctx(), |ui| {
                ui.label(frame_rate_text);
            });
    }
}
