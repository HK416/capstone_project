use std::{collections::BTreeMap, f32::consts::TAU, num::NonZeroU32, sync::Arc, time::Instant};

use ahash::{HashMap, HashSet, RandomState};
use hecs::{Entity, World};
use mod_app::{
    app::AppHandle,
    etc::{AppEvent, Viewport, WindowSize},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        update_view_state, update_view_state_timer, ActionState, ActionStateTimer, BulletData,
        CapturePoint, CharacterFlags, CharacterKind, HealthData, HeldInput, LatLon, LoginToken,
        MovementState, MovementStateTimer, ObjectId, SkillCostData, StageAttributes, Team, UserId,
        UserName, ViewState, ViewStateTimer, MAX_IN_GAME_PLAYERS, MAX_LATITUDE, MIN_LATITUDE,
    },
    protocol::{InGameFinishPacket, InGamePullPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::object3d::Frustum;
use mod_render::{UiRenderer, SWAPCHAIN_FORMAT};
use rodio::{Sink, Source};
use winit::{event::MouseButton, window::Window};

use crate::{
    asset::{
        cull_stage_entities, MeshPool, ModelPool, MotionPool, SamplerPool, SoundDataPool,
        StageBoundingVolumnHierarchy, TextureDataPool, TexturePool, TextureViewPool,
        BG_SOUND_THEME_23, CHARACTER_IMG_SMALL_URI, HUD_LAYOUT_URI_02, IMG_FONT_DRAW,
        IMG_FONT_LOSE_URI, IMG_FONT_WIN_URI, NOTOSANS_BOLD, NOTOSANS_REGULAR, UI_NOTICE,
        UI_VICTORY_ST_01, WEAPON_ICON_URI,
    },
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage, cleanup,
        clear_render_target_with_skybox, collect_character_resource, collect_stage_resource,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_bullet,
        draw_character, draw_character_eye_mouth, draw_character_halo, draw_character_halo_outline,
        draw_energy_bullet, draw_stage, draw_stage_barrier, draw_tree, update_bullet_hierarchy,
        update_bullet_resource, update_camera_and_skybox_resource, update_camera_hierarchy,
        update_camera_param, update_character_hierarchy, update_character_resource,
        update_stage_hierarchy, update_stage_resource, AccumRenderTarget, AlphaBlendPipeline,
        BakeList, BloomPipeline, BoneCollection, BrightRenderTarget, Camera, CameraResource, Child,
        DirectionLight, GaussianBlurPipeline, GlobalLightDataLayout, LightSetResource,
        LightTransformDataLayout, MaterialKind, MaterialResource, MeshRenderer, OpaqueMap,
        PlayerArchetype, Projection, RenderTask, RevealRenderTarget, ShadowMap, Sibling,
        SkinnedMeshRenderer, SkinningAnimation, Skybox, ToParentTrans, TransparentMap,
        WorldTransform, CHARACTER_ATTRIBUTES,
    },
    config::Locale,
    player_execute,
    scenes::{
        FatalErrorSceneLayer, InGameResultScene, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS,
        ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS, FONT_COLOR, TEAM_COLOR,
    },
};

/// 게임 장면 지속 시간
const SCENE_DURATION: u32 = 4_000;

