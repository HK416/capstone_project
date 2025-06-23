use std::sync::Arc;

use ahash::HashMap;
use hecs::{Entity, ViewBorrow, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, CharacterKind, ExSkillCost, GamePlayData, HealthPoint,
        LatLon, LoginToken, MovementState, MovementStateTimer, PlayPhasePlayer, RemainingBullet,
        UserId, ViewState, ViewStateTimer, MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, PrepareStagePacket, PullStagePacket, RawPacket},
};
use mod_physics::object3d::Frustum;
use mod_render::{UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT};
use winit::window::Window;

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, StageBoundingVolumn,
        StageBoundingVolumnHierarchy, TextureDataPool, TexturePool, TextureViewPool,
        CHARACTER_ICON_URIS, FIELD_DECO_00_URI, FIELD_DECO_01_URI, IMG_FONT_LOSE_URI,
        IMG_FONT_MISSION_URI, IMG_FONT_START_URI, IMG_FONT_WIN_URI, NOTOSANS_REGULAR,
        SCHALE_ICON_URI, TIMER_ICON_URI, WEAPON_ICON_MASK_URI, WEAPON_ICON_URI,
    },
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage,
        clear_render_target_with_skybox, collect_bake_resources,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_character,
        draw_character_eye_mouth, draw_character_halo, draw_stage, update_character_resource,
        update_entity_hierarchy, update_stage_resource, AccumRenderTarget, AlphaBlendPipeline,
        BakeList, BloomPipeline, BoneCollection, BrightRenderTarget, CameraDataLayout,
        CameraResource, CameraUniform, CharacterBakePipeline, CharacterRenderPipeline, Child,
        EyeMouthBakePipeline, EyeMouthRenderPipeline, GaussianBlurPipeline, GlobalLight,
        GlobalLightDataLayout, HaloRenderPipeline, LightSetResource, LightTransformDataLayout,
        MaterialKind, MeshRenderer, OpaqueMap, Projection, RevealRenderTarget, ShadowMap, Sibling,
        SkinnedMeshRenderer, SkinningAnimation, Skybox, SkyboxDataLayout, SkyboxRenderPipeline,
        StageBakePipeline, StageRenderPipeline, ToParentTrans, TransparentMap, TreeRenderPipeline,
        WorldTransform, GLOBAL_SHADOW_MAP_SIZE, LOCAL_SHADOW_MAP_SIZE, SHADOW_FORMAT,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

use super::InGameDominationModeScene;

