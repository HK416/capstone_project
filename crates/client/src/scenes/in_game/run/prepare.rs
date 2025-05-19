use std::sync::Arc;

use ahash::HashMap;
use hecs::{Entity, World};
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
        StageLightData, UserId, ViewState, ViewStateTimer, MAX_IN_GAME_PLAYERS,
    },
    protocol::{Packet, PacketType, PrepareStagePacket, PullStagePacket, RawPacket},
};
use mod_physics::object3d::Frustum;
use mod_render::UiRenderer;
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
        clear_render_target_with_skybox, compute_cascade_splits,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_character,
        draw_character_eye_mouth, draw_character_halo, draw_stage, update_character_resource,
        update_entity_hierarchy, update_stage_resource, BakeList, BoneCollection, CameraDataLayout,
        CameraResource, CameraUniform, CharacterBakePipeline, CharacterRenderPipeline, Child,
        EyeMouthBakePipeline, EyeMouthRenderPipeline, HaloRenderPipeline, LightSetDataLayout,
        LightSetResource, LightTransformDataLayout, MaterialKind, MeshRenderer, OpaqueMap,
        Projection, ShadowMap, Sibling, SkinnedMeshRenderer, SkinningAnimation, Skybox,
        SkyboxDataLayout, SkyboxRenderPipeline, StageBakePipeline, StageRenderPipeline,
        ToParentTrans, TransparentMap, WeightedBlendedOITRenderPipeline,
        WeightedBlendedOITResource, WorldTransform, NUM_CASCADES,
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
    /// 지형의 조명 데이터 집합입니다.
    lights: Vec<StageLightData>,

    /// 조명 집합 쉐이더 리소스입니다.
    light_set_resource: Option<LightSetResource>,
    /// 알파 블렌딩 쉐이더 리소스입니다.
    alpha_blend_resource: Option<WeightedBlendedOITResource>,

    /// 게임 인터페이스 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,

    /// 조명 렌더링 리소스 집합입니다.
    bake_list: BakeList,
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
        lights: Vec<StageLightData>,
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
            lights,
            light_set_resource: None,
            alpha_blend_resource: None,
            ui_textures: HashMap::default(),
            bake_list: Vec::default(),
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
        self.light_set_resource = Some(LightSetResource::new(Some("Main"), device));
    }

    /// 알파 블렌드에 사용되는 쉐이더 리소스를 생성합니다.
    fn create_alpha_blend_resource(&mut self, window: &Window, device: &wgpu::Device) {
        let (width, height): (u32, u32) = window.inner_size().into();
        self.alpha_blend_resource = Some(WeightedBlendedOITResource::new(width, height, device));
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

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 조명 엔터티를 수집합니다.
    fn culling_lights(&self) -> Vec<&StageLightData> {
        // FIXME: 현재는 모든 엔터티를 전부 렌더링함
        self.lights.iter().collect()
    }

    /// 조명 쉐이더 리소스를 갱신합니다.
    fn update_light_resource<'a>(
        &self,
        lights: Vec<&'a StageLightData>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        bake_list: &mut BakeList,
    ) {
        let mut data_layout = Box::new(LightSetDataLayout::default());
        let light_set_resource = self.light_set_resource.as_ref().unwrap();
        for data in lights {
            match data {
                StageLightData::Directional(light) => {
                    // 카메라의 월드 공간 행렬을 가져옵니다.
                    let world = self.world.as_ref().unwrap();
                    let mut query = world
                        .query_one::<&WorldTransform>(self.main_camera)
                        .expect("invalid entity");
                    let transform = query.get().expect("invalid entity component");

                    data_layout.direction_w = light.direction.into();
                    data_layout.color = light.color.into();

                    let splits = compute_cascade_splits(NUM_CASCADES, 0.01, 50.0, 0.85);
                    for i in 0..NUM_CASCADES {
                        // 프러스텀의 모서리 위치를 계산합니다.
                        let near = if i == 0 { 0.01 } else { splits[i - 1] };
                        let far = splits[i];
                        let fov_y = 60f32.to_radians();
                        let corner = compute_frustum_corners_no_inverse(
                            transform,
                            fov_y,
                            16.0 / 9.0,
                            near,
                            far,
                        );

                        // 조명 변환 행렬을 계산합니다.
                        let proj_view =
                            compute_light_view_proj_matrix(&corner, light.direction.into(), 5.0);

                        // 전역 조명 유니폼 버퍼 데이터를 갱신합니다.
                        data_layout.global_lights[i] = LightTransformDataLayout {
                            proj_view: proj_view.to_cols_array(),
                        };

                        // 전역 조명 그림자 쉐이더 리소스를 가져옵니다.
                        let resource = light_set_resource.get_global(i);
                        resource.uniform.update(
                            device,
                            encoder,
                            staging_buffers,
                            LightTransformDataLayout {
                                proj_view: proj_view.to_cols_array(),
                            },
                        );
                        bake_list.push(resource);
                    }
                }
            }
        }

        // 유니폼 버퍼를 갱신합니다.
        light_set_resource
            .uniform
            .update(device, encoder, staging_buffers, data_layout);
    }
}

