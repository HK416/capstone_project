use std::{sync::Arc, time::Instant};

use ahash::HashMap;
use hecs::{Entity, ViewBorrow, World};
use mod_app::{
    app::AppHandle,
    etc::{AppEvent, Viewport, WindowSize},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, CharacterKind, GameInputBits, LoginToken, StageKind, UserId,
    },
    protocol::{InGamePullPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_physics::object3d::Frustum;
use mod_render::{UiRenderer, SWAPCHAIN_FORMAT};
use winit::{event::MouseButton, window::Window};

use crate::{
    asset::{
        cull_stage_entities, MeshPool, ModelPool, MotionPool, SamplerPool,
        StageBoundingVolumnHierarchy, TextureDataPool, TexturePool, TextureViewPool,
        HUD_LAYOUT_URI_03, IMG_FONT_MISSION_URI, NOTOSANS_BOLD,
    },
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage, cleanup,
        clear_render_target_with_skybox, collect_character_resource, collect_stage_resource,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_character,
        draw_character_eye_mouth, draw_character_halo, draw_stage, draw_tree,
        local_transform_query_mut, update_action_state_timer, update_character_hierarchy,
        update_character_resource, update_stage_hierarchy, update_stage_resource,
        AccumRenderTarget, AlphaBlendPipeline, AnimationQuery, BakeList, BloomPipeline,
        BoneCollection, BrightRenderTarget, Camera, CameraDataLayout, CameraResource,
        CameraUniform, Child, DirectionLight, GaussianBlurPipeline, GlobalLightDataLayout,
        LightSetResource, LightTransformDataLayout, MaterialKind, MeshRenderer, OpaqueMap,
        PlayerArchetype, Projection, RenderTask, RevealRenderTarget, ShadowMap, ShadowResource,
        Sibling, SkinnedMeshRenderer, Skybox, SkyboxDataLayout, ToParentTrans, TransparentMap,
        WorldTransform, CHARACTER_ATTRIBUTES,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, InGameRunScene, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS, FONT_COLOR,
    },
};

/// 카메라의 Fov-y 값 (단위: 라디안)
const CAMERA_FOV_Y: f32 = 60f32.to_radians();
/// Ui 레이아웃 애니메이션 시간 (단위: ms)
const MAX_ANIME_TIME: u16 = 1250;