/// 애플리케이션 표시 언어에 따른 게임 방법 안내 텍스트
const INFOMATION_TEXTS: [&'static str; NUM_LOCALE] =
    ["맵 중앙의 목표 구역을 먼저 선점하는 팀이 승리"];

/// 게임 진행 전 대기하는 게임 장면입니다.
pub struct InGameDominationModePrepareScene {
    /// 애플리케이션 표시언어입니다.
    locale: Locale,
    /// 현재 사용자의 식별자입니다.
    user_id: UserId,
    /// 현재 사용자의 로그인 토큰입니다.
    token: LoginToken,

    /// 게임 장면의 남은 시간입니다.
    remainint_time_sec: f32,
    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

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
    /// 지형 엔터티의 Bounding Volumn Hierarchy 입니다.
    stages: StageBoundingVolumnHierarchy,
    /// 카메라 뷰 프러스텀 컬링된 엔터티 집합입니다.
    culling_stages: Vec<Entity>,

    /// 전역 조명 데이터입니다.
    global_light: Option<GlobalLight>,
    /// 조명 집합 쉐이더 리소스입니다.
    light_set_resource: Option<LightSetResource>,

    /// 반투명 오브젝트의 누적 값(Accumuldate)을 저장하는 렌더 타겟입니다.
    accum_render_target: Option<AccumRenderTarget>,
    /// 반투명 오브젝트의 노출 값(Revealage)을 저장하는 렌더 타겟입니다.
    reveal_render_target: Option<RevealRenderTarget>,
    /// 발광체 오브젝트의 색상을 저장하는 렌더 타겟입니다.
    bright_render_target: Option<BrightRenderTarget>,

    /// 알파 블렌딩을 수행하는 파이프라인입니다.
    alpha_blend_pipeline: Option<AlphaBlendPipeline>,
    /// 가우시안 블러를 수행하는 파이프라인입니다.
    gaussian_blur_pipeline: Option<GaussianBlurPipeline>,
    /// Bloom 효과를 구현하는 파이프라인입니다.
    bloom_pipeline: Option<BloomPipeline>,

    /// 게임 인터페이스 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,

    /// 조명 렌더링 리소스 집합입니다.
    bake_list: BakeList,
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
// 초기화 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModePrepareScene {
    /// 새로운 `InGameDominationModePrepareScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        world: World,
        skybox: Skybox,
        players: HashMap<UserId, Entity>,
        stages: StageBoundingVolumnHierarchy,
        global_light: Option<GlobalLight>,
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
            remainint_time_sec: f32::INFINITY,
            elapsed_time_sec: 0.0,
            world: Some(world),
            skybox: Some(skybox),
            main_camera: Entity::DANGLING,
            players,
            disconnected_players: Vec::with_capacity(MAX_IN_GAME_PLAYERS),
            stages,
            culling_stages: Vec::default(),
            global_light,
            light_set_resource: None,
            accum_render_target: None,
            reveal_render_target: None,
            bright_render_target: None,
            alpha_blend_pipeline: None,
            gaussian_blur_pipeline: None,
            bloom_pipeline: None,
            ui_textures: HashMap::default(),
            bake_list: Vec::default(),
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

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, device: &wgpu::Device) {
        // 플레이어 캐릭터 위치를 가져옵니다.
        let entity = self.get_player_entity();
        // Safe: 게임 월드는 `on_enter`가 호출되는 시점에선 제거되지 않습니다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };
        let trans = world
            .query_one_mut::<&ToParentTrans>(entity)
            .expect("invalid entity or invalid entity component");

        // 카메라의 위치와 방향을 설정합니다.
        let pivot = trans.get_translation() + glam::Vec3A::Y * 0.6;
        let position = pivot
            + trans.get_right_vector() * 1.0
            + trans.get_look_vector() * 1.0
            + glam::Vec3A::Y * 0.05;
        let look = (pivot - position).normalize();
        let right = glam::Vec3A::Y.cross(look);
        let up = look.cross(right);
        let cam_trans = glam::Mat4::from_mat3_translation(
            glam::mat3(right.into(), up.into(), look.into()),
            position.into(),
        );

        // 카메라 쉐이더 리소스를 생성합니다.
        let camera_uniform = CameraUniform::uninit(Some("Main"), device);
        let camera_resource = CameraResource::new(Some("Main"), device, &camera_uniform);

        // 엔터티를 생성합니다.
        self.main_camera = world.spawn((
            ToParentTrans(cam_trans),
            WorldTransform::default(),
            Projection::perspective(60f32.to_radians(), 16.0 / 9.0, 0.01, 50.0),
            camera_uniform,
            camera_resource,
            Frustum::from_mat4(glam::Mat4::IDENTITY),
        ));
    }

    /// UI에 사용되는 텍스처를 UI렌더러에 등록합니다.
    fn register_ui_texture(&mut self, device: &wgpu::Device, egui_renderer: &mut UiRenderer) {
        self.register_field_deco_00_texture(device, egui_renderer);
        self.register_field_deco_01_texture(device, egui_renderer);
        self.register_timer_icon_texture(device, egui_renderer);
        self.register_schale_icon_texture(device, egui_renderer);
        self.register_character_icon_textures(device, egui_renderer);
        self.register_img_font_lose_texture(device, egui_renderer);
        self.register_img_font_mission_texture(device, egui_renderer);
        self.register_img_font_start_texture(device, egui_renderer);
        self.register_img_font_win_texture(device, egui_renderer);
        self.register_player_weapon_icon_texture(device, egui_renderer);
        self.register_player_weapon_icon_mask_texture(device, egui_renderer);
    }

    /// `Field_Deco_00` 텍스처를 UI 렌더러에 등록합니다.
    fn register_field_deco_00_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Field_Deco_00` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(FIELD_DECO_00_URI)
            .expect("Field_Deco_00 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            FIELD_DECO_00_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// `Field_Deco_01` 텍스처를 UI 렌더러에 등록합니다.
    fn register_field_deco_01_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Field_Deco_01` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(FIELD_DECO_01_URI)
            .expect("Field_Deco_01 texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            FIELD_DECO_01_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 타이머 아이콘 텍스처를 UI 렌더러에 등록합니다.
    fn register_timer_icon_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Timer_Icon` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(TIMER_ICON_URI)
            .expect("Timer_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture: Arc<wgpu::TextureView> = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            TIMER_ICON_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// Schale 아이콘 텍스처를 UI 렌더러에 등록합니다.
    fn register_schale_icon_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Schale_Icon` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(SCHALE_ICON_URI)
            .expect("Schale_Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            SCHALE_ICON_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 캐릭터 아이콘 텍스처를 UI 렌더러에 등록합니다.
    fn register_character_icon_textures(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        for uri in CHARACTER_ICON_URIS {
            // 캐릭터 이미지 텍스처를 가져옵니다.
            let result = self.texture_pool.get(uri);

            let texture = match result {
                Some(texture) => texture,
                None => continue,
            };
            let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

            // 텍스처 뷰를 생성합니다.
            let texture = self
                .texture_view_pool
                .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

            // egui 렌더러에 텍스처를 등록합니다.
            let texture_id =
                egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

            self.ui_textures.insert(
                uri.into(),
                egui::load::SizedTexture {
                    id: texture_id,
                    size: texture_size,
                },
            );
        }
    }

    /// `Img_Font_Lose`텍스처를 UI렌더러에 등록합니다.
    fn register_img_font_lose_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Img_Font_Lose` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(IMG_FONT_LOSE_URI)
            .expect("the Img_Font_Lose texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            IMG_FONT_LOSE_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// `Img_Font_Mission`텍스처를 UI렌더러에 등록합니다.
    fn register_img_font_mission_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Img_Font_Mission` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(IMG_FONT_MISSION_URI)
            .expect("the Img_Font_Mission texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            IMG_FONT_MISSION_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// `Img_Font_Start`텍스처를 UI렌더러에 등록합니다.
    fn register_img_font_start_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Img_Font_Start` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(IMG_FONT_START_URI)
            .expect("the Img_Font_Start texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            IMG_FONT_START_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// `Img_Font_Win`텍스처를 UI렌더러에 등록합니다.
    fn register_img_font_win_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // `Img_Font_Start` 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(IMG_FONT_WIN_URI)
            .expect("the Img_Font_Win texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            IMG_FONT_WIN_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 플레이어 무기 아이콘 텍스처를 UI 렌더러에 등록합니다.
    fn register_player_weapon_icon_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // 플레이어 캐릭터 무기 아이콘 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(WEAPON_ICON_URI)
            .expect("the Weapon Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            WEAPON_ICON_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 플레이어 무기 아이콘 마스킹 텍스처를 UI 렌더러에 등록합니다.
    fn register_player_weapon_icon_mask_texture(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut UiRenderer,
    ) {
        // 플레이어 캐릭터 무기 아이콘 마스킹 텍스처를 가져옵니다.
        let texture = self
            .texture_pool
            .get(WEAPON_ICON_MASK_URI)
            .expect("the Weapon Icon texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 텍스처 뷰를 생성합니다.
        let texture = self
            .texture_view_pool
            .get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let texture_id =
            egui_renderer.register_native_texture(device, &texture, wgpu::FilterMode::Linear);

        self.ui_textures.insert(
            WEAPON_ICON_MASK_URI.into(),
            egui::load::SizedTexture {
                id: texture_id,
                size: texture_size,
            },
        );
    }

    /// 조명 집합에 사용되는 쉐이더 리소스를 생성합니다.
    fn create_light_set_resource(&mut self, device: &wgpu::Device) {
        self.light_set_resource = self.global_light.as_ref().map(|light| {
            LightSetResource::new(
                Some("City"),
                device,
                &light.static_shadow_map_view,
                &light.static_shadow_map_sampler,
                GLOBAL_SHADOW_MAP_SIZE,
                LOCAL_SHADOW_MAP_SIZE,
            )
        });
    }

    /// 지연 쉐이더 기법을 사용하는 파이프라인과 쉐이더 리소스를 생성합니다.
    fn create_deferred(&mut self, window: &Window, device: &wgpu::Device) {
        // 현재 애플리케이션 창의 크기를 가져옵니다.
        let (width, height) = window.inner_size().into();

        // Bloom을 위한 텍스처와 파이프라인을 생성합니다.
        let (gaussian_blur_pipeline, bright_render_target, bloom_pipeline) = match self
            .gaussian_blur_pipeline
            .take()
            .zip(self.bloom_pipeline.take())
        {
            Some((gaussian_blur_pipeline, bloom_pipeline)) => {
                gaussian_blur_pipeline.renew(width, height, device, bloom_pipeline)
            }
            None => GaussianBlurPipeline::new(width, height, device, SWAPCHAIN_FORMAT),
        };

        // Weighted-Blended OIT를 위한 렌더 타겟을 생성합니다.
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

        self.accum_render_target = Some(accum_render_target);
        self.reveal_render_target = Some(reveal_render_target);
        self.bright_render_target = Some(bright_render_target);

        self.alpha_blend_pipeline = Some(alpha_blend_pipeline);
        self.gaussian_blur_pipeline = Some(gaussian_blur_pipeline);
        self.bloom_pipeline = Some(bloom_pipeline);
    }
}

//--------------------------------------------------------------------------------------------
// 플레이어 조작과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModePrepareScene {
    /// 플레이어 엔터티를 반환합니다.
    fn get_player_entity(&self) -> Entity {
        self.players
            .get(&self.user_id)
            .cloned()
            .expect("the player entity must exist!")
    }
}

//--------------------------------------------------------------------------------------------
// 네트워크 통신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModePrepareScene {
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
}

//--------------------------------------------------------------------------------------------
// 엔터티 계층 구조 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModePrepareScene {
    /// 카메라를 갱신합니다.
    fn update_camera(&mut self) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };
        update_entity_hierarchy(world, self.main_camera, glam::Mat4::IDENTITY);
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

    // /// 캐릭터의 무기를 갱신합니다.
    // fn update_character_weapon(&mut self) {
    //     // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
    //     let world = unsafe { self.world.as_mut().unwrap_unchecked() };

    //     type Query<'a> = (&'a CharacterKind, &'a ActionState, &'a SkinningAnimation);
    //     let element_view = world.view::<Query>();
    //     let child_view = world.view::<&Child>();
    //     let sibling_view = world.view::<&Sibling>();
    //     let mut transform_view = world.view::<(&ToParentTrans, &mut WorldTransform)>();

    //     for entity in self.players.values().cloned() {
    //         let (&character_kind, &action_state, skinning_animation) = element_view
    //             .get(entity)
    //             .expect("invalid entity or invalid entity component");

    //         set_weapon_position(
    //             character_kind,
    //             action_state,
    //             skinning_animation,
    //             &child_view,
    //             &sibling_view,
    //             &mut transform_view,
    //         );
    //     }
    // }

    /// 캐릭터 엔터티의 계층 구조를 갱신합니다.
    fn update_character(&mut self) {
        self.animate_character();

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 캐릭터의 계층 구조를 갱신합니다.
        for entity in self.players.values().cloned() {
            update_entity_hierarchy(world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 지형 엔터티의 계층 구조를 갱신합니다.
    fn update_stage(&mut self) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        for entity in self.culling_stages.iter().cloned() {
            update_entity_hierarchy(world, entity, glam::Mat4::IDENTITY);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 쉐이더 리소스 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameDominationModePrepareScene {
    fn update_camera_and_skybox_resource(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        type Query<'a> = (
            &'a CameraUniform,
            &'a WorldTransform,
            &'a Projection,
            &'a mut Frustum,
        );

        // 카메라 엔터티의 요소를 가져옵니다.
        let (uniform, trans, proj, frustum) = world
            .query_one_mut::<Query>(self.main_camera)
            .expect("invalid entity or invalid entity component");

        // 카메라 데이터 유니폼 버퍼를 갱신합니다.
        let position_w = trans.get_translation();
        let view = trans.to_view_trans();
        let proj_view = proj.0 * view;
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
        // Safe: 스카이박스가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let skybox = unsafe { self.skybox.as_ref().unwrap_unchecked() };
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
        vec![self.get_player_entity()]
    }

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 지형 엔터티를 수집합니다.
    ///
    /// # Note
    /// 이 함수는 카메라의 월드 변환 행렬을 갱신한 후 호출되어야 합니다.
    ///
    fn culling_stages(&self) -> Vec<Entity> {
        // 카메라의 위치와 뷰 프러스텀을 가져옵니다.
        let world = self.world.as_ref().unwrap();
        let mut query = world
            .query_one::<&Frustum>(self.main_camera)
            .expect("invalid entity");
        let frustum = query.get().expect("invalid entity component");

        // 프러스텀 컬링된 엔터티를 수집합니다.
        let mut entities = self.stages.area.clone();
        if let Some(node) = self.stages.root.as_ref() {
            Self::culling_stage_recursive(frustum, node, &mut entities);
        }

        entities
    }

    /// 카메라 프러스텀과 교차되는 엔터티를 수집합니다.
    fn culling_stage_recursive(
        frustum: &Frustum,
        node: &StageBoundingVolumn,
        entities: &mut Vec<Entity>,
    ) {
        if frustum.sphere_test(&node.sphere) {
            entities.push(node.entity);
        }
        if let Some(left_node) = node.left.as_ref() {
            Self::culling_stage_recursive(frustum, left_node, entities);
        }
        if let Some(right_node) = node.right.as_ref() {
            Self::culling_stage_recursive(frustum, right_node, entities);
        }
    }

    /// 전역 조명 쉐이더 리소스를 갱신합니다.
    fn update_global_light_resource(
        &self,
        window: &Window,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        bake_list: &mut BakeList,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        mesh_filter_view: &mut ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &mut ViewBorrow<'_, SkinnedMeshRenderer>,
    ) {
        // 전역 조명이 없는 경우 해당 함수를 생략합니다.
        if self.global_light.is_none() {
            return;
        }

        // 애플리케이션 창의 크기를 가져옵니다.
        let (width, height): (f32, f32) = window.inner_size().into();

        // 카메라의 월드 공간 행렬을 가져옵니다.
        let world = self.world.as_ref().unwrap();
        let mut query = world
            .query_one::<&WorldTransform>(self.main_camera)
            .expect("invalid entity");
        let transform = query.get().expect("invalid entity component");

        // 카메라의 뷰 프러스텀의 모서리 위치를 계산합니다.
        let frustum_corners = compute_frustum_corners_no_inverse(
            transform,
            60f32.to_radians(),
            width / height,
            0.01,
            15.0,
        );

        // 전역 조명의 변환 행렬을 계산합니다.
        let g_light = self.global_light.as_ref().unwrap();
        let light_proj_view =
            compute_light_view_proj_matrix(&frustum_corners, g_light.direction_w, 5.0);

        // 전역 조명 데이터 유니폼 버퍼를 갱신합니다.
        let light_set_resource = self.light_set_resource.as_ref().unwrap();
        light_set_resource.global_light_uniform.update(
            device,
            encoder,
            staging_buffers,
            GlobalLightDataLayout {
                proj_view: light_proj_view.to_cols_array(),
                direction_w: g_light.direction_w.to_array(),
                color: g_light.color.to_array(),
                ..Default::default()
            },
        );

        // 전역 조명의 그림자 쉐이더 리소스를 가져오고 내용을 갱신합니다.
        let shadow_resource = light_set_resource.get_global();
        shadow_resource.uniform.update(
            device,
            encoder,
            staging_buffers,
            LightTransformDataLayout {
                proj_view: light_proj_view.to_cols_array(),
            },
        );

        // 조명이 비추는 영역과 교차하는 엔터티를 수집합니다.
        let frustum = Frustum::from_mat4(light_proj_view);
        let mut entities: Vec<Entity> = self.players.values().cloned().collect();
        entities.extend_from_slice(&self.stages.area);

        if let Some(node) = self.stages.root.as_ref() {
            Self::culling_stage_recursive(&frustum, node, &mut entities);
        }

        // 수집된 엔터티의 MeshFilter를 수집합니다.
        let mut shadow_map = ShadowMap::default();
        for entity in entities {
            collect_bake_resources(
                entity,
                &mut shadow_map,
                child_view,
                sibling_view,
                mesh_filter_view,
                skinned_mesh_filter_view,
            );
        }

        // BakeList에 추가합니다.
        bake_list.push((shadow_resource, shadow_map));
    }
}

//--------------------------------------------------------------------------------------------

impl GameScene for InGameDominationModePrepareScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle, ui_renderer: &mut UiRenderer) {
        let event = AppEvent::CursorDisable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        let device = app.render_device();
        self.register_ui_texture(&device, ui_renderer);
        self.create_main_camera(&device);
        self.create_light_set_resource(device);
        self.create_deferred(window, &device);
    }

    fn on_enter_foreground(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorDisable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_enter_background(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i]
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
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

                // 다음 게임 장면으로 전환합니다.
                let world = self.world.take().unwrap();
                let skybox = self.skybox.take().unwrap();
                let players = self.players.to_owned();
                let disconnected_players = self.disconnected_players.to_owned();
                let stages = self.stages.to_owned();
                let global_light = self.global_light.take();
                let light_set_resource = self.light_set_resource.take().unwrap();
                let accum_render_target = self.accum_render_target.take().unwrap();
                let reveal_render_target = self.reveal_render_target.take().unwrap();
                let bright_render_target = self.bright_render_target.take().unwrap();
                let alpha_blend_pipeline = self.alpha_blend_pipeline.take().unwrap();
                let gaussian_blur_pipeline = self.gaussian_blur_pipeline.take().unwrap();
                let bloom_pipeline = self.bloom_pipeline.take().unwrap();
                let ui_textures = self.ui_textures.to_owned();
                let mut next_scene = InGameDominationModeScene::new(
                    self.locale,
                    self.user_id,
                    self.token,
                    world,
                    skybox,
                    players,
                    disconnected_players,
                    stages,
                    global_light,
                    light_set_resource,
                    accum_render_target,
                    reveal_render_target,
                    bright_render_target,
                    alpha_blend_pipeline,
                    gaussian_blur_pipeline,
                    bloom_pipeline,
                    ui_textures,
                    self.mesh_pool.clone(),
                    self.model_pool.clone(),
                    self.motion_pool.clone(),
                    self.texture_pool.clone(),
                    self.texture_data_pool.clone(),
                    self.texture_view_pool.clone(),
                    self.sampler_pool.clone(),
                );
                next_scene.setup_progress(packet.capture_point, packet.remaining_time_sec);
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::FinishStage => {
                // 이벤트를 보류합니다.
                let event_loop_proxy = app.event_loop_proxy();
                let event = AppEvent::PacketReceived(packet);
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::PrepareStage => {
                let packet = PrepareStagePacket::from_raw(packet);
                self.update_player_from_pull_packet(&packet.players);
                self.remainint_time_sec = packet.remaining_time_sec;
            }
            _ => {}
        }

        None
    }

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        self.create_deferred(window, app.render_device());
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        // 남은 시간과 경과 시간을 갱신합니다.
        self.remainint_time_sec = (self.remainint_time_sec - elapsed_time_sec).max(0.0);
        self.elapsed_time_sec += elapsed_time_sec;
    }

    fn on_prepare_draw(&mut self, window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        self.update_camera();
        self.update_character();

        // 프러스텀 컬링을 수행합니다.
        self.culling_stages = self.culling_stages();
        self.update_stage();

        let device = app.render_device();
        let queue = app.render_queue();
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 카메라 쉐이더 리소스를 갱신합니다.
        self.update_camera_and_skybox_resource(device, &mut encoder, &mut staging_buffers);

        let mut shadow_map = HashMap::default();
        let mut opaque_map = HashMap::default();
        let mut transparent_map = HashMap::default();
        let mut bake_list = Vec::default();

        let world = self.world.as_ref().unwrap();
        let child_view = &world.view::<&Child>();
        let sibling_view = &world.view::<&Sibling>();
        let transform_view = &world.view::<&WorldTransform>();
        let mesh_filter_view = &mut world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &mut world.view::<SkinnedMeshRenderer>();

        // 캐릭터 쉐이더 리소스를 갱신합니다.
        let entities = self.culling_character();
        for entity in entities {
            update_character_resource(
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

        // 지형의 쉐이더 리소스를 갱신합니다.
        for entity in self.culling_stages.iter().cloned() {
            update_stage_resource(
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
        }

        // 전역 조명 쉐이더 리소스를 갱신합니다.
        self.update_global_light_resource(
            window,
            device,
            &mut encoder,
            &mut staging_buffers,
            &mut bake_list,
            child_view,
            sibling_view,
            mesh_filter_view,
            skinned_mesh_filter_view,
        );

        queue.submit(Some(encoder.finish()));
        drop(staging_buffers);

        self.opaque_map = opaque_map;
        self.transparent_map = transparent_map;
        self.bake_list = bake_list;
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        app: &dyn AppHandle,
    ) {
        if self.world.is_none() {
            return;
        }

        let device = app.render_device();

        // 카메라 쉐이더 리소스를 가져옵니다.
        let world = self.world.as_mut().unwrap();
        let camera_resource = world
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

        // 쉐이더 리소스를 가져옵니다.
        let skybox = self.skybox.as_ref().unwrap();
        let light_set_resource = self.light_set_resource.as_ref().unwrap();
        let accum_render_target = self.accum_render_target.as_ref().unwrap();
        let reveal_render_target = self.reveal_render_target.as_ref().unwrap();
        let bright_render_target = self.bright_render_target.as_ref().unwrap();
        let alpha_blend_pipeline = self.alpha_blend_pipeline.as_ref().unwrap();
        let gaussian_blur_pipeline = self.gaussian_blur_pipeline.as_ref().unwrap();
        let bloom_pipeline = self.bloom_pipeline.as_ref().unwrap();

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

            for ((mesh, kind), submesh_resources) in shadow_map.iter() {
                match kind {
                    MaterialKind::Character => bake_character(
                        &mesh,
                        CharacterBakePipeline::get_or_init(device, SHADOW_FORMAT),
                        shadow_resource,
                        submesh_resources,
                        &mut rpass,
                    ),
                    MaterialKind::CharacterEyeMouth => bake_character_eye_mouth(
                        &mesh,
                        EyeMouthBakePipeline::get_or_init(device, SHADOW_FORMAT),
                        shadow_resource,
                        submesh_resources,
                        &mut rpass,
                    ),
                    MaterialKind::Stage | MaterialKind::Tree => bake_stage(
                        &mesh,
                        StageBakePipeline::get_or_init(device, SHADOW_FORMAT),
                        shadow_resource,
                        submesh_resources,
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

            for ((mesh, kind), material_resources) in self.opaque_map.iter() {
                match kind {
                    MaterialKind::Character => {
                        draw_character(
                            &mesh,
                            CharacterRenderPipeline::get_or_init(
                                &device,
                                SWAPCHAIN_FORMAT,
                                DEPTH_FORMAT,
                            ),
                            &camera_resource,
                            light_set_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::CharacterEyeMouth => {
                        draw_character_eye_mouth(
                            &mesh,
                            EyeMouthRenderPipeline::get_or_init(
                                &device,
                                SWAPCHAIN_FORMAT,
                                DEPTH_FORMAT,
                            ),
                            &camera_resource,
                            light_set_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::CharacterHalo => {
                        draw_character_halo(
                            &mesh,
                            HaloRenderPipeline::get_or_init(
                                &device,
                                SWAPCHAIN_FORMAT,
                                DEPTH_FORMAT,
                            ),
                            &camera_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::Stage => {
                        draw_stage(
                            &mesh,
                            StageRenderPipeline::get_or_init(
                                &device,
                                SWAPCHAIN_FORMAT,
                                DEPTH_FORMAT,
                            ),
                            &camera_resource,
                            light_set_resource,
                            &material_resources,
                            &mut rpass,
                        );
                    }
                    MaterialKind::Tree => {
                        draw_stage(
                            &mesh,
                            TreeRenderPipeline::get_or_init(
                                &device,
                                SWAPCHAIN_FORMAT,
                                DEPTH_FORMAT,
                            ),
                            &camera_resource,
                            light_set_resource,
                            material_resources,
                            &mut rpass,
                        );
                    }
                    _ => {}
                };
            }

            clear_render_target_with_skybox(
                &skybox,
                SkyboxRenderPipeline::get_or_init(&device, SWAPCHAIN_FORMAT, DEPTH_FORMAT),
                &mut rpass,
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("transparent pass");
        {
            let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(InGame(TransparentPass))"),
                color_attachments: &[
                    // 0번 렌더 타겟: 누적 값 렌더 타겟
                    Some(wgpu::RenderPassColorAttachment {
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear({
                                wgpu::Color {
                                    a: 1.0,
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
                    // 1번 렌더 타겟: 노출 값 렌더 타겟
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
                    // 2번 렌더 타겟: 발광체 색깔 렌더 타겟
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
        self.opaque_map.clear();
        self.transparent_map.clear();
        self.bake_list.clear();
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        const BEG_X: f32 = -940.0;
        const END_X: f32 = 0.0;
        const DURATION: f32 = 1.25;
        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 임무 설명 인터페이스 속성
        // - 기준 가로 크기: 940
        // - 기준 세로 크기: 120
        // - 기준 시작 위치: (0, 504)
        // - 기준 종료 위치: (940, 624)
        let field_deco_01 = self
            .ui_textures
            .get(FIELD_DECO_01_URI)
            .cloned()
            .expect("the Field_Deco_01 must exsit!");

        let tint = egui::Color32::from_white_alpha(192);
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(x * scale, 504.0 * scale),
            egui::pos2((x + 895.0) * scale, 624.0 * scale),
        );
        let content_uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(0.9214659686, 1.0));
        let deco_rect_0 = egui::Rect::from_min_max(
            egui::pos2((x + 895.0) * scale, 504.0 * scale),
            egui::pos2((x + 940.0) * scale, 579.0 * scale),
        );
        let deco_uv_0 =
            egui::Rect::from_min_max(egui::pos2(0.9214659686, 0.0), egui::Pos2::new(1.0, 0.625));
        let deco_rect_1 = egui::Rect::from_min_max(
            egui::pos2((x + 895.0) * scale, 579.0 * scale),
            egui::pos2((x + 940.0) * scale, 624.0 * scale),
        );
        let deco_uv_1 =
            egui::Rect::from_min_max(egui::pos2(0.9214659686, 0.625), egui::Pos2::new(1.0, 1.0));

        // 임무 폰트 인터페이스 속성
        // - 기준 가로 크기: 256
        // - 기준 세로 크기: 64
        // - 기준 시작 위치: (24, 530)
        // - 기준 종료 위치: (280, 594)
        let img_font_mission = self
            .ui_textures
            .get(IMG_FONT_MISSION_URI)
            .cloned()
            .expect("the ImgFont_Mission must exist!");
        let font_rect = egui::Rect::from_min_max(
            egui::pos2((x + 24.0) * scale, 530.0 * scale),
            egui::pos2((x + 280.0) * scale, 594.0 * scale),
        );

        // 임무 설명 인터페이스
        let family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let font_id = egui::FontId::new(24.0 * scale, family);
        let text = egui::RichText::new(INFOMATION_TEXTS[i])
            .font(font_id)
            .color(egui::Color32::BLACK);
        let text_rect = egui::Rect::from_min_max(
            egui::pos2((x + 296.0) * scale, 530.0 * scale),
            egui::pos2((x + 895.0) * scale, 594.0 * scale),
        );

        egui::Area::new(egui::Id::new("Notify_Layout")).show(app.egui_ctx(), |ui| {
            egui::Image::new(field_deco_01)
                .uv(content_uv)
                .tint(tint)
                .paint_at(ui, content_rect);
            egui::Image::new(field_deco_01)
                .uv(deco_uv_0)
                .tint(tint)
                .paint_at(ui, deco_rect_0);
            egui::Image::new(field_deco_01)
                .uv(deco_uv_1)
                .tint(tint)
                .paint_at(ui, deco_rect_1);

            egui::Image::new(img_font_mission).paint_at(ui, font_rect);

            ui.put(
                text_rect,
                egui::Label::new(text).sense(egui::Sense::empty()),
            );
        });
    }
}