//--------------------------------------------------------------------------------------------

impl GameScene for InGameDominationModePrepareScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {
        app.disable_cursor();

        let device = app.render_device();
        let mut egui_renderer = app.egui_renderer_mut();
        self.register_ui_texture(&device, &mut egui_renderer);
        self.create_main_camera(&device);
        self.create_light_set_resource(device);
        self.create_alpha_blend_resource(window, &device);
    }

    fn on_enter_foreground(&mut self, app: &dyn AppHandle) {
        app.disable_cursor();
    }

    fn on_enter_background(&mut self, app: &dyn AppHandle) {
        app.enable_cursor();
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
                let lights = self.lights.to_owned();
                let light_set_resource = self.light_set_resource.take().unwrap();
                let alpha_blend_resource = self.alpha_blend_resource.take().unwrap();
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
                    lights,
                    light_set_resource,
                    alpha_blend_resource,
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
        self.create_alpha_blend_resource(window, app.render_device());
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, _app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        // 남은 시간과 경과 시간을 갱신합니다.
        self.remainint_time_sec = (self.remainint_time_sec - elapsed_time_sec).max(0.0);
        self.elapsed_time_sec += elapsed_time_sec;
    }

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
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

        // 조명 쉐이더 리소스를 갱신합니다.
        let lights = self.culling_lights();
        self.update_light_resource(
            lights,
            device,
            &mut encoder,
            &mut staging_buffers,
            &mut bake_list,
        );

        queue.submit(Some(encoder.finish()));
        drop(staging_buffers);

        self.shadow_map = shadow_map;
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
        _app: &dyn AppHandle,
    ) {
        if self.world.is_none() {
            return;
        }

        // Safe: 게임 월드가 없는 경우 게임 장면이 갱신되거나 렌더링 되지 않는다.
        let world = unsafe { self.world.as_mut().unwrap_unchecked() };

        // 카메라 쉐이더 리소스를 가져옵니다.
        let camera_resource = world
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

        // 쉐이더 리소스를 가져옵니다.
        let light_set_resource = self.light_set_resource.as_ref().unwrap();
        let alpha_blend_resource = self.alpha_blend_resource.as_ref().unwrap();
        let skybox = self.skybox.as_ref().unwrap();

        encoder.push_debug_group("shadow pass");
        for shadow_resource in self.bake_list.iter() {
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

            for ((mesh, kind), resources) in self.shadow_map.iter() {
                let func = match kind {
                    MaterialKind::Character => bake_character,
                    MaterialKind::CharacterEyeMouth => bake_character_eye_mouth,
                    MaterialKind::Stage => bake_stage,
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::Character => CharacterBakePipeline::get(),
                    MaterialKind::CharacterEyeMouth => EyeMouthBakePipeline::get(),
                    MaterialKind::Stage => StageBakePipeline::get(),
                    _ => continue,
                }
                .unwrap();

                func(&mesh, pipeline, &shadow_resource, &resources, &mut rpass);
            }
        }
        encoder.pop_debug_group();

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
                    MaterialKind::Character => draw_character,
                    MaterialKind::CharacterEyeMouth => draw_character_eye_mouth,
                    MaterialKind::CharacterHalo => draw_character_halo,
                    MaterialKind::Stage => {
                        draw_stage(
                            &mesh,
                            StageRenderPipeline::get().unwrap(),
                            &camera_resource,
                            light_set_resource,
                            &resources,
                            &mut rpass,
                        );
                        continue;
                    }
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::Character => CharacterRenderPipeline::get(),
                    MaterialKind::CharacterEyeMouth => EyeMouthRenderPipeline::get(),
                    MaterialKind::CharacterHalo => HaloRenderPipeline::get(),
                    _ => continue,
                }
                .unwrap();

                func(&mesh, pipeline, &camera_resource, &resources, &mut rpass);
            }

            clear_render_target_with_skybox(
                &skybox,
                SkyboxRenderPipeline::get().unwrap(),
                &mut rpass,
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("transparent pass");
        {
            let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