/// 애플리케이션 표시 언어에 따른 게임 방법 안내 텍스트
const MESSAGE_TEXTS: [&'static str; NUM_LOCALE] = ["맵 중앙의 목표 구역을 먼저 선점하는 팀이 승리"];

/// 게임 시작 전 대기하는 장면입니다.
pub struct InGameEnterScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 게임 스테이지 종류
    stage_kind: StageKind,
    /// 남은 대기 시간입니다.
    remaining_time_ms: u16,
    /// 레이아웃 이동 시간입니다.
    animation_time_ms: u16,
    /// 첫 번째 마우스 눌림 여부 플래그
    first_mouse_pressed: bool,

    /// 게임 월드
    world: Option<World>,
    /// 카메라 엔터티
    camera: Entity,
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

    /// Ui 스케일
    ui_scale: f32,
    /// 클립 사각형 영역
    clip_rect: egui::Rect,
    /// 이미지 폰트 텍스처
    img_font_texture: egui::load::SizedTexture,
    /// 폰트 배경 텍스처
    layout_texture: egui::load::SizedTexture,
    /// 폰트 배경 사각형 영역
    layout_rect: egui::Rect,

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

impl InGameEnterScene {
    /// 새로운 `InGameEnterScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        stage_kind: StageKind,
        remaining_time_ms: u16,
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
            locale,
            uid,
            token,
            stage_kind,
            remaining_time_ms,
            animation_time_ms: 0,
            first_mouse_pressed: false,
            world: Some(world),
            camera: Entity::DANGLING,
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
            ui_scale: 1.0,
            clip_rect: egui::Rect::ZERO,
            img_font_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            layout_texture: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
            layout_rect: egui::Rect::ZERO,
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

    /// 플레이어 캐릭터를 설정합니다.
    fn setup_player(&mut self) {
        // 플레이어 캐릭터를 초기화 설정합니다.
        let (entity, _archetype) = self.player_entity();
        let world = self.world.as_mut().expect("the world must be exists!");
        let (_, action_state) = world
            .query_one_mut::<&mut (ActionState, ActionState)>(entity)
            .expect("invalid entity or invalid entity component!");

        *action_state = ActionState::Callsign;
    }

    /// 카메라 엔터티를 생성합니다.
    fn create_camera(&mut self, size: WindowSize, device: &wgpu::Device) {
        // 플레이어 캐릭터의 위치를 가져옵니다.
        let (entity, archetype) = self.player_entity();
        let world = self.world.as_mut().expect("the world must be exists!");
        let transform = local_transform_query_mut(world, entity, archetype);

        // 카메라의 위치와 방향을 설정합니다.
        let pivot = transform.get_translation() + glam::Vec3A::Y * 0.6;
        let translation = pivot
            + transform.get_right_vector() * 1.0
            + transform.get_look_vector() * 1.0
            + glam::Vec3A::Y * 0.05;
        let look = (pivot - translation).normalize();
        let right = glam::Vec3A::Y.cross(look);
        let up = look.cross(right);
        let transform = glam::Mat4::from_mat3_translation(
            glam::mat3(right.into(), up.into(), look.into()),
            translation.into(),
        );

        // 카메라 컴포넌트 데이터를 생성합니다.
        let local_transform = ToParentTrans(transform);
        let world_transform = WorldTransform(transform);
        let (width, height): (f32, f32) = size.size().into();
        let aspect_ratio = width / height;
        let projection = Projection::perspective(CAMERA_FOV_Y, aspect_ratio, 0.1, 50.0);
        let proj_view = projection.0 * world_transform.to_view_trans();
        let frustum = Frustum::from_mat4(proj_view);

        // 카메라 쉐이더 리소스를 생성합니다.
        let label = format!("InGameEnter(Camera)");
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

    /// 스테이지 엔터티에 대해 카메라 뷰 프러스텀 컬링을 수행합니다.
    ///
    /// # Note
    /// 이 함수는 카메라의 월드 변환 행렬을 갱신한 후 호출되어야 합니다.
    ///
    fn cull_stage_entities(&mut self) {
        // 카메라의 뷰 프러스텀을 가져옵니다.
        let world = self.world.as_mut().expect("the world must be exists!");
        let frustum = world
            .query_one_mut::<&Frustum>(self.camera)
            .expect("invalid entity or invalid entity component!");

        // 엔터티를 수집합니다.
        if let Some(hierarchy) = &self.stage {
            self.culling_stage_entities = cull_stage_entities(frustum, hierarchy);
        }
    }

    /// 배경 레이아웃 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_hud_layout_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(HUD_LAYOUT_URI_03)
            .expect("HUD_Layout_03 texture must be preloaded!");
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

    /// 이미지 폰트 텍스처를 Ui 렌더러에 등록합니다.
    fn regist_img_font_texture(&mut self, device: &wgpu::Device, ui_renderer: &mut UiRenderer) {
        // 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(IMG_FONT_MISSION_URI)
            .expect("ImgFont_Mission texture must be preloaded!");
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

    /// 카메라와 스카이박스를 갱신합니다.
    fn update_camera_and_skybox(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        // 카메라 엔터티의 요소를 가져옵니다.
        type Q<'a> = (
            &'a CameraUniform,
            &'a (Camera, WorldTransform),
            &'a Projection,
            &'a mut Frustum,
        );
        let mut query = world.query_one::<Q>(self.camera).expect("invalid entity!");
        let (camera_uniform, (_, world_transform), projection, frustum) =
            query.get().expect("invalid entity component!");

        // 카메라 유니폼 버퍼를 갱신합니다.
        let position_w = world_transform.get_translation();
        let proj_view = projection.0 * world_transform.to_view_trans();
        let data = CameraDataLayout {
            position_w: position_w.to_array(),
            proj_view: proj_view.to_cols_array(),
            ..Default::default()
        };
        camera_uniform.update(device, encoder, staging_buffers, data);

        // 카메라 절두체를 갱신합니다.
        *frustum = Frustum::from_mat4(proj_view);

        // 스카이박스 유니폼 버퍼를 갱신합니다.
        let skybox = self.skybox.as_ref().expect("the skybox must be exists!");
        let data = SkyboxDataLayout {
            proj_view: proj_view.to_cols_array(),
            color: [1.0, 1.0, 1.0],
            ..Default::default()
        };
        skybox
            .uniform
            .update(device, encoder, staging_buffers, data);
    }

    /// 캐릭터 애니메이션을 재생합니다.
    fn update_player_character(&self, elapsed_time_ms: u16) {
        type Q<'a> = (
            &'a CharacterKind,
            &'a mut (ActionState, ActionState),
            &'a mut ActionStateTimer,
        );

        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };
        let (entity, _archetype) = self.player_entity();
        let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
        let (&character_kind, (prev_action_state, action_state), action_state_timer) =
            query.get().expect("invalid entity component!");

        // 플레이어 엔터티의 행동 상태를 갱신합니다.
        let i = character_kind as usize;
        let character_attributes = CHARACTER_ATTRIBUTES[i];
        update_action_state_timer(
            GameInputBits::empty(),
            action_state,
            action_state_timer,
            character_attributes,
            elapsed_time_ms,
        );
    }

    /// 조명 쉐이더 리소스를 갱신합니다.
    ///
    /// # Note
    /// 이 함수는 카메라의 계층 구조가 갱신된 후 호출되어야 합니다.
    ///
    fn update_light_resource(
        &self,
        device: Arc<wgpu::Device>,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        mesh_filter_view: &ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &ViewBorrow<'_, SkinnedMeshRenderer>,
        bake_tasks: &Arc<Queue<(Arc<ShadowResource>, ShadowMap)>>,
        window_size: WindowSize,
    ) {
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        let hierarchy = self.stage.as_ref().expect("the stage must be exists!");
        let light_resource = self
            .light_resource
            .as_ref()
            .expect("the light shader resource must be exists!");

        let bake_tasks_cloned = bake_tasks.clone();
        rayon::in_place_scope(|scope| {
            let device_cloned = device.clone();
            let bake_tasks = bake_tasks_cloned.clone();
            scope.spawn(move |_| {
                let mut staging_buffers = Vec::default();
                let mut encoder = device_cloned
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                if let Some(directional_light) = self.direction_light.as_ref() {
                    // 카메라의 월드 공간 행렬을 가져옵니다.
                    let mut query = world
                        .query_one::<&(Camera, WorldTransform)>(self.camera)
                        .expect("invalid entity!");
                    let (_, transform) = query.get().expect("invalid entity component!");

                    // 카메라의 뷰 프러스텀의 모서리 위치를 계산합니다.
                    let (width, height): (f32, f32) = window_size.size().into();
                    let frustum_corners = compute_frustum_corners_no_inverse(
                        transform,
                        CAMERA_FOV_Y,
                        width / height,
                        0.01,
                        15.0,
                    );

                    // 전역 조명의 변환 행렬을 계산합니다.
                    const MARGIN: f32 = 5.0;
                    let color = directional_light.color;
                    let light_dir = directional_light.direction_w;
                    let light_proj_view =
                        compute_light_view_proj_matrix(&frustum_corners, light_dir, MARGIN);

                    // 유니폼 버퍼를 갱신합니다.
                    let data = GlobalLightDataLayout {
                        static_light_proj_view: directional_light.light_proj_view.to_cols_array(),
                        light_proj_view: light_proj_view.to_cols_array(),
                        direction_w: light_dir.to_array(),
                        color: color.to_array(),
                        intensity: 1.0,
                        ..Default::default()
                    };
                    light_resource.global_light_uniform.update(
                        &device_cloned,
                        &mut encoder,
                        &mut staging_buffers,
                        data,
                    );

                    // 전역 조명의 그림자 쉐이더 리소스를 갱신합니다.
                    let shadow_resource = light_resource.get_global();
                    let data = LightTransformDataLayout {
                        proj_view: light_proj_view.to_cols_array(),
                    };
                    shadow_resource.uniform.update(
                        &device_cloned,
                        &mut encoder,
                        &mut staging_buffers,
                        data,
                    );

                    // 조명이 비추는 영역과 교차하는 엔터티를 수집합니다.
                    let frustum = Frustum::from_mat4(light_proj_view);
                    let mut stage_entity_list = hierarchy.area.clone();
                    stage_entity_list.append(&mut cull_stage_entities(&frustum, hierarchy));
                    let mut transform_resources = ShadowMap::default();
                    collect_stage_resource(
                        world,
                        &stage_entity_list,
                        child_view,
                        sibling_view,
                        mesh_filter_view,
                        skinned_mesh_filter_view,
                        &mut transform_resources,
                    );

                    let (entity, archetype) = self.player_entity();
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

                    bake_tasks.push((shadow_resource.clone(), transform_resources));
                }
            });
        });
    }

    /// Ui의 크기를 재설정합니다.
    fn resize_ui(&mut self, window: &Window, app: &dyn AppHandle) {
        // 클립 사각형 영역의 크기를 재조정합니다.
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        (self.clip_rect, self.ui_scale) = Self::resize_clip_rect(viewport, scale_factor);

        // 레이아웃 사각형 영역의 크기를 재조정합니다.
        self.layout_rect = Self::resize_layout_bg(&self.clip_rect);
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

    /// 레이아웃 사각형 영역의 크기를 재조정합니다.
    fn resize_layout_bg(clip_rect: &egui::Rect) -> egui::Rect {
        let width = clip_rect.width() * 0.7;
        let height = width * 0.135;
        let size = egui::vec2(width, height);
        let min = clip_rect.min + egui::vec2(0.0, clip_rect.height() * 0.7);
        egui::Rect::from_min_size(min, size)
    }

    /// 레이아웃 사각형을 그립니다.
    fn draw_layout_bg(&self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Layout_Bg"))
            .sense(egui::Sense::empty())
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);

                let t = self.animation_time_ms as f32 / MAX_ANIME_TIME as f32;
                let begin = -self.layout_rect.width();
                let position = egui::vec2(begin * (1.0 - t), 0.0);

                let left_top = self.layout_rect.min + position;
                let right_bottom =
                    self.layout_rect.max - egui::Vec2::splat(22.5 * self.ui_scale) + position;
                let rect = egui::Rect::from_min_max(left_top, right_bottom);
                let uv = egui::Rect::from_pos(egui::pos2(0.9214659686, 0.625));
                egui::Image::new(self.layout_texture)
                    .tint(egui::Color32::from_white_alpha(192))
                    .sense(egui::Sense::empty())
                    .uv(uv)
                    .paint_at(ui, rect);

                let left_top = self.layout_rect.left_bottom()
                    - egui::vec2(0.0, 22.5 * self.ui_scale)
                    + position;
                let right_bottom = self.layout_rect.right_bottom()
                    - egui::vec2(22.5 * self.ui_scale, 0.0)
                    + position;
                let rect = egui::Rect::from_min_max(left_top, right_bottom);
                let uv =
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.625), egui::pos2(0.9214659686, 1.0));
                egui::Image::new(self.layout_texture)
                    .tint(egui::Color32::from_white_alpha(192))
                    .sense(egui::Sense::empty())
                    .uv(uv)
                    .paint_at(ui, rect);

                let left_top =
                    self.layout_rect.right_top() - egui::vec2(22.5 * self.ui_scale, 0.0) + position;
                let right_bottom = self.layout_rect.right_bottom()
                    - egui::vec2(0.0, 22.5 * self.ui_scale)
                    + position;
                let rect = egui::Rect::from_min_max(left_top, right_bottom);
                let uv =
                    egui::Rect::from_min_max(egui::pos2(0.9214659686, 0.0), egui::pos2(1.0, 0.625));
                egui::Image::new(self.layout_texture)
                    .tint(egui::Color32::from_white_alpha(192))
                    .sense(egui::Sense::empty())
                    .uv(uv)
                    .paint_at(ui, rect);

                let left_top = self.layout_rect.right_bottom()
                    - egui::Vec2::splat(22.5 * self.ui_scale)
                    + position;
                let right_bottom = self.layout_rect.right_bottom() + position;
                let rect = egui::Rect::from_min_max(left_top, right_bottom);
                let uv =
                    egui::Rect::from_min_max(egui::pos2(0.9214659686, 0.625), egui::pos2(1.0, 1.0));
                egui::Image::new(self.layout_texture)
                    .tint(egui::Color32::from_white_alpha(192))
                    .sense(egui::Sense::empty())
                    .uv(uv)
                    .paint_at(ui, rect);
            });
    }

    /// 이미지 폰트를 그립니다.
    fn draw_img_font(&self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("Mission_Font"))
            .sense(egui::Sense::empty())
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let t = self.animation_time_ms as f32 / MAX_ANIME_TIME as f32;
                let begin = -self.layout_rect.width();
                let position = egui::vec2(begin * (1.0 - t), 0.0);

                let min = self.layout_rect.min;
                let max = min + self.layout_rect.size() * egui::vec2(0.333, 1.0);
                let rect = egui::Rect::from_min_max(min, max);

                let texture_size = self.img_font_texture.size;
                let ratio = texture_size.x / texture_size.y;
                let center = rect.center() + position;
                let width = rect.width() * 0.7;
                let height = width / ratio;
                let size = egui::vec2(width, height);
                let rect = egui::Rect::from_center_size(center, size);
                egui::Image::new(self.img_font_texture)
                    .sense(egui::Sense::empty())
                    .paint_at(ui, rect);
            });
    }

    /// 안내 메시지를 그립니다.
    fn draw_message(&self, ctx: &egui::Context, i: usize) {
        egui::Area::new(egui::Id::new("Message_Font"))
            .sense(egui::Sense::empty())
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.shrink_clip_rect(self.clip_rect);
                let t = self.animation_time_ms as f32 / MAX_ANIME_TIME as f32;
                let begin = -self.layout_rect.width();
                let position = egui::vec2(begin * (1.0 - t), 0.0);

                let max = self.layout_rect.max + position;
                let min = max - self.layout_rect.size() * egui::vec2(0.666, 1.0);
                let rect = egui::Rect::from_min_max(min, max);

                let text = MESSAGE_TEXTS[i];
                let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
                let font_id = egui::FontId::new(24.0 * self.ui_scale, family);
                let text = egui::RichText::new(text).font(font_id).color(FONT_COLOR);
                let label = egui::Label::new(text)
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .sense(egui::Sense::empty())
                    .selectable(false);
                ui.put(rect, label);
            });
    }
}

