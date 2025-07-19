use std::sync::Arc;

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
        update_action_state_timer, update_movement_state_timer, ActionState, ActionStateTimer,
        BulletData, CharacterFlags, CharacterKind, GameTier, HeldInput, InGamePlayerResultData,
        LatLon, LoginToken, MovementState, MovementStateTimer, ProfileIcon, SkillCostData,
        StageAttributes, Team, UserId, MAX_IN_GAME_PLAYERS,
    },
    protocol::InGameFinishPacket,
};
use mod_parallelism::collections::Queue;
use mod_physics::object3d::Frustum;
use mod_render::{UiRenderer, SWAPCHAIN_FORMAT};
use rodio::Sink;
use winit::window::Window;

use crate::{
    asset::{
        cull_stage_entities, MeshPool, ModelPool, MotionPool, SamplerPool, SoundDataPool,
        StageBoundingVolumnHierarchy, TextureDataPool, TexturePool, TextureViewPool,
        CHARACTER_IMG_URI, EMBLEM_BG_URI, IMG_FONT_DRAW, IMG_FONT_LOSE_URI, IMG_FONT_WIN_URI,
        NOTOSANS_BOLD, NOTOSANS_REGULAR, PROFILE_ICON_URI, UI_NOTICE,
    },
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage,
        clear_render_target_with_skybox, collect_character_resource, collect_stage_resource,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_bullet,
        draw_character, draw_character_eye_mouth, draw_character_halo, draw_energy_bullet,
        draw_stage, draw_tree, update_camera_and_skybox_resource, update_character_hierarchy,
        update_character_resource, update_stage_hierarchy, update_stage_resource,
        AccumRenderTarget, AlphaBlendPipeline, BakeList, BloomPipeline, BoneCollection,
        BrightRenderTarget, Camera, CameraResource, CameraUniform, Child, DirectionLight,
        GaussianBlurPipeline, GlobalLightDataLayout, LightSetResource, LightTransformDataLayout,
        MaterialKind, MeshRenderer, OpaqueMap, PlayerArchetype, Projection, RenderTask,
        RevealRenderTarget, ShadowMap, Sibling, SkinnedMeshRenderer, SkinningAnimation, Skybox,
        ToParentTrans, TransparentMap, WorldTransform, CHARACTER_ATTRIBUTES,
    },
    config::{Locale, NUM_LOCALE},
    player_execute,
    scenes::{
        FatalErrorSceneLayer, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS, FONT_COLOR,
    },
};

const SCENE_DURATION: u16 = 13_500;