/// 게임 종료 후 대기하는 장면입니다.
pub struct InGameExitScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,
    /// 시야 조작 민감도입니다.
    control_sensitivity: f32,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 최대 게임 플레이 시간
    max_game_play_time_ms: u32,
    /// 클라이언트 게임 플레이 경과 시간
    elapsed_time_ms: u32,
    /// 점령 데이터
    capture_point: CapturePoint,

    /// 게임 완료 패킷
    packet: Option<InGameFinishPacket>,
    /// 플레이어 우승 여부, 비겼을 경우 `None`
    is_player_win: Option<bool>,

    /// 첫 번쨰 마우스 눌림 여부 플래그
    first_mouse_pressed: bool,
    /// 지형 속성 데이터입니다.
    stage_attributes: Arc<StageAttributes>,
    /// 게임 월드 x축 전체 절반 크기
    half_size_x: NonZeroU32,
    /// 게임 월드 y축 전체 절반 크기
    half_size_y: NonZeroU32,
    /// 게임 월드 z축 전체 절반 크기
    half_size_z: NonZeroU32,
    /// 플레이어 시야 상태입니다.
    view_state: ViewState,
    /// 플레이어 시야 상태 타이머입니다.
    view_state_timer: ViewStateTimer,

    /// 플레이어 캐릭터 종류
    player_character: CharacterKind,
    /// 플레이어가 속한 팀
    player_team: Team,

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

    /// 총알 엔터티
    bullets: HashMap<ObjectId, Entity>,
    /// 플레이어 엔터티
    players: HashMap<UserId, (Entity, PlayerArchetype)>,
    /// 스테이지 엔터티
    stage: Option<StageBoundingVolumnHierarchy>,

    /// 재생 중인 배경음 목록
    background_sounds: Vec<Sink>,
    /// 이펙트 사운드 재생 여부
    play_effect_sound: bool,

    /// 이번 프레임에 사용된 모든 Staging Buffer를 담음
    frame_staging_buffers: Vec<wgpu::Buffer>,
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

    /// 헤일로 외곽선 재질 쉐이더 리소스
    outlines: HashMap<Team, MaterialResource>,

    /// 스테이지 스카이박스
    skybox: Option<Skybox>,
    /// 스테이지 방향 조명
    direction_light: Option<DirectionLight>,
    /// 조명 쉐이더 리소스
    light_resource: Option<LightSetResource>,

    /// 조명 렌더링 리소스 집합입니다.
    bake_list: BakeList,
    /// 불투명 메쉬 렌더링 리소스 집합입니다.
    opaque_resources: OpaqueMap,
    /// 투명 메쉬 렌더링 리소스 집합입니다.
    transparent_resources: TransparentMap,

    /// Ui 스케일 값입니다.
    ui_scale: f32,
    /// 인터페이스 클립 사각형 영역입니다.
    clip_rect: egui::Rect,

    /// 인터페이스 배경 레이아웃 텍스처입니다.
    layout_texture: egui::load::SizedTexture,
    /// 이미지 폰트 텍스처입니다.
    img_font_texture: egui::load::SizedTexture,

    /// 체력 인터페이스 사각형 영역입니다.
    health_point_rect: egui::Rect,

    /// 무기 아이콘 텍스처입니다.
    weapon_icon_texture: egui::load::SizedTexture,
    /// 무기 인터페이스 사각형 영역입니다.
    weapon_info_rect: egui::Rect,

    /// 스킬 코스트 사각형 영역입니다.
    skill_cost_rect: egui::Rect,

    /// 타이머 사각형 영역입니다.
    timer_rect: egui::Rect,
    /// 블루 팀 점수 사각형 영역입니다.
    blue_score_rect: egui::Rect,
    /// 레드 팀 점수 사각형 영역입니다.
    red_score_rect: egui::Rect,

    /// 캐릭터 아이콘 텍스터입니다.
    character_icon_textures: HashMap<CharacterKind, egui::load::SizedTexture>,
    /// 팀 상황 사각형 영역입니다.
    team_status_rect: egui::Rect,

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
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InGameExitScene {
    /// 새로운 `InGameExitScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        control_sensitivity: f32,
        flip_horizontal: bool,
        flip_vertical: bool,
        max_game_play_time_ms: u32,
        capture_point: CapturePoint,
        packet: InGameFinishPacket,
        is_player_win: Option<bool>,
        first_mouse_pressed: bool,
        stage_attributes: Arc<StageAttributes>,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
        view_state: ViewState,
        view_state_timer: ViewStateTimer,
        player_character: CharacterKind,
        player_team: Team,
        world: World,
        camera: Entity,
        camera_fov_y: f32,
        camera_rel_position: glam::Vec3A,
        camera_aspect_ratio: f32,
        bullets: HashMap<ObjectId, Entity>,
        players: HashMap<UserId, (Entity, PlayerArchetype)>,
        stage: StageBoundingVolumnHierarchy,
        accum_render_target: AccumRenderTarget,
        reveal_render_target: RevealRenderTarget,
        bright_render_target: BrightRenderTarget,
        alpha_blend_pipeline: AlphaBlendPipeline,
        gaussian_blur_pipeline: GaussianBlurPipeline,
        bloom_pipeline: BloomPipeline,
        outlines: HashMap<Team, MaterialResource>,
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
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            background_volume,
            effect_volume,
            voice_volume,
            control_sensitivity,
            flip_horizontal,
            flip_vertical,
            max_game_play_time_ms,
            capture_point,
            elapsed_time_ms: 0,
            packet: Some(packet),
            is_player_win,
            first_mouse_pressed,
            stage_attributes,
            half_size_x,
            half_size_y,
            half_size_z,
            view_state,
            view_state_timer,
            player_character,
            player_team,
            world: Some(world),
            camera,
            camera_fov_y,
            camera_rel_position,
            camera_aspect_ratio,
            bullets,
            players,
            stage: Some(stage),
            background_sounds: Vec::with_capacity(1),
            play_effect_sound: false,
            frame_staging_buffers: Vec::default(),
            accum_render_target: Some(accum_render_target),
            reveal_render_target: Some(reveal_render_target),
            bright_render_target: Some(bright_render_target),
            alpha_blend_pipeline: Some(alpha_blend_pipeline),
            gaussian_blur_pipeline: Some(gaussian_blur_pipeline),
            bloom_pipeline: Some(bloom_pipeline),
            outlines,
            skybox: Some(skybox),
            direction_light: Some(direction_light),
            light_resource: Some(light_resource),
            bake_list: BakeList::default(),
            opaque_resources: OpaqueMap::default(),
            transparent_resources: TransparentMap::default(),
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            layout_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            img_font_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            health_point_rect: egui::Rect::ZERO,
            weapon_icon_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            weapon_info_rect: egui::Rect::ZERO,
            skill_cost_rect: egui::Rect::ZERO,
            timer_rect: egui::Rect::ZERO,
            blue_score_rect: egui::Rect::ZERO,
            red_score_rect: egui::Rect::ZERO,
            character_icon_textures: HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::new(),
            ),
            team_status_rect: egui::Rect::ZERO,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_pool,
            texture_data_pool,
            texture_view_pool,
            sampler_pool,
            sound_data_pool,
        }
    }

    /// 플레이어 엔터티를 반환합니다.
    fn player_entity(&self) -> (Entity, PlayerArchetype) {
        self.players
            .get(&self.uid)
            .cloned()
            .expect("no such entity!")
    }

    /// 카메라 파라미터 데이터를 갱신합니다.
    fn update_camera_param(&mut self) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        let mut latitude = 0.0;
        let mut longitude = 0.0;
        type Query<'a> = (&'a ActionState, &'a ActionStateTimer, &'a LatLon);
        player_execute!(archetype, world, entity, Query, |(
            &action_state,
            &action_state_timer,
            &latlon,
        )| {
            latitude = latlon.lat;
            longitude = latlon.lon;

            // 카메라 파라미터를 갱신합니다.
            update_camera_param(
                &mut self.camera_rel_position,
                &mut self.camera_fov_y,
                self.player_character,
                action_state,
                self.view_state,
                action_state_timer,
                self.view_state_timer,
            );
        });

        // 카메라 변환 행렬을 생성합니다.
        let distance = self.camera_rel_position * glam::Vec3A::NEG_Z;
        let mut transform = glam::Mat4::from_translation(distance.into());
        let rotation = glam::Mat4::from_rotation_y(longitude);
        transform = rotation * transform;

        let forward = glam::Vec3A::from_vec4(transform.z_axis);
        let forward = forward.normalize_or(glam::Vec3A::Z);
        let axis = glam::Vec3A::Y.cross(forward);
        let rotation = glam::Mat4::from_axis_angle(axis.into(), latitude);
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

        let mut translation = glam::Vec3A::ZERO;
        player_execute!(archetype, world, entity, &WorldTransform, |trans| {
            translation = trans.get_translation();
        });

        // 카메라 엔터티 계층 구조를 갱신합니다.
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
    fn pull_server_data(&mut self, time_stamp: Instant, packet: InGamePullPacket) {
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        // 서버 패킷 수신 지연 시간을 계산합니다.
        let delay_time = Instant::now()
            .saturating_duration_since(time_stamp)
            .as_millis()
            .min(SCENE_DURATION as u128) as u32;
        let rtt_time = (packet.ping) as u32 / 2;
        let latency = delay_time + rtt_time;

        // 서버 시간을 계산합니다.
        let server_play_elapsed_time_ms = packet
            .play_elapsed_time_ms
            .saturating_add(latency)
            .min(SCENE_DURATION);
        let offset = server_play_elapsed_time_ms as i32 - self.elapsed_time_ms as i32;

        // 클라이언트 시간을 보정합니다.
        self.elapsed_time_ms = self
            .elapsed_time_ms
            .saturating_add_signed(offset)
            .min(SCENE_DURATION);

        // 플레이어 데이터를 갱신합니다.
        for data in packet.players.iter() {
            // 해당 플레이어 엔터티를 가져옵니다.
            let (entity, archetype) = self
                .players
                .get(&data.uid)
                .cloned()
                .expect("player not found!");

            // 캐릭터 속성 데이터를 가져옵니다.
            let (&character_kind, character_flags) = world
                .query_one_mut::<(&CharacterKind, &CharacterFlags)>(entity)
                .expect("invalid entity or invalid entity component!");

            // 서버와 접속 중이지 않은 경우 건너뜁니다.
            if !character_flags.is_connected() {
                continue;
            }

            let i = character_kind as usize;
            let attribute = CHARACTER_ATTRIBUTES[i];

            type Query<'a> = (
                &'a mut ActionState,
                &'a mut ActionStateTimer,
                &'a mut MovementState,
                &'a mut MovementStateTimer,
                &'a mut ToParentTrans,
            );
            player_execute!(archetype, world, entity, Query, |(
                action_state,
                action_state_timer,
                movement_state,
                movement_state_timer,
                transform,
            )| {
                *action_state = data.action_state();
                *action_state_timer = data.action_state_timer(attribute);
                *movement_state = data.movement_state();
                *movement_state_timer = data.movement_state_timer(attribute);

                let rotation = data.rotation();
                let translation =
                    data.trasnaltion(self.half_size_x, self.half_size_y, self.half_size_z);
                transform.set_rotation_translation(rotation.into(), translation.into());
            });
        }
    }

    /// 인터페이스 배경 레이아웃 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_layout_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 레이아웃 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_LAYOUT_URI_02)
            .expect("HUD_Layout_02 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.layout_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    fn regist_img_font_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        let uri = match self.is_player_win {
            Some(is_player_win) => {
                if is_player_win {
                    IMG_FONT_WIN_URI
                } else {
                    IMG_FONT_LOSE_URI
                }
            }
            None => IMG_FONT_DRAW,
        };

        // 레이아웃 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(uri)
            .expect("ImgFont_Win, ImgFont_Lose and ImgFont_Draw texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.img_font_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 무기 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_weapon_icon_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        let texture = self
            .texture_pool
            .get(WEAPON_ICON_URI)
            .expect("Weapon_Icon texture must be preloaded");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: self.player_character as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.weapon_icon_texture = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };
    }

    /// 캐릭터 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_character_icon_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
    ) {
        let texture = self
            .texture_pool
            .get(CHARACTER_IMG_SMALL_URI)
            .expect("Weapon_Icon texture must be preloaded");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        let character_kinds: HashSet<_> = {
            let world = self.world.as_mut().expect("the world must be exists!");
            let character_view = world.view_mut::<&CharacterKind>();
            self.players
                .values()
                .map(|&(entity, _archetype)| {
                    character_view
                        .get(entity)
                        .cloned()
                        .expect("invalid entity or invalid entity component!")
                })
                .collect()
        };

        for kind in character_kinds {
            // 텍스처의 텍스처 뷰를 생성합니다.
            let texture = self.texture_view_pool.get_or_init(
                &texture,
                &wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: kind as u32,
                    array_layer_count: Some(1),
                    ..Default::default()
                },
            );

            // egui 렌더러에 텍스처를 등록합니다.
            let texture_id =
                ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

            // 등록된 텍스처 정보를 저장합니다.
            self.character_icon_textures.insert(
                kind,
                egui::load::SizedTexture {
                    id: texture_id,
                    size: texture_size,
                },
            );
        }
    }

    fn resize_ui(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형 영역의 크기를 재조정합니다.
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        (self.clip_rect, self.ui_scale) = Self::resize_clip_rect(viewport, scale_factor);

        // 체력 인터페이스 영역의 크기를 재조정합니다.
        self.health_point_rect = Self::resize_health_point_rect(&self.clip_rect);
        // 무기 인터페이스 영역의 크기를 재조정합니다.
        let texture_size = self.weapon_icon_texture.size;
        self.weapon_info_rect = Self::resize_weapon_info_rect(&texture_size, &self.clip_rect);
        // 스킬 인터페이스 영역의 크기를 재조정합니다.
        self.skill_cost_rect =
            Self::resize_skill_cost_rect(&self.clip_rect, &self.weapon_info_rect);
        // 타이머 영역의 크기를 재조정합니다.
        self.timer_rect = Self::resize_timer_rect(&self.clip_rect);
        // 블루 팀 스코어 영역의 크기를 재조정합니다.
        self.blue_score_rect = Self::resize_blue_score_rect(&self.clip_rect);
        // 레드 팀 스코어 영역의 크기를 재조정합니다.
        self.red_score_rect = Self::resize_red_score_rect(&self.clip_rect);
        // 팀 상태 영역의 크기를 재조정합니다.
        self.team_status_rect = Self::resize_team_status_rect(&self.clip_rect);
    }

    /// 애니메이션 값을 가져옵니다.
    fn ui_animation_factor(&self) -> f32 {
        1.0 - self.elapsed_time_ms.min(1000) as f32 / 1000.0
    }

    /// 클립 사각형 영역의 크기를 재조정합니다.
    fn resize_clip_rect(viewport: &Viewport, scale_factor: f32) -> (egui::Rect, f32) {
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        (clip_rect, scale)
    }

    /// 체력 사각형 영역의 크기를 재조정합니다.
    fn resize_health_point_rect(clip_rect: &egui::Rect) -> egui::Rect {
        let width = clip_rect.width() * 0.21875;
        let height = width * 0.325;
        let margin = clip_rect.size().min_elem() * 0.03;
        let left_bottom = clip_rect.left_bottom() + egui::vec2(margin, -margin);
        let right_top = left_bottom + egui::vec2(width, -height);
        egui::Rect::from_two_pos(left_bottom, right_top)
    }

    /// 체력 인터페이스의 콘텐츠 영역을 반환합니다.
    fn health_point_content_rect(&self) -> egui::Rect {
        self.health_point_rect
            .scale_from_center2(egui::vec2(0.82, 0.92))
    }

    /// 무기 인터페이스 영역의 크기를 재조정합니다.
    fn resize_weapon_info_rect(texture_size: &egui::Vec2, clip_rect: &egui::Rect) -> egui::Rect {
        let ratio = texture_size.x / texture_size.y;
        let width = clip_rect.width() * 0.3;
        let height = clip_rect.width() * 0.2 / ratio;
        let margin = clip_rect.size().min_elem() * 0.03;
        let right_bottom = clip_rect.right_bottom() - egui::vec2(margin * 2.0, margin);
        let left_top = right_bottom - egui::vec2(width, height);
        egui::Rect::from_min_max(left_top, right_bottom)
    }

    /// 무기 아이콘의 콘텐츠 영역을 반환합니다.
    fn weapon_icon_content_rect(&self) -> egui::Rect {
        let rect = self.weapon_info_content_rect();
        let texture_size = self.weapon_icon_texture.size;
        let ratio = texture_size.x / texture_size.y;
        let height = rect.height();
        let width = height * ratio;
        let right_bottom = rect.right_bottom();
        let left_top = right_bottom - egui::vec2(width, height);
        egui::Rect::from_min_max(left_top, right_bottom)
    }

    /// 무기 라벨의 콘텐츠 영역을 반환합니다.
    fn weapon_label_content_rect(&self) -> egui::Rect {
        let rect = self.weapon_info_content_rect();
        let texture_size = self.weapon_icon_texture.size;
        let ratio = texture_size.x / texture_size.y;
        let height = rect.height();
        let width = rect.width() - height * ratio;
        let left_bottom = rect.left_bottom();
        let right_top = left_bottom + egui::vec2(width, -height);
        egui::Rect::from_two_pos(left_bottom, right_top)
    }

    /// 무기 인터페이스의 콘텐츠 영역을 반환합니다.
    fn weapon_info_content_rect(&self) -> egui::Rect {
        self.weapon_info_rect
            .scale_from_center2(egui::vec2(0.82, 0.92))
    }

    /// 스킬 코스트 인터페이스 영역의 크기를 재조정합니다.
    fn resize_skill_cost_rect(clip_rect: &egui::Rect, weapon_info_rect: &egui::Rect) -> egui::Rect {
        let width = weapon_info_rect.width() * 0.9;
        let height = weapon_info_rect.height();
        let margin_x = clip_rect.size().min_elem() * 0.035;
        let margin_y = clip_rect.size().min_elem() * 0.01;
        let right_bottom = weapon_info_rect.right_top() + egui::vec2(margin_x, -margin_y);
        let left_top = right_bottom - egui::vec2(width, height);
        egui::Rect::from_two_pos(left_top, right_bottom)
    }

    /// 타이머 사각형 영역의 크기를 재조정합니다.
    fn resize_timer_rect(clip_rect: &egui::Rect) -> egui::Rect {
        let margin = clip_rect.size().min_elem() * 0.04;
        let width = clip_rect.width() * 0.1;
        let height = 0.5 * width;
        let center = egui::pos2(
            clip_rect.center().x,
            clip_rect.top() + margin + 0.5 * height,
        );
        let size = egui::vec2(width, height);
        egui::Rect::from_center_size(center, size)
    }

    /// 블루팀 스코어 영역의 크기를 재조정합니다.
    fn resize_blue_score_rect(clip_rect: &egui::Rect) -> egui::Rect {
        let margin = clip_rect.size().min_elem() * 0.06;
        let size = clip_rect.width() * 0.1;
        let width = clip_rect.width() * 0.26;
        let height = width * 0.04;
        let left_top = egui::pos2(clip_rect.center().x + 0.5 * size, clip_rect.top() + margin);
        let right_bottom = left_top + egui::vec2(width, height);
        egui::Rect::from_min_max(left_top, right_bottom)
    }

    /// 레드팀 스코어 영역의 크기를 재조정합니다.
    fn resize_red_score_rect(clip_rect: &egui::Rect) -> egui::Rect {
        let margin = clip_rect.size().min_elem() * 0.06;
        let size = clip_rect.width() * 0.1;
        let width = clip_rect.width() * 0.26;
        let height = width * 0.04;
        let right_top = egui::pos2(clip_rect.center().x - 0.5 * size, clip_rect.top() + margin);
        let left_bottom = right_top + egui::vec2(-width, height);
        egui::Rect::from_two_pos(right_top, left_bottom)
    }

    /// 스코어 영역을 모두 포함하는 영역을 반환합니다.
    fn score_union_rect(&self) -> egui::Rect {
        self.blue_score_rect
            .union(self.red_score_rect)
            .union(self.timer_rect)
    }

    /// 팀 상태를 표시하는 영역을 반환합니다.
    fn resize_team_status_rect(clip_rect: &egui::Rect) -> egui::Rect {
        let margin = clip_rect.size().min_elem() * 0.05;
        let height = clip_rect.height() * 0.3;
        let width = clip_rect.width() * 0.16;
        let size = egui::vec2(width, height);
        let min = clip_rect.left_center() + egui::vec2(margin, -0.75 * height);
        egui::Rect::from_min_size(min, size)
    }

    /// 스킬 아이콘 영역을 반환합니다.
    fn skill_icon_rect(&self) -> egui::Rect {
        let height = self.skill_cost_rect.height();
        let left_top = self.skill_cost_rect.left_top();
        let right_bottom = left_top + egui::vec2(height * 1.5, height);
        egui::Rect::from_min_max(left_top, right_bottom)
    }

    /// 스킬 게이지 영역을 반환합니다.
    fn skill_gauge_rect(&self) -> egui::Rect {
        let height = self.skill_cost_rect.height();
        let width = self.skill_cost_rect.width() - height * 1.5;
        let right_bottom = self.skill_cost_rect.right_bottom();
        let left_top = right_bottom - egui::vec2(width, height);
        egui::Rect::from_min_max(left_top, right_bottom)
    }

    /// 체력 인터페이스를 그립니다.
    fn draw_health_point(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Health_Point"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                self.draw_health_point_bg(ui);
                self.draw_health_point_gauge(ui);
                self.draw_health_point_label(ui);
            });
    }

    /// 체력 인터페이스의 배경을 그립니다.
    fn draw_health_point_bg(&self, ui: &mut egui::Ui) {
        const TINT: egui::Color32 = egui::Color32::from_black_alpha(160);
        const SIZE: f32 = 256.0;
        const TOP: f32 = 11.0;
        const LEFT: f32 = 17.0;
        const BOTTOM: f32 = 242.0;
        const RIGHT: f32 = 235.0;
        const INNER_LEFT_TOP: egui::Pos2 = egui::pos2(66.0, 22.0);
        const INNER_RIGHT_TOP: egui::Pos2 = egui::pos2(222.0, 22.0);
        const INNER_LEFT_BOTTOM: egui::Pos2 = egui::pos2(30.0, 228.0);
        const INNER_RIGHT_BOTTOM: egui::Pos2 = egui::pos2(182.0, 228.0);

        let t = self.ui_animation_factor();
        let base_x = -self.clip_rect.width() * 0.5 * (1.0 - t);
        let content_rect = self.health_point_content_rect();

        let uv = egui::Rect::from_min_max(
            egui::pos2(RIGHT / SIZE, TOP / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(
                base_x + self.health_point_rect.left(),
                self.health_point_rect.top(),
            ),
            egui::pos2(base_x + content_rect.left(), content_rect.top()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, TOP / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.left(), self.health_point_rect.top()),
            egui::pos2(base_x + content_rect.right(), content_rect.top()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, TOP / SIZE),
            egui::pos2(LEFT / SIZE, INNER_LEFT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.right(), self.health_point_rect.top()),
            egui::pos2(base_x + self.health_point_rect.right(), content_rect.top()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(RIGHT / SIZE, INNER_RIGHT_TOP.y / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + self.health_point_rect.left(), content_rect.top()),
            egui::pos2(base_x + content_rect.left(), content_rect.bottom()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_TOP.y / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.left(), content_rect.top()),
            egui::pos2(base_x + content_rect.right(), content_rect.bottom()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_TOP.y / SIZE),
            egui::pos2(LEFT / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.right(), content_rect.top()),
            egui::pos2(
                base_x + self.health_point_rect.right(),
                content_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(RIGHT / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(
                base_x + self.health_point_rect.left(),
                content_rect.bottom(),
            ),
            egui::pos2(
                base_x + content_rect.left(),
                self.health_point_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.left(), content_rect.bottom()),
            egui::pos2(
                base_x + content_rect.right(),
                self.health_point_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
            egui::pos2(LEFT / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.right(), content_rect.bottom()),
            egui::pos2(
                base_x + self.health_point_rect.right(),
                self.health_point_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);
    }

    /// 체력 게이지 인터페이스의 배경을 그립니다.
    fn draw_health_point_gauge(&self, ui: &mut egui::Ui) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        let mut shield_percent = 0.0;
        let mut health_percent = 1.0;
        player_execute!(archetype, world, entity, &HealthData, |health_data| {
            (health_percent, shield_percent) = health_data.percent().unwrap_or((1.0, 0.0));
            shield_percent = shield_percent.clamp(0.0, 1.0);
            health_percent = health_percent.clamp(0.0, 1.0);
        });

        let t = self.ui_animation_factor();
        let base_x = -self.clip_rect.width() * 0.5 * (1.0 - t);
        let content_rect = self.health_point_content_rect();

        const NUM_GAUGE: usize = 8;
        const NUM_INTERVAL: usize = NUM_GAUGE - 1;
        let interval = content_rect.width() * 0.01;
        let gauge_size = (content_rect.width() - interval * NUM_INTERVAL as f32) / NUM_GAUGE as f32;
        let center_y = (content_rect.top() + content_rect.center().y) * 0.5;
        let health_range = gauge_size * NUM_GAUGE as f32 * health_percent;
        let shield_range = gauge_size * NUM_GAUGE as f32 * (health_percent + shield_percent);
        let mut cnt = NUM_GAUGE;

        let begin = base_x + content_rect.left();
        let end = base_x + content_rect.right();
        let mut x = end;
        let mut curr = gauge_size * NUM_GAUGE as f32;
        while shield_range < curr {
            let center = egui::pos2(x - gauge_size * 0.5, center_y);
            let size = egui::Vec2::splat(gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            ui.painter()
                .rect_filled(rect, gauge_size * 0.1, egui::Color32::DARK_GRAY);

            x = x - (gauge_size + interval);
            curr -= gauge_size;
            cnt -= 1;
        }

        let width = shield_range - curr;
        if shield_percent > 0.0 && width > 0.0 {
            let center = egui::pos2(x + interval + width * 0.5, center_y);
            let size = egui::vec2(width, gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            ui.painter()
                .rect_filled(rect, gauge_size * 0.1, egui::Color32::from_rgb(255, 192, 0));
        }

        while health_range < curr {
            let center = egui::pos2(x - gauge_size * 0.5, center_y);
            let size = egui::Vec2::splat(gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            ui.painter()
                .rect_filled(rect, gauge_size * 0.1, egui::Color32::from_rgb(255, 192, 0));

            x = x - (gauge_size + interval);
            curr -= gauge_size;
            cnt -= 1;
        }

        let width = health_range - curr;
        if health_percent > 0.0 && width > 0.0 {
            let center = egui::pos2(x + interval + width * 0.5, center_y);
            let size = egui::vec2(width, gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            let fill_color = match cnt < 3 {
                true => egui::Color32::LIGHT_RED,
                false => egui::Color32::WHITE,
            };
            ui.painter().rect_filled(rect, gauge_size * 0.1, fill_color);
        }

        while begin < x {
            let center = egui::pos2(x - gauge_size * 0.5, center_y);
            let size = egui::Vec2::splat(gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            let fill_color = match cnt <= 3 {
                true => egui::Color32::LIGHT_RED,
                false => egui::Color32::WHITE,
            };
            ui.painter().rect_filled(rect, gauge_size * 0.1, fill_color);

            x = x - (gauge_size + interval);
            cnt -= 1;
        }
    }

    /// 체력 게이지 인터페이스의 라벨을 그립니다.
    fn draw_health_point_label(&self, ui: &mut egui::Ui) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        let mut shield = 0;
        let mut health = 0;
        player_execute!(archetype, world, entity, &HealthData, |health_data| {
            shield = health_data.shield;
            health = health_data.remaining;
        });

        let t = self.ui_animation_factor();
        let base_x = -self.clip_rect.width() * 0.5 * (1.0 - t);
        let content_rect = self.health_point_content_rect();
        let total_health = (shield + health).min(9999);

        // 체력 텍스트를 생성합니다.
        let text = format!("{}", total_health);
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(22.0 * self.ui_scale, family);
        let color = if shield > 0 {
            egui::Color32::from_rgb(255, 192, 0)
        } else {
            egui::Color32::WHITE
        };
        let text = egui::RichText::new(text).font(font_id).color(color);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        let rect = egui::Rect::from_min_max(
            egui::pos2(
                base_x + content_rect.left() + content_rect.width() * 0.68,
                content_rect.center().y,
            ),
            egui::pos2(
                base_x + content_rect.right_bottom().x,
                content_rect.right_bottom().y,
            ),
        );
        ui.put(rect, label);

        let mut min = content_rect.left_center();
        min.x += base_x;
        let mut max = content_rect.right_center();
        max.x += base_x;
        ui.painter().line(
            vec![min, max],
            egui::Stroke::new(1.0 * self.ui_scale, egui::Color32::WHITE),
        );
    }

    /// 무기 정보 인터페이스를 그립니다.
    fn draw_weapon_info(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Weapon_Info"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                self.draw_weapon_info_bg(ui);
                self.draw_weapon_icon(ui);
            });
    }

    /// 무기 정보 인터페이스 배경을 그립니다.
    fn draw_weapon_info_bg(&self, ui: &mut egui::Ui) {
        const TINT: egui::Color32 = egui::Color32::from_black_alpha(160);
        const SIZE: f32 = 256.0;
        const TOP: f32 = 11.0;
        const LEFT: f32 = 17.0;
        const BOTTOM: f32 = 242.0;
        const RIGHT: f32 = 235.0;
        const INNER_LEFT_TOP: egui::Pos2 = egui::pos2(66.0, 22.0);
        const INNER_RIGHT_TOP: egui::Pos2 = egui::pos2(222.0, 22.0);
        const INNER_LEFT_BOTTOM: egui::Pos2 = egui::pos2(30.0, 228.0);
        const INNER_RIGHT_BOTTOM: egui::Pos2 = egui::pos2(182.0, 228.0);

        let t = self.ui_animation_factor();
        let base_x = self.clip_rect.width() * 0.5 * (1.0 - t);
        let content_rect = self.weapon_info_content_rect();

        let uv = egui::Rect::from_min_max(
            egui::pos2(LEFT / SIZE, TOP / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(
                base_x + self.weapon_info_rect.left(),
                self.weapon_info_rect.top(),
            ),
            egui::pos2(base_x + content_rect.left(), content_rect.top()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, TOP / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.left(), self.weapon_info_rect.top()),
            egui::pos2(base_x + content_rect.right(), content_rect.top()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, TOP / SIZE),
            egui::pos2(RIGHT / SIZE, INNER_RIGHT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.right(), self.weapon_info_rect.top()),
            egui::pos2(base_x + self.weapon_info_rect.right(), content_rect.top()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(LEFT / SIZE, INNER_LEFT_TOP.y / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + self.weapon_info_rect.left(), content_rect.top()),
            egui::pos2(base_x + content_rect.left(), content_rect.bottom()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_TOP.y / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.left(), content_rect.top()),
            egui::pos2(base_x + content_rect.right(), content_rect.bottom()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_TOP.y / SIZE),
            egui::pos2(RIGHT / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.right(), content_rect.top()),
            egui::pos2(
                base_x + self.weapon_info_rect.right(),
                content_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(LEFT / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + self.weapon_info_rect.left(), content_rect.bottom()),
            egui::pos2(base_x + content_rect.left(), self.weapon_info_rect.bottom()),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.left(), content_rect.bottom()),
            egui::pos2(
                base_x + content_rect.right(),
                self.weapon_info_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
            egui::pos2(RIGHT / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + content_rect.right(), content_rect.bottom()),
            egui::pos2(
                base_x + self.weapon_info_rect.right(),
                self.weapon_info_rect.bottom(),
            ),
        );
        egui::Image::new(self.layout_texture)
            .tint(TINT)
            .uv(uv)
            .paint_at(ui, rect);
    }

    /// 무기 아이콘을 그립니다.
    fn draw_weapon_icon(&self, ui: &mut egui::Ui) {
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        let mut max_bullets = 0;
        let mut remaining_bullets = 0;
        player_execute!(archetype, world, entity, &BulletData, |bullet_data| {
            max_bullets = bullet_data.num_maximum_bullets().min(99);
            remaining_bullets = bullet_data.remaining.min(99);
        });

        let t = self.ui_animation_factor();
        let base_x = self.clip_rect.width() * 0.5 * (1.0 - t);
        let mut rect = self.weapon_icon_content_rect();
        rect.min.x += base_x;
        rect.max.x += base_x;
        egui::Image::new(self.weapon_icon_texture)
            .sense(egui::Sense::empty())
            .paint_at(ui, rect);

        let mut rect = self.weapon_label_content_rect();
        rect.min.x += base_x;
        rect.max.x += base_x;
        let text = format!("{}/{}", remaining_bullets, max_bullets);
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(24.0 * self.ui_scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .wrap_mode(egui::TextWrapMode::Truncate)
            .selectable(false);
        ui.put(rect, label);
    }

    /// 스킬 코스트 정보 인터페이스를 그립니다.
    fn draw_skill_cost_info(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("SkillCost"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                self.draw_skill_icon(ui);
                self.draw_skill_gauge(ui);
            });
    }

    /// 스킬 아이콘 인터페이스를 그립니다.
    fn draw_skill_icon(&self, ui: &mut egui::Ui) {
        const TINT: egui::Color32 = egui::Color32::from_black_alpha(160);
        let offset_x =
            0.5 * (self.weapon_info_rect.width() - self.weapon_info_content_rect().width());

        let t = self.ui_animation_factor();
        let base_x = self.clip_rect.width() * 0.5 * (1.0 - t);
        let outer_rect = self.skill_icon_rect();
        let mut inner_rect = outer_rect.scale_from_center2(egui::vec2(1.0, 0.9));
        inner_rect.max.x -= offset_x;
        inner_rect.min.x += offset_x;

        Self::draw_layout(
            base_x,
            TINT,
            &outer_rect,
            &inner_rect,
            self.layout_texture,
            ui,
        );
    }

    /// 스킬 게이지 인터페이스를 그립니다.
    fn draw_skill_gauge(&self, ui: &mut egui::Ui) {
        const TINT: egui::Color32 = egui::Color32::from_black_alpha(160);
        let offset =
            0.5 * (self.weapon_info_rect.width() - self.weapon_info_content_rect().width());

        let t = self.ui_animation_factor();
        let base_x = self.clip_rect.width() * 0.5 * (1.0 - t);
        let outer_rect = self.skill_gauge_rect();
        let mut inner_rect = outer_rect.scale_from_center2(egui::vec2(1.0, 0.9));
        inner_rect.max.x -= offset;
        inner_rect.min.x += offset;
        Self::draw_layout(
            base_x,
            TINT,
            &outer_rect,
            &inner_rect,
            self.layout_texture,
            ui,
        );

        let mut persent = 0.0;
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };
        player_execute!(
            archetype,
            world,
            entity,
            &SkillCostData,
            |skill_cost_data| {
                persent = skill_cost_data.percent().unwrap_or(1f32);
                persent = persent.clamp(0.0, 1.0);
            }
        );

        const NUM_INTERVAL: usize = 7;
        const NUM_GAGUE: usize = NUM_INTERVAL + 1;
        let interval = inner_rect.width() * 0.01;
        let gauge_size = (inner_rect.width() - interval * NUM_INTERVAL as f32) / NUM_GAGUE as f32;
        let center_y = inner_rect.center().y;
        let range = gauge_size * NUM_GAGUE as f32 * persent;

        let begin = base_x + inner_rect.left();
        let end = base_x + inner_rect.right();
        let mut x = end;
        let mut curr = gauge_size * NUM_GAGUE as f32;
        while range < curr {
            let center = egui::pos2(x - gauge_size * 0.5, center_y);
            let size = egui::Vec2::splat(gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            ui.painter()
                .rect_filled(rect, gauge_size * 0.1, egui::Color32::DARK_GRAY);
            x = x - (gauge_size + interval);
            curr -= gauge_size;
        }

        let width = range - curr;
        if persent > 0.0 && width > 0.0 {
            let center = egui::pos2(x + interval + width * 0.5, center_y);
            let size = egui::vec2(width, gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            ui.painter()
                .rect_filled(rect, gauge_size * 0.1, egui::Color32::from_rgb(255, 192, 0));
        }

        while begin < x {
            let center = egui::pos2(x - gauge_size * 0.5, center_y);
            let size = egui::Vec2::splat(gauge_size);
            let rect = egui::Rect::from_center_size(center, size);
            ui.painter()
                .rect_filled(rect, gauge_size * 0.1, egui::Color32::from_rgb(255, 192, 0));
            x = x - (gauge_size + interval);
        }
    }

    /// 정방향 레이아웃 이미지를 그립니다.
    fn draw_layout(
        base_x: f32,
        tint: egui::Color32,
        outer_rect: &egui::Rect,
        inner_rect: &egui::Rect,
        layout_texture: egui::load::SizedTexture,
        ui: &mut egui::Ui,
    ) {
        const SIZE: f32 = 256.0;
        const TOP: f32 = 11.0;
        const LEFT: f32 = 17.0;
        const BOTTOM: f32 = 242.0;
        const RIGHT: f32 = 235.0;
        const INNER_LEFT_TOP: egui::Pos2 = egui::pos2(66.0, 22.0);
        const INNER_RIGHT_TOP: egui::Pos2 = egui::pos2(222.0, 22.0);
        const INNER_LEFT_BOTTOM: egui::Pos2 = egui::pos2(30.0, 228.0);
        const INNER_RIGHT_BOTTOM: egui::Pos2 = egui::pos2(182.0, 228.0);

        let uv = egui::Rect::from_min_max(
            egui::pos2(LEFT / SIZE, TOP / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + outer_rect.left(), outer_rect.top()),
            egui::pos2(base_x + inner_rect.left(), inner_rect.top()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, TOP / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + inner_rect.left(), outer_rect.top()),
            egui::pos2(base_x + inner_rect.right(), inner_rect.top()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, TOP / SIZE),
            egui::pos2(RIGHT / SIZE, INNER_RIGHT_TOP.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + inner_rect.right(), outer_rect.top()),
            egui::pos2(base_x + outer_rect.right(), inner_rect.top()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(LEFT / SIZE, INNER_LEFT_TOP.y / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + outer_rect.left(), inner_rect.top()),
            egui::pos2(base_x + inner_rect.left(), inner_rect.bottom()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_TOP.y / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + inner_rect.left(), inner_rect.top()),
            egui::pos2(base_x + inner_rect.right(), inner_rect.bottom()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_TOP.y / SIZE),
            egui::pos2(RIGHT / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + inner_rect.right(), inner_rect.top()),
            egui::pos2(base_x + outer_rect.right(), inner_rect.bottom()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(LEFT / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
            egui::pos2(INNER_LEFT_TOP.x / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + outer_rect.left(), inner_rect.bottom()),
            egui::pos2(base_x + inner_rect.left(), outer_rect.bottom()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_LEFT_TOP.x / SIZE, INNER_LEFT_BOTTOM.y / SIZE),
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + inner_rect.left(), inner_rect.bottom()),
            egui::pos2(base_x + inner_rect.right(), outer_rect.bottom()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);

        let uv = egui::Rect::from_min_max(
            egui::pos2(INNER_RIGHT_BOTTOM.x / SIZE, INNER_RIGHT_BOTTOM.y / SIZE),
            egui::pos2(RIGHT / SIZE, BOTTOM / SIZE),
        );
        let rect = egui::Rect::from_min_max(
            egui::pos2(base_x + inner_rect.right(), inner_rect.bottom()),
            egui::pos2(base_x + outer_rect.right(), outer_rect.bottom()),
        );
        egui::Image::new(layout_texture)
            .tint(tint)
            .uv(uv)
            .paint_at(ui, rect);
    }

    /// 게임 정보 인터페이스를 그립니다.
    fn draw_score(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Game_Info"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                self.draw_timer(ui);
                self.draw_blue_team_gauge(ui);
                self.draw_red_team_gauge(ui);
            });
    }

    fn draw_timer(&self, ui: &mut egui::Ui) {
        let packet = match self.packet.as_ref() {
            Some(packet) => packet,
            None => return,
        };

        let union_rect = self.score_union_rect();
        let t = self.ui_animation_factor();
        let base_y = -union_rect.height() * (1.0 - t);

        let outer_rect = self.timer_rect;
        let size = egui::Vec2::splat(outer_rect.size().min_elem() * 0.05);
        let mut inner_rect = outer_rect;
        inner_rect.min += size;
        inner_rect.max -= size;

        let play_time_ms = self
            .max_game_play_time_ms
            .saturating_sub(packet.play_time_ms);
        let remaining_time_sec = play_time_ms as f32 / 1000.0;
        let text = format!("{}", remaining_time_sec.ceil() as u32);
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(42.0 * self.ui_scale, family);
        let center = egui::pos2(
            inner_rect.center().x,
            0.5 * (self.clip_rect.top() + inner_rect.center().y) + base_y,
        );
        let offsets = [
            egui::vec2(-1.0, 0.0),
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, -1.0),
            egui::vec2(0.0, 1.0),
        ];

        for offset in offsets {
            ui.painter().text(
                center + offset * self.ui_scale,
                egui::Align2::CENTER_CENTER,
                &text,
                font_id.clone(),
                FONT_COLOR,
            );
        }

        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            &text,
            font_id.clone(),
            egui::Color32::WHITE,
        );
    }

    fn draw_blue_team_gauge(&self, ui: &mut egui::Ui) {
        let union_rect = self.score_union_rect();
        let t = self.ui_animation_factor();
        let base_y = -union_rect.height() * (1.0 - t);

        let mut outer_rect = self.blue_score_rect;
        let font_center = (self.clip_rect.top() + outer_rect.top()) * 0.5;
        outer_rect.min.y += base_y;
        outer_rect.max.y += base_y;

        let size = egui::Vec2::splat(outer_rect.size().min_elem() * 0.2);
        let mut inner_rect = outer_rect;
        inner_rect.min += size;
        inner_rect.max -= size;

        // 블루 팀 점령도 게이지
        let percent = self.capture_point.blue_progress();
        let corner_radius = outer_rect.height() * 0.5;
        ui.painter()
            .rect_filled(outer_rect, corner_radius, egui::Color32::WHITE);
        if percent > 0.0 {
            let width = outer_rect.width() * percent;
            let right_bottom = outer_rect.right_bottom();
            let left_top = right_bottom - egui::vec2(width, outer_rect.height());
            let rect = egui::Rect::from_min_max(left_top, right_bottom);
            const GUAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
            ui.painter().rect_filled(rect, corner_radius, GUAGE_COLOR);
        }

        // 블루 팀 게이지를 그립니다.
        let i = Team::Blue as usize;
        let percent = self.capture_point.blue_score();
        let width = inner_rect.width() * percent;
        let height = inner_rect.height();
        let corner_radius = inner_rect.height() * 0.5;
        let left_top = inner_rect.left_top();
        let right_bottom = left_top + egui::vec2(width, height);
        let rect = egui::Rect::from_min_max(left_top, right_bottom);
        ui.painter()
            .rect_filled(inner_rect, corner_radius, egui::Color32::GRAY);
        ui.painter().rect_filled(rect, corner_radius, TEAM_COLOR[i]);

        // 블루팀 라벨을 그립니다.
        let text = "BLUE";
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(24.0 * self.ui_scale, family);
        let center = egui::pos2(outer_rect.center().x, font_center + base_y);
        let offsets = [
            egui::vec2(-1.0, 0.0),
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, -1.0),
            egui::vec2(0.0, 1.0),
        ];

        for offset in offsets {
            ui.painter().text(
                center + offset * self.ui_scale,
                egui::Align2::CENTER_CENTER,
                text,
                font_id.clone(),
                FONT_COLOR,
            );
        }

        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            font_id.clone(),
            TEAM_COLOR[Team::Blue as usize],
        );
    }

    /// 레드 팀 점령도 게이지를 그립니다.
    fn draw_red_team_gauge(&self, ui: &mut egui::Ui) {
        let union_rect = self.score_union_rect();
        let t = self.ui_animation_factor();
        let base_y = -union_rect.height() * (1.0 - t);

        let mut outer_rect = self.red_score_rect;
        let font_center = (self.clip_rect.top() + outer_rect.top()) * 0.5;
        outer_rect.min.y += base_y;
        outer_rect.max.y += base_y;

        let size = egui::Vec2::splat(outer_rect.size().min_elem() * 0.2);
        let mut inner_rect = outer_rect;
        inner_rect.min += size;
        inner_rect.max -= size;

        // 레드 팀 점령도 게이지
        let percent = self.capture_point.red_progress();
        let corner_radius = outer_rect.height() * 0.5;
        ui.painter()
            .rect_filled(outer_rect, corner_radius, egui::Color32::WHITE);
        if percent > 0.0 {
            let width = outer_rect.width() * percent;
            let left_top = outer_rect.left_top();
            let right_bottom = left_top + egui::vec2(width, outer_rect.height());
            let rect = egui::Rect::from_min_max(left_top, right_bottom);
            const GUAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
            ui.painter().rect_filled(rect, corner_radius, GUAGE_COLOR);
        }

        // 레드 팀 게이지를 그립니다.
        let i = Team::Red as usize;
        let percent = self.capture_point.red_score();
        let width = inner_rect.width() * percent;
        let height = inner_rect.height();
        let corner_radius = inner_rect.height() * 0.5;
        let right_bottom = inner_rect.right_bottom();
        let left_top = right_bottom - egui::vec2(width, height);
        let rect = egui::Rect::from_min_max(left_top, right_bottom);
        ui.painter()
            .rect_filled(inner_rect, corner_radius, egui::Color32::GRAY);
        ui.painter().rect_filled(rect, corner_radius, TEAM_COLOR[i]);

        // 레드 팀 라벨을 그립니다.
        let text = "RED";
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(24.0 * self.ui_scale, family);
        let center = egui::pos2(outer_rect.center().x, font_center + base_y);
        let offsets = [
            egui::vec2(-1.0, 0.0),
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, -1.0),
            egui::vec2(0.0, 1.0),
        ];

        for offset in offsets {
            ui.painter().text(
                center + offset * self.ui_scale,
                egui::Align2::CENTER_CENTER,
                text,
                font_id.clone(),
                FONT_COLOR,
            );
        }

        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            font_id.clone(),
            TEAM_COLOR[Team::Red as usize],
        );
    }

    /// 이미지 폰트를 출력합니다.
    fn draw_img_font(&mut self, ctx: &egui::Context) {
        let texture_size = self.img_font_texture.size;
        let ratio = texture_size.x / texture_size.y;
        let height = self.clip_rect.height() * 0.4;
        let width = height * ratio;
        let size = egui::vec2(width, height);
        let center = self.clip_rect.center();
        let rect = egui::Rect::from_center_size(center, size);

        const HALF_SCENE_DURATION: u32 = SCENE_DURATION / 2;
        const FADE_IN_DURATION: u32 = 500;
        let time = self.elapsed_time_ms.saturating_sub(HALF_SCENE_DURATION);
        let t = (time as f32 / FADE_IN_DURATION as f32).min(1.0);
        let t = t * t * (3.0 - 2.0 * t);
        let tint = egui::Color32::from_white_alpha((255.0 * t) as u8);

        egui::Area::new(egui::Id::new("Result_ImgFont"))
            .order(egui::Order::Foreground)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                egui::Image::new(self.img_font_texture)
                    .sense(egui::Sense::empty())
                    .tint(tint)
                    .paint_at(ui, rect);
            });
    }

    /// 팀 상태 인터페이스를 그립니다.
    fn draw_team_status(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Team_Status"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                self.draw_player_status(ui);
            });
    }

    /// 플레이어 상태 인터페이스를 그립니다.
    fn draw_player_status(&self, ui: &mut egui::Ui) {
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        // 데이터를 수집합니다.
        type Query<'a> = (&'a CharacterKind, &'a UserName, &'a (Team, usize));
        let view = world.view::<Query>();
        let mut map = BTreeMap::new();
        for (&uid, &(entity, archetype)) in self.players.iter() {
            if uid == self.uid {
                continue;
            }

            let (&kind, &name, &(team, team_index)) = view
                .get(entity)
                .expect("invalid entity or invalid entity component");
            if team == self.player_team {
                player_execute!(
                    archetype,
                    world,
                    entity,
                    (&HealthData, &SkillCostData),
                    |(health_data, skill_cost_data)| {
                        map.insert(
                            team_index,
                            (kind, name, health_data.percent(), skill_cost_data.percent()),
                        );
                    }
                );
            }
        }

        let t = self.ui_animation_factor();
        let interval = self.clip_rect.height() * 0.01;
        let base_x = -self.clip_rect.width() * 0.5 * (1.0 - t);
        let status_width = self.team_status_rect.width();
        let status_height = (self.team_status_rect.height() - interval * 3.0) / 4.0;

        let mut pos = self.team_status_rect.min + egui::vec2(base_x, 0.0);
        for (kind, name, health, skill_cost) in map.values() {
            // 캐릭터 아이콘
            let texture = self
                .character_icon_textures
                .get(kind)
                .cloned()
                .expect("character icon texture must be preloaded!");
            let ratio = texture.size.x / texture.size.y;
            let icon_height = status_height * 0.7;
            let icon_width = status_height * ratio;
            let size = egui::vec2(icon_width, icon_height);
            let rect = egui::Rect::from_min_size(pos, size);
            egui::Image::new(texture)
                .sense(egui::Sense::empty())
                .paint_at(ui, rect);

            // 닉네임
            let p0 = pos + egui::vec2(icon_width, 0.0);
            let name_width = (status_width - icon_width).max(0.0);
            let name_height = icon_height;
            let center = p0 + egui::vec2(name_width, name_height) * 0.5;
            let text = name.to_string();
            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
            let font_id = egui::FontId::new(14.0 * self.ui_scale, family);
            const OFFSET: [egui::Vec2; 4] = [
                egui::vec2(-1.0, 0.0),
                egui::vec2(1.0, 0.0),
                egui::vec2(0.0, -1.0),
                egui::vec2(0.0, 1.0),
            ];
            for offset in OFFSET {
                ui.painter().text(
                    center + offset * self.ui_scale,
                    egui::Align2::CENTER_CENTER,
                    &text,
                    font_id.clone(),
                    FONT_COLOR,
                );
            }
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                &text,
                font_id,
                egui::Color32::WHITE,
            );

            // 체력
            let (health_p, shield_p) = health.unwrap_or((1.0, 0.0));
            let p1 = pos + egui::vec2(0.0, icon_height);
            let height = status_height * 0.15;
            let size = egui::vec2(status_width, height);
            let rect = egui::Rect::from_min_size(p1, size);
            ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
            let content_rect = rect.scale_from_center(0.95);
            let size = content_rect.size() * egui::vec2(health_p + shield_p, 1.0);
            let rect = egui::Rect::from_min_size(content_rect.min, size);
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(255, 192, 0));
            let size = content_rect.size() * egui::vec2(health_p, 1.0);
            let rect = egui::Rect::from_min_size(content_rect.min, size);
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::LIGHT_GREEN);

            // 스킬 게이지
            let skill_p = skill_cost.unwrap_or(1.0);
            let p2 = p1 + egui::vec2(0.0, height);
            let size = egui::vec2(status_width, height);
            let rect = egui::Rect::from_min_size(p2, size);
            ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
            let content_rect = rect.scale_from_center(0.95);
            let size = content_rect.size() * egui::vec2(skill_p, 1.0);
            let rect = egui::Rect::from_min_size(content_rect.min, size);
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(255, 192, 0));

            pos = pos + egui::vec2(0.0, status_height + interval);
        }
    }
}