impl GameScene for InGameEnterScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        if window.has_focus() {
            self.first_mouse_pressed = true;
            let event = AppEvent::CursorDisable;
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        let size = app.window_size();
        let device = app.render_device();
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.setup_player();

        self.create_camera(size, device);
        self.update_camera_and_skybox(device, &mut encoder, &mut staging_buffers);

        self.cull_stage_entities();
        {
            let world = self.world.as_ref().expect("the world must be exists!");
            let child_view = world.view::<&Child>();
            let sibling_view = world.view::<&Sibling>();
            update_stage_hierarchy(
                world,
                &self.culling_stage_entities,
                &child_view,
                &sibling_view,
            );
        }

        self.regist_hud_layout_texture(device, ui_renderer);
        self.regist_img_font_texture(device, ui_renderer);
        self.resize_ui(window, app);

        let queue = app.render_queue();
        queue.submit(Some(encoder.finish()));
        drop(staging_buffers);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        _app: &dyn AppHandle,
        ui_renderer: &mut UiRenderer,
    ) {
        // Ui 렌더러에 등록된 텍스처를 해제합니다.
        ui_renderer.free_texture(&self.layout_texture.id);
        ui_renderer.free_texture(&self.img_font_texture.id);
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
            let device = app.render_device();
            self.create_weighted_blend_oit_resource(size, device);
            self.create_bloom_resource(size, device);
            self.resize_ui(window, app);
        }
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
        _: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::InGamePull => {
                let packet = InGamePullPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let mut world = match self.world.take() {
                    Some(world) => world,
                    None => return None,
                };

                // 생성한 카메라를 제거합니다.
                cleanup(&mut world, self.camera);

                let players = self.players.clone();
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
                let skybox = self.skybox.take().expect("the skybox must be exists!");
                let direction_light = self
                    .direction_light
                    .take()
                    .expect("the direction light must be exists!");
                let light_resource = self
                    .light_resource
                    .take()
                    .expect("the light shader resource must be exists!");
                let scene = InGameRunScene::new(
                    packet.epoch,
                    self.locale,
                    self.uid,
                    self.token,
                    self.stage_kind,
                    packet.remaining_time_ms,
                    self.first_mouse_pressed,
                    world,
                    players,
                    stage,
                    accum_render_target,
                    reveal_render_target,
                    bright_render_target,
                    alpha_blend_pipeline,
                    gaussian_blur_pipeline,
                    bloom_pipeline,
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
                );
                let flow = GameSceneFlow::Change(Box::new(scene));
                let event = AppEvent::AddGameSceneFlow(flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
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
        let elapsed_time_ms = (elapsed_time_sec * 1000.0) as u16;
        self.animation_time_ms = (self.animation_time_ms + elapsed_time_ms).min(MAX_ANIME_TIME);
        self.remaining_time_ms = self.remaining_time_ms.saturating_sub(elapsed_time_ms);
        self.update_player_character(elapsed_time_ms);
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        let world = match self.world.as_ref() {
            Some(world) => world,
            None => return,
        };

        let device = app.render_device();
        let queue = app.render_queue();
        let child_view = &world.view::<&Child>();
        let sibling_view = &world.view::<&Sibling>();
        let mesh_filter_view = &world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &world.view::<SkinnedMeshRenderer>();
        let stage_entities = &self.culling_stage_entities;

        // 캐릭터 애니메이션을 재생합니다.
        let (entity, archetype) = self.player_entity();
        let animation_view = world.view::<AnimationQuery>();
        let collection_view = world.view::<&BoneCollection>();
        animate_character(
            world,
            entity,
            archetype,
            &self.motion_pool,
            &animation_view,
            &collection_view,
        );

        // 캐릭터 계층 구조를 갱신합니다.
        update_character_hierarchy(world, entity, archetype, &child_view, &sibling_view);

        let draw_tasks = &Arc::new(Queue::new());
        rayon::in_place_scope(move |scope| {
            // 캐릭터 쉐이더 리소스를 갱신합니다.
            scope.spawn(move |_| {
                let mut staging_buffers = Vec::default();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

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

            // 스테이지 쉐이더 리소스를 갱신합니다.
            scope.spawn(move |_| {
                let mut staging_buffers = Vec::default();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                update_stage_resource(
                    world,
                    stage_entities,
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
        });

        let device = device.clone();
        let window_size = app.window_size();
        let bake_tasks = Arc::new(Queue::new());
        self.update_light_resource(
            device,
            child_view,
            sibling_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
            &bake_tasks,
            window_size,
        );

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

    fn ui_callback(&mut self, _window: &Window, app: &dyn AppHandle) {
        let ctx = app.egui_ctx();
        self.draw_layout_bg(ctx);
        self.draw_img_font(ctx);
        self.draw_message(ctx, self.locale as usize);
    }
}