/// 애플리케이션 언어에 따른 `적을 처치한 횟수` 텍스트
const KNOCK_OUT_TEXTS: [&'static str; NUM_LOCALE] = ["처치 횟수"];
/// 애플리케이션 언어에 따른 `전투 불능 횟수` 텍스트
const DOWNED_TEXTS: [&'static str; NUM_LOCALE] = ["전투 불능 횟수"];
/// 애플리케이션 언어에 따른 `적에게 준 피해량`텍스트
const DAMAGE_DEALT_TEXTS: [&'static str; NUM_LOCALE] = ["준 피해량"];
/// 애플리케이션 언어에 따른 `적에게 입은 피해량`텍스트
const DAMAGE_TAKEN_TEXTS: [&'static str; NUM_LOCALE] = ["입은 피해량"];
/// 애플리케이션 언어에 따른 `팀을 회복시킨 회복량`텍스트
const HEALING_GIVEN_TEXTS: [&'static str; NUM_LOCALE] = ["팀 회복량"];

pub struct InGameResultScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    _token: LoginToken,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 게임 장면 경과 시간
    elapsed_time_ms: u16,

    /// 게임 결과 패킷
    packet: InGameFinishPacket,
    /// 플레이어 우승 여부, 비겼을 경우 `None`
    is_player_win: Option<bool>,

    /// 지형 속성 데이터입니다.
    stage_attributes: Arc<StageAttributes>,

    /// 플레이어 캐릭터 종류
    _player_character: CharacterKind,
    /// 플레이어가 속한 팀
    player_team: Team,

    /// 게임 월드
    world: World,

    /// 카메라 엔터티
    camera: Entity,
    /// 카메라 Fov-y 값
    camera_fov_y: f32,

    /// 승리 팀 플레이어 엔터티
    winner: HashMap<UserId, (Entity, PlayerArchetype)>,
    /// 플레이어 엔터티
    players: HashMap<UserId, (Entity, PlayerArchetype)>,
    /// 스테이지 엔터티
    stage: Option<StageBoundingVolumnHierarchy>,

    /// 내 팀의 결과 데이터
    my_team: Vec<InGamePlayerResultData>,
    /// 다른 팀의 결과 데이터
    other_team: Vec<InGamePlayerResultData>,

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

    /// 결과를 표시하는 영역
    result_rect: egui::Rect,

    /// 이미지 폰트 텍스처입니다.
    img_font_texture: egui::load::SizedTexture,
    /// 프로필 배경 텍스처
    profile_bg_textures: HashMap<GameTier, egui::load::SizedTexture>,
    /// 프로필 아이콘 텍스처
    profile_icon_textures: HashMap<ProfileIcon, egui::load::SizedTexture>,
    /// 캐릭터 이미지 텍스처
    character_img_textures: HashMap<CharacterKind, egui::load::SizedTexture>,

    /// 메쉬 풀 객체입니다.
    _mesh_pool: MeshPool,
    /// 모델 풀 객체입니다.
    _model_pool: ModelPool,
    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 데이터 풀 객체입니다.
    _texture_data_pool: TextureDataPool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    _sampler_pool: SamplerPool,
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InGameResultScene {
    /// 새로운 `InGameResultScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        packet: InGameFinishPacket,
        is_player_win: Option<bool>,
        stage_attributes: Arc<StageAttributes>,
        player_character: CharacterKind,
        player_team: Team,
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
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            uid,
            _token: token,
            background_volume,
            effect_volume,
            voice_volume,
            elapsed_time_ms: 0,
            packet,
            is_player_win,
            stage_attributes,
            _player_character: player_character,
            player_team,
            world,
            camera: Entity::DANGLING,
            camera_fov_y: 1.0,
            winner: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new()),
            players,
            stage: Some(stage),
            my_team: Vec::default(),
            other_team: Vec::default(),
            frame_staging_buffers: Vec::default(),
            accum_render_target: Some(accum_render_target),
            reveal_render_target: Some(reveal_render_target),
            bright_render_target: Some(bright_render_target),
            alpha_blend_pipeline: Some(alpha_blend_pipeline),
            gaussian_blur_pipeline: Some(gaussian_blur_pipeline),
            bloom_pipeline: Some(bloom_pipeline),
            skybox: Some(skybox),
            direction_light: Some(direction_light),
            light_resource: Some(light_resource),
            bake_list: BakeList::default(),
            opaque_resources: OpaqueMap::default(),
            transparent_resources: TransparentMap::default(),
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            result_rect: egui::Rect::ZERO,
            img_font_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            profile_bg_textures: HashMap::default(),
            profile_icon_textures: HashMap::default(),
            character_img_textures: HashMap::default(),
            _mesh_pool: mesh_pool,
            _model_pool: model_pool,
            motion_pool,
            texture_pool,
            _texture_data_pool: texture_data_pool,
            texture_view_pool,
            _sampler_pool: sampler_pool,
            sound_data_pool,
        }
    }

    // 플레이어를 초기화합니다.
    fn setup_players(&mut self) {
        // 우승 팀을 판단합니다.
        let winner_team = match self.packet.winner {
            Some(team) => team,
            None => self.player_team,
        };

        // 우승팀 플레이어 캐릭터의 위치와 상태를 초기화합니다.
        let team_view = self.world.view::<&(Team, usize)>();
        for (&uid, &(entity, archetype)) in self.players.iter() {
            // 플레이어가 속한 팀을 가져옵니다.
            let &(team, team_index) = team_view
                .get(entity)
                .expect("invalid entity or invalid entity component!");

            if winner_team == team {
                type Query<'a> = (
                    &'a mut ActionState,
                    &'a mut ActionStateTimer,
                    &'a mut MovementState,
                    &'a mut MovementStateTimer,
                    &'a mut ToParentTrans,
                );
                player_execute!(archetype, &self.world, entity, Query, |(
                    action_state,
                    action_state_timer,
                    movement_state,
                    movement_state_timer,
                    transform,
                )| {
                    *action_state = ActionState::VictoryStart;
                    *action_state_timer = ActionStateTimer::new(0);
                    *movement_state = MovementState::Idle;
                    *movement_state_timer = MovementStateTimer::new(0);

                    let rotation = self.stage_attributes.winner_rotation;
                    let translation = self.stage_attributes.winner_positions[team_index];
                    transform.set_rotation_translation(rotation.into(), translation.into());
                });

                // 승리 팀 플레이어에 추가합니다.
                self.winner.insert(uid, (entity, archetype));
            }
        }
    }

    /// 카메라 엔터티를 생성합니다.
    fn create_camera(&mut self, size: WindowSize, device: &wgpu::Device) {
        let world = &mut self.world;
        let rotation = self.stage_attributes.camera_rotation;
        let translation = self.stage_attributes.camera_position;
        let transform = glam::Mat4::from_rotation_translation(rotation.into(), translation.into());

        // 카메라 컴포넌트 데이터를 생성합니다.
        let fov_y_radians = self.stage_attributes.camera_fov_y;
        let local_transform = ToParentTrans(transform);
        let world_transform = WorldTransform(transform);
        let (width, height): (f32, f32) = size.size().into();
        let aspect_ratio = width / height;
        let projection = Projection::perspective(fov_y_radians, aspect_ratio, 0.1, 50.0);
        let proj_view = projection.0 * world_transform.to_view_trans();
        let frustum = Frustum::from_mat4(proj_view);

        // 카메라 쉐이더 리소스를 생성합니다.
        let label = format!("InGameResult(Camera)");
        let camera_uniform = CameraUniform::uninit(Some(&label), device);
        let camera_resource = CameraResource::new(Some(&label), device, &camera_uniform);

        // 엔터티를 생성합니다.
        self.camera_fov_y = fov_y_radians;
        self.camera = world.spawn((
            (Camera, local_transform),
            (Camera, world_transform),
            projection,
            frustum,
            camera_uniform,
            camera_resource,
        ));
    }

    /// Weighted-Blended OIT에 사용되는 렌더 타겟과 파이프라인을 생성합니다.
    fn create_weighted_blend_oit_resource(&mut self, size: WindowSize, device: &wgpu::Device) {
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

    /// 프로필 배경 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_bg_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
        tier: GameTier,
    ) {
        // 프로필 배경 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(EMBLEM_BG_URI)
            .expect("Emblem_BG texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 프로필 배경 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: tier as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_bg_textures.insert(
            tier,
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 프로필 아이콘 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_profile_icon_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
        icon: ProfileIcon,
    ) {
        // 프로필 아이콘 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(PROFILE_ICON_URI)
            .expect("Profile_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 프로필 아이콘 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: icon as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.profile_icon_textures.insert(
            icon,
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 캐릭터 이미지 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_character_img_texture(
        &mut self,
        device: &wgpu::Device,
        ui_renderer: &mut UiRenderer,
        character_kind: CharacterKind,
    ) {
        // 캐릭터 편성 배경화면 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(CHARACTER_IMG_URI)
            .expect("Character_Img texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture = self.texture_view_pool.get_or_init(
            &texture,
            &wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: character_kind as u32,
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            ui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        // 등록된 텍스처 정보를 저장합니다.
        self.character_img_textures.insert(
            character_kind,
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    fn resize_ui(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형 영역의 크기를 재조정합니다.
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        (self.clip_rect, self.ui_scale) = Self::resize_clip_rect(viewport, scale_factor);

        // 임무 결과 배경 영역의 크기를 재조정합니다.
        self.result_rect = Self::resize_result_rect(&self.clip_rect);
    }

    /// 애니메이션 값을 가져옵니다.
    fn ui_animation_factor(&self) -> f32 {
        let time = self.elapsed_time_ms.saturating_sub(3_000);
        time.min(500) as f32 / 500.0
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

    /// 결과를 표시하는 영역의 크기를 재조정합니다.
    fn resize_result_rect(clip_rect: &egui::Rect) -> egui::Rect {
        let width = clip_rect.width() * 1.0;
        let height = width * 0.35;
        let size = egui::vec2(width, height);
        let center = clip_rect.center();
        egui::Rect::from_center_size(center, size)
    }

    /// 결과를 표시하는 영역의 콘텐츠 크기를 반환합니다.
    fn result_rect_content_area(&self) -> egui::Rect {
        self.result_rect.scale_from_center(0.95)
    }

    /// 임무 결과 배경 화면을 그립니다.
    fn draw_result_background(&self, ctx: &egui::Context) {
        let t = self.ui_animation_factor();
        let fill_color = egui::Color32::from_black_alpha((192.0 * t) as u8);
        egui::Area::new(egui::Id::new("Mission_Result_Bg"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                ui.painter().rect_filled(self.clip_rect, 0.0, fill_color);
            });
    }

    /// 결과 화면을 그립니다.
    fn draw_result_content(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Game_Result"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let content_rect = self.result_rect_content_area();
                let interval = content_rect.width() * 0.05;
                let width = (content_rect.width() - interval * 4.0) / 5.0;
                let height = content_rect.height();

                let num_players = self.my_team.len();
                let total_width =
                    width * num_players as f32 + interval * num_players.saturating_sub(1) as f32;
                let min = content_rect.center_top() - egui::vec2(total_width * 0.5, 0.0);
                for (i, player) in self.my_team.iter().enumerate() {
                    self.draw_player_data(i, min.x, min.y, width, height, interval, player, ui);
                }

                let x = content_rect.center().x;
                let y = content_rect.bottom() + content_rect.height() * 0.05;
                let center = egui::pos2(x, y);
                self.draw_information_label(center, ui);
            });
    }

    /// 안내 문구 라벨을 그립니다.
    fn draw_information_label(&self, center: egui::Pos2, ui: &mut egui::Ui) {
        let t = self.ui_animation_factor();
        let tint = egui::Color32::from_white_alpha((255.0 * t) as u8);
        let remaining_time_ms = SCENE_DURATION.saturating_sub(self.elapsed_time_ms);
        let remaining_time_sec = remaining_time_ms as f32 / 1000.0;
        let text = match self.locale {
            Locale::KOR => format!(
                "{}초 뒤 자동으로 결과 화면에서 나갑니다.",
                remaining_time_sec.ceil() as u32
            ),
        };
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(18.0 * self.ui_scale, family);

        const OFFSETS: [egui::Vec2; 4] = [
            egui::vec2(-1.0, 0.0),
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, -1.0),
            egui::vec2(0.0, 1.0),
        ];
        for offset in OFFSETS {
            ui.painter().text(
                center + offset * self.ui_scale,
                egui::Align2::CENTER_CENTER,
                &text,
                font_id.clone(),
                egui::Color32::WHITE * tint,
            );
        }

        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            &text,
            font_id,
            FONT_COLOR * tint,
        );
    }

    /// 플레이어 데이터를 그립니다.
    fn draw_player_data(
        &self,
        i: usize,
        beg_x: f32,
        beg_y: f32,
        width: f32,
        height: f32,
        interval: f32,
        player: &InGamePlayerResultData,
        ui: &mut egui::Ui,
    ) {
        let t = self.ui_animation_factor();
        let tint = egui::Color32::from_white_alpha((255.0 * t) as u8);

        // 배경
        let min_x = beg_x + (width + interval) * i as f32;
        let min_y = beg_y;
        let min = egui::pos2(min_x, min_y);
        let size = egui::vec2(width, height);
        let rect = egui::Rect::from_min_size(min, size);
        let margin = rect.size().min_elem() * 0.05;
        let corner_radius = margin;
        let stroke = match self.uid == player.uid {
            true => egui::Stroke::new(
                4.0 * self.ui_scale,
                egui::Color32::from_rgb(242, 201, 76) * tint,
            ),
            false => egui::Stroke::new(1.0 * self.ui_scale, egui::Color32::BLACK * tint),
        };
        ui.painter().rect(
            rect,
            corner_radius,
            egui::Color32::WHITE * tint,
            stroke,
            egui::StrokeKind::Middle,
        );

        // 프로필 배경
        let content_rect = rect.scale_from_center(0.96);
        let texture = self
            .profile_bg_textures
            .get(&player.tier())
            .cloned()
            .unwrap();
        let ratio = texture.size.x / texture.size.y;
        let width = content_rect.width();
        let height = width / ratio;
        let size = egui::vec2(width, height);
        let min = content_rect.left_top();
        let profile_rect = egui::Rect::from_min_size(min, size);
        egui::Image::new(texture)
            .tint(tint)
            .sense(egui::Sense::empty())
            .paint_at(ui, profile_rect);

        // 프로필 아이콘
        let texture = self
            .profile_icon_textures
            .get(&player.profile_icon)
            .cloned()
            .unwrap();
        let ratio = texture.size.x / texture.size.y;
        let height = profile_rect.height() * 0.85;
        let width = height * ratio;
        let size = egui::vec2(width, height);
        let min = min + egui::vec2(0.0, profile_rect.height() * 0.05);
        let icon_rect = egui::Rect::from_min_size(min, size);
        egui::Image::new(texture)
            .tint(tint)
            .sense(egui::Sense::empty())
            .paint_at(ui, icon_rect);

        // 사용자 이름
        let text = player.name.to_string();
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(FONT_COLOR * tint);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);
        let label_rect = egui::Rect::from_min_max(
            profile_rect.center_top() - egui::vec2(profile_rect.width() * 0.25, 0.0),
            profile_rect.max,
        );
        ui.put(label_rect, label);

        // 캐릭터 이미지
        let texture = self
            .character_img_textures
            .get(&player.character_kind)
            .cloned()
            .unwrap();
        let ratio = texture.size.x / texture.size.y;
        let width = content_rect.width() * 0.6;
        let height = width / ratio;
        let size = egui::vec2(width, height);
        let center = content_rect.center_top()
            + egui::vec2(0.0, profile_rect.height() + margin + height * 0.5);
        let ch_rect = egui::Rect::from_center_size(center, size);
        egui::Image::new(texture)
            .tint(tint)
            .sense(egui::Sense::empty())
            .paint_at(ui, ch_rect);

        // 제압 횟수
        let cursor_x = content_rect.left();
        let cursor_y = center.y + height * 0.5;
        let width = content_rect.width() * 0.9;
        let height = content_rect.bottom() - cursor_y;
        let min = egui::pos2(cursor_x, cursor_y);
        let size = egui::vec2(width, height);
        let i = self.locale as usize;
        egui::Area::new(egui::Id::new("Player_Data"))
            .order(egui::Order::Middle)
            .fixed_pos(min)
            .default_size(size)
            .show(ui.ctx(), |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let text = KNOCK_OUT_TEXTS[i];
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(text)
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(&player.kill_count.to_string())
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let text = DOWNED_TEXTS[i];
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(text)
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(&player.retreat_count.to_string())
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let text = DAMAGE_DEALT_TEXTS[i];
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(text)
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(&player.damage_dealt.to_string())
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let text = DAMAGE_TAKEN_TEXTS[i];
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(text)
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(&player.damage_taken.to_string())
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let text = HEALING_GIVEN_TEXTS[i];
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(text)
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
                            let font_id = egui::FontId::new(12.0 * self.ui_scale, family);
                            let text = egui::RichText::new(&player.healing_given.to_string())
                                .font(font_id)
                                .color(FONT_COLOR * tint);
                            let label = egui::Label::new(text)
                                .sense(egui::Sense::empty())
                                .selectable(false);
                            ui.add(label);
                        });
                    });
                });
            });
    }

    fn draw_result(&mut self, ctx: &egui::Context) {
        let t = self.ui_animation_factor();
        let tint = egui::Color32::from_white_alpha((255.0 * t) as u8);
        egui::Area::new(egui::Id::new("Game_Result"))
            .order(egui::Order::Background)
            .sense(egui::Sense::empty())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let ratio = self.img_font_texture.size.x / self.img_font_texture.size.y;
                let height = self.clip_rect.height() * 0.2;
                let width = height * ratio;
                let size = egui::vec2(width, height);
                let min = self.clip_rect.left_top();
                let rect = egui::Rect::from_min_size(min, size);
                egui::Image::new(self.img_font_texture)
                    .tint(tint)
                    .paint_at(ui, rect);
            });
    }
}