impl GameScene for InGameExitScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let device = app.render_device();
        self.regist_layout_texture(device, ui_renderer);
        self.regist_img_font_texture(device, ui_renderer);
        self.regist_weapon_icon_texture(device, ui_renderer);
        self.regist_character_icon_texture(device, ui_renderer);
        self.resize_ui(window, app);

        // 배경음을 추가합니다.
        let decoded = self
            .sound_data_pool
            .get(BG_SOUND_THEME_23)
            .expect("Theme_23 sound must be preloaded!");
        let source = decoded.as_source().repeat_infinite();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.background_volume as f32 / 255.0);
        sink.append(source);
        sink.pause();
        self.background_sounds.push(sink);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        let sink_list = app.sink_list();
        for sink in self.background_sounds.drain(..) {
            sink_list.push(sink);
        }

        ui_renderer.free_texture(&self.layout_texture.id);
        ui_renderer.free_texture(&self.img_font_texture.id);
        ui_renderer.free_texture(&self.weapon_icon_texture.id);
        for texture in self.character_icon_textures.values() {
            ui_renderer.free_texture(&texture.id);
        }
    }

    fn on_enter_background(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
        self.first_mouse_pressed = false;
    }

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        if self.world.is_some() {
            let size = app.window_size();
            let (width, height): (f32, f32) = size.size().into();
            self.camera_aspect_ratio = width / height;

            let device = app.render_device();
            self.create_weighted_blend_oit_resource(size, device);
            self.create_bloom_resource(size, device);
            self.resize_ui(window, app);
        }
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            title,
            message,
            self.sound_data_pool.clone(),
        );
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 효과음을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(UI_NOTICE)
            .expect("UI_Notice sound must be preloaded!");
        let source = decoded.as_source();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.effect_volume as f32 / 255.0);
        sink.append(source);
        sink.play();
        sink.detach();
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
                self.pull_server_data(time_stamp, packet);
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

    fn on_mouse_btn_released(
        &mut self,
        _button: MouseButton,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> bool {
        if !self.first_mouse_pressed {
            let event = AppEvent::CursorDisable;
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
            self.first_mouse_pressed = true;
        }

        return true;
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
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return true,
        };

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

        let delta_lat = dy.to_radians() * offset;
        let delta_lon = dx.to_radians() * offset;

        player_execute!(archetype, world, entity, &mut LatLon, |latlon| {
            latlon.lat = (latlon.lat + delta_lat).clamp(MIN_LATITUDE, MAX_LATITUDE);
            latlon.lon = (latlon.lon + delta_lon) % TAU;
        });

        true
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        let elapsed_time_ms = (elapsed_time_sec * 1000.0) as u32;
        self.elapsed_time_ms = self.elapsed_time_ms.saturating_add(elapsed_time_ms);

        // 배경 음악을 줄입니다.
        let sink_list = app.sink_list();
        if !sink_list.is_empty() {
            let i = self.ui_animation_factor();
            let mut temp = Vec::with_capacity(sink_list.len());
            while let Some(sink) = sink_list.pop() {
                let value = self.background_volume as f32 / 255.0 * i;
                sink.set_volume(value);
                temp.push(sink);
            }

            if self.elapsed_time_ms > 1000 {
                for sink in temp {
                    sink.stop();
                }

                for sink in self.background_sounds.iter() {
                    sink.play();
                }
            } else {
                for sink in temp {
                    sink_list.push(sink);
                }
            }
        }

        if self.elapsed_time_ms > 2000 && !self.play_effect_sound {
            // 효과음을 재생합니다.
            self.play_effect_sound = true;
            let decoded = self
                .sound_data_pool
                .get(UI_VICTORY_ST_01)
                .expect("UI_Victory_ST_01 sound must be preloaded!");
            let source = decoded.as_source();
            let sink = Sink::connect_new(app.audio_mixer());
            sink.set_volume(self.effect_volume as f32 / 255.0);
            sink.append(source);
            sink.play();
            sink.detach();
        }

        // 플레이어를 갱신합니다.
        let (entity, archetype) = self.player_entity();
        let world = match self.world.as_mut() {
            Some(world) => world,
            None => return,
        };

        let i = self.player_character as usize;
        let character_attributes = CHARACTER_ATTRIBUTES[i];
        player_execute!(archetype, world, entity, &ActionState, |action_state| {
            // 시야 상태 타이머를 갱신합니다.
            update_view_state_timer(
                *action_state,
                &mut self.view_state,
                &mut self.view_state_timer,
                character_attributes,
                elapsed_time_ms as u16,
            );
            update_view_state(
                *action_state,
                &mut self.view_state,
                &mut self.view_state_timer,
                character_attributes,
                HeldInput::empty(),
            );
        });
    }

    fn on_post_update(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 게임 장면 지속 시간을 초과한 경우 생략
        if self.elapsed_time_ms < SCENE_DURATION {
            return;
        }

        // 게임 월드를 가져옵니다.
        let mut world = match self.world.take() {
            Some(world) => world,
            None => return,
        };

        // 카메라 엔터티를 정리합니다.
        cleanup(&mut world, self.camera);

        // 총알 엔터티를 정리합니다.
        for entity in self.bullets.values().cloned() {
            cleanup(&mut world, entity);
        }

        let packet = self
            .packet
            .take()
            .expect("the result packet must be exists!");
        let stage = self.stage.take().expect("the stage must be exists!");
        let accum_render_target = self
            .accum_render_target
            .take()
            .expect("the accumulate render target must be exists!");
        let reveal_render_target = self
            .reveal_render_target
            .take()
            .expect("the revealage render target must be exists!");
        let bright_render_target = self
            .bright_render_target
            .take()
            .expect("the brightness render target must be exists!");
        let alpha_blend_pipeline = self
            .alpha_blend_pipeline
            .take()
            .expect("the alpha blending render pipeline must be exists!");
        let gaussian_blur_pipeline = self
            .gaussian_blur_pipeline
            .take()
            .expect("the gaussian blur compute pipeline must be exists!");
        let bloom_pipeline = self
            .bloom_pipeline
            .take()
            .expect("the bloom render pipeline must be exists!");
        let outlines = self.outlines.drain().collect();
        let skybox = self.skybox.take().expect("the skybox must be exists!");
        let direction_light = self
            .direction_light
            .take()
            .expect("the direction light must be exists!");
        let light_resource = self
            .light_resource
            .take()
            .expect("the light shader resource must be exists!");
        // 다음 게임 장면으로 전환합니다.
        let scene = InGameResultScene::new(
            self.locale,
            self.uid,
            self.token,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            packet,
            self.is_player_win,
            self.stage_attributes.clone(),
            self.player_character,
            self.player_team,
            world,
            self.players.clone(),
            stage,
            accum_render_target,
            reveal_render_target,
            bright_render_target,
            alpha_blend_pipeline,
            gaussian_blur_pipeline,
            bloom_pipeline,
            outlines,
            skybox,
            direction_light,
            light_resource,
            self.mesh_pool.clone(),
            self.model_pool.clone(),
            self.motion_pool.clone(),
            self.texture_pool.clone(),
            self.texture_data_pool.clone(),
            self.texture_view_pool.clone(),
            self.sampler_pool.clone(),
            self.sound_data_pool.clone(),
        );
        let flow = GameSceneFlow::Change(Box::new(scene));
        let event = AppEvent::AddGameSceneFlow(flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 이전 프레임에서 사용한 Staging Buffer를 모두 제거합니다.
        self.frame_staging_buffers.clear();

        // 변환 행렬을 갱신합니다.
        {
            let world = match self.world.as_ref() {
                Some(world) => world,
                None => return,
            };

            let child_view = &world.view::<&Child>();
            let sibling_view = &world.view::<&Sibling>();
            let character_view = &world.view::<&CharacterKind>();
            let character_flag_view = &world.view::<&CharacterFlags>();
            let skinning_view = &world.view::<&SkinningAnimation>();
            let collection_view = &world.view::<&BoneCollection>();
            let motion_pool = &self.motion_pool;
            type Query<'a> = (
                &'a ActionState,
                &'a MovementState,
                &'a ActionStateTimer,
                &'a MovementStateTimer,
                &'a LatLon,
            );

            rayon::in_place_scope(|scope| {
                // 각 캐릭터의 애니메이션을 재생합니다.
                for (entity, archetype) in self.players.values().cloned() {
                    // 플레이어가 접속 중이 아닌 경우 건너뜁니다.
                    let flag = character_flag_view
                        .get(entity)
                        .expect("invalid entity or invalid entity component!");
                    if !flag.is_connected() {
                        continue;
                    }

                    scope.spawn(move |_| {
                        player_execute!(
                            archetype,
                            world,
                            entity,
                            Query,
                            |(
                                &action_state,
                                &movement_state,
                                &action_state_timer,
                                &movement_state_timer,
                                &latlon,
                            )| {
                                // 캐릭터 애니메이션을 재생합니다.
                                animate_character(
                                    world,
                                    entity,
                                    archetype,
                                    &motion_pool,
                                    action_state,
                                    movement_state,
                                    action_state_timer,
                                    movement_state_timer,
                                    latlon,
                                    character_view,
                                    skinning_view,
                                    collection_view,
                                );

                                // 캐릭터 계층 구조를 갱신합니다.
                                update_character_hierarchy(
                                    world,
                                    entity,
                                    archetype,
                                    action_state,
                                    child_view,
                                    sibling_view,
                                    character_view,
                                    skinning_view,
                                );
                            }
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
        let draw_call: &Arc<Queue<_>> = &Arc::new(Queue::new());
        {
            let device = app.render_device();
            let world = self.world.as_ref().expect("the world must be exists!");
            let skybox = self.skybox.as_ref().expect("the skybox must be exists!");
            let hierarchy = self.stage.as_ref();
            let camera_entity = self.camera;

            let outline_resources = &self.outlines;
            let child_view = &world.view::<&Child>();
            let sibling_view = &world.view::<&Sibling>();
            let character_team_view = &world.view::<&(Team, usize)>();
            let character_flag_view = &world.view::<&CharacterFlags>();
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

                    draw_call.push((encoder.finish(), staging_buffers));
                });

                // 총알 엔터티의 쉐이더 리소스를 갱신합니다.
                let bullet_entities: Vec<_> = self.bullets.values().cloned().collect();
                scope.spawn(move |_| {
                    let mut staging_buffers = Vec::default();
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                    for entity in bullet_entities {
                        // 총알 엔터티의 계층 구조를 갱신합니다.
                        update_bullet_hierarchy(world, entity, child_view, sibling_view);

                        // 총알 엔터티의 쉐이더 리소스를 갱신합니다.
                        update_bullet_resource(
                            world,
                            entity,
                            &device,
                            &mut encoder,
                            &mut staging_buffers,
                            child_view,
                            sibling_view,
                            mesh_filter_view,
                            skinned_mesh_filter_view,
                            draw_tasks,
                        );
                    }

                    draw_call.push((encoder.finish(), staging_buffers));
                });

                // 캐릭터 엔터티의 쉐이더 리소스를 갱신합니다.
                for (entity, archetype) in self.players.values().cloned() {
                    // 플레이어가 접속 중이 아닌 경우 건너뜁니다.
                    let flag = character_flag_view
                        .get(entity)
                        .expect("invalid entity or invalid entity component!");
                    if !flag.is_connected() {
                        continue;
                    }

                    scope.spawn(move |_| {
                        let mut staging_buffers = Vec::default();
                        let mut encoder = device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        let (team, _team_index) = character_team_view
                            .get(entity)
                            .expect("invalid entity or invalid entity component!");
                        let outline_resource = outline_resources
                            .get(team)
                            .expect("the outline material shader resource must be exists!");
                        update_character_resource(
                            world,
                            entity,
                            archetype,
                            &device,
                            &mut encoder,
                            &mut staging_buffers,
                            outline_resource,
                            child_view,
                            sibling_view,
                            mesh_filter_view,
                            skinned_mesh_filter_view,
                            draw_tasks,
                        );

                        draw_call.push((encoder.finish(), staging_buffers));
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

                        draw_call.push((encoder.finish(), staging_buffers));
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

                        draw_call.push((encoder.finish(), staging_buffers));

                        // 조명이 비추는 영역과 교차하는 엔터티를 수집합니다.
                        let frustum = Frustum::from_mat4(light_proj_view);
                        let mut transform_resources = ShadowMap::default();
                        for (entity, archetype) in player_entities {
                            // 플레이어가 접속 중이 아닌 경우 건너뜁니다.
                            let flag = character_flag_view
                                .get(entity)
                                .expect("invalid entity or invalid entity component!");
                            if !flag.is_connected() {
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

        let mut command_buffers = Vec::new();
        while let Some((encoder, mut buffers)) = draw_call.pop() {
            command_buffers.push(encoder);
            self.frame_staging_buffers.append(&mut buffers);
        }

        let queue = app.render_queue();
        queue.submit(command_buffers);

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
                        depth_slice: None,
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
                        depth_slice: None,
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
                    MaterialKind::Bullet => {
                        draw_bullet(
                            mesh,
                            device,
                            camera_resource,
                            light_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
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
                    MaterialKind::CharacterHaloOutline => {
                        draw_character_halo_outline(
                            mesh,
                            device,
                            camera_resource,
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
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                        depth_slice: None,
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
                        depth_slice: None,
                        view: reveal_render_target.view(),
                        resolve_target: None,
                    }),
                    // 2번 렌더 타겟: bloom
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
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

            for ((mesh, kind), material_resources) in self.transparent_resources.iter() {
                match kind {
                    MaterialKind::EnergyBullet => {
                        draw_energy_bullet(
                            mesh,
                            device,
                            camera_resource,
                            light_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::StageBarrier => {
                        draw_stage_barrier(
                            mesh,
                            device,
                            camera_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    _ => {}
                }
            }
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
                    depth_slice: None,
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

    fn ui_callback(&mut self, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        let ctx = app.egui_ctx();
        self.draw_health_point(ctx);
        self.draw_weapon_info(ctx);
        self.draw_skill_cost_info(ctx);
        self.draw_score(ctx);
        self.draw_team_status(ctx);
        self.draw_img_font(ctx);
    }
}