impl GameScene for InGameResultScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        // 마우스 커서를 활성화합니다.
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 플레이어를 초기화합니다.
        let size = app.window_size();
        let device = app.render_device();
        self.setup_players();
        self.create_camera(size, device);

        // 이미지 폰트 텍스처를 등록합니다.
        self.regist_img_font_texture(device, ui_renderer);

        // 프로필 아이콘과 프로필 배경, 캐릭터 이미지 텍스처를 등록합니다.
        let mut tier_set = HashSet::default();
        let mut icon_set = HashSet::default();
        let mut character_set = HashSet::default();
        for data in self.packet.players.iter() {
            tier_set.insert(data.tier());
            icon_set.insert(data.profile_icon);
            character_set.insert(data.character_kind);
        }
        for tier in tier_set {
            self.regist_profile_bg_texture(device, ui_renderer, tier);
        }
        for icon in icon_set {
            self.regist_profile_icon_texture(device, ui_renderer, icon);
        }
        for kind in character_set {
            self.regist_character_img_texture(device, ui_renderer, kind);
        }

        // 팀 데이터를 가져옵니다.
        (self.my_team, self.other_team) = self
            .packet
            .players
            .drain(..)
            .partition(|data| data.team() == self.player_team);

        self.my_team.sort_by_key(|data| data.team_index());
        self.other_team.sort_by_key(|data| data.team_index());

        self.resize_ui(window, app);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        // 현재 재생 중인 배경 음을 중지합니다.
        let sink_list = app.sink_list();
        while let Some(sink) = sink_list.pop() {
            sink.stop();
        }

        let iterator = self
            .profile_bg_textures
            .values()
            .chain(self.profile_icon_textures.values())
            .chain(self.character_img_textures.values());
        for texture in iterator {
            ui_renderer.free_texture(&texture.id);
        }
        ui_renderer.free_texture(&self.img_font_texture.id);
    }

    fn on_enter_foreground(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 마우스 커서를 활성화합니다.
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_enter_background(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 마우스 커서를 활성화합니다.
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        let size = app.window_size();
        let device = app.render_device();
        self.create_weighted_blend_oit_resource(size, device);
        self.create_bloom_resource(size, device);
        self.resize_ui(window, app);
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

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        let elapsed_time_ms = (elapsed_time_sec * 1000.0) as u16;
        self.elapsed_time_ms = self.elapsed_time_ms.saturating_add(elapsed_time_ms);

        // 플레이어 상태를 갱신합니다.
        type Query<'a> = (
            &'a mut ActionState,
            &'a mut ActionStateTimer,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
        );
        let world = &self.world;
        let character_view = &world.view::<&CharacterKind>();
        rayon::in_place_scope(|scope| {
            for (entity, archetype) in self.winner.values().cloned() {
                scope.spawn(move |_| {
                    // 캐릭터 속성 데이터를 가져옵니다.
                    let &character_kind = character_view
                        .get(entity)
                        .expect("invalid entity or invalid entity component!");
                    let i = character_kind as usize;
                    let character_attributes = CHARACTER_ATTRIBUTES[i];

                    player_execute!(archetype, world, entity, Query, |(
                        action_state,
                        action_state_timer,
                        movement_state,
                        movement_state_timer,
                    )| {
                        update_action_state_timer(
                            HeldInput::empty(),
                            &mut BulletData::default(),
                            &mut SkillCostData::default(),
                            action_state,
                            action_state_timer,
                            character_attributes,
                            elapsed_time_ms,
                            &mut vec![],
                        );
                        update_movement_state_timer(
                            *action_state,
                            movement_state,
                            movement_state_timer,
                            character_attributes,
                            elapsed_time_ms,
                        );
                    });
                });
            }
        });
    }

    fn on_post_update(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 장면 지속 시간을 초과한 경우 이전 장면으로 되돌아갑니다.
        if self.elapsed_time_ms >= SCENE_DURATION {
            let flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        // 이전 프레임에서 사용한 Staging Buffer를 모두 제거합니다.
        self.frame_staging_buffers.clear();

        // 변환 행렬을 갱신합니다.
        {
            let world = &self.world;
            let child_view = &world.view::<&Child>();
            let sibling_view = &world.view::<&Sibling>();
            let character_view = &world.view::<&CharacterKind>();
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
                for (entity, archetype) in self.winner.values().cloned() {
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

        let draw_tasks: &Arc<Queue<_>> = &Arc::new(Queue::new());
        let bake_tasks: &Arc<Queue<_>> = &Arc::new(Queue::new());
        let draw_call: &Arc<Queue<_>> = &Arc::new(Queue::new());
        {
            let device = app.render_device();
            let world = &self.world;
            let hierarchy = &self.stage;
            let skybox = self.skybox.as_ref().expect("the skybox must be exists!");
            let camera_entity = self.camera;

            let child_view = &world.view::<&Child>();
            let sibling_view = &world.view::<&Sibling>();
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

                // 캐릭터 엔터티의 쉐이더 리소스를 갱신합니다.
                for (entity, archetype) in self.winner.values().cloned() {
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

        // 카메라 쉐이더 리소스를 가져옵니다.
        let mut query = self
            .world
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
        if self.elapsed_time_ms < 3000 {
            return;
        }

        let ctx = app.egui_ctx();
        self.draw_result_background(ctx);
        self.draw_result_content(ctx);
        self.draw_result(ctx);
    }
}
