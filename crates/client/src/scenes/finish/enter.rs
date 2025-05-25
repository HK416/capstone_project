//! 게임 결과에 진입하는 장면에 관련된 코드를 관리합니다.
//!

use std::{collections::VecDeque, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, ViewBorrow, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{
    ActionState, ActionStateTimer, CapturePoint, CharacterKind, ExSkillCost, FinishPhasePlayer,
    GameInputBits, HealthPoint, LatLon, LoginToken, MovementState, MovementStateTimer, ObjectId,
    RemainingBullet, StageKind, Team, UserId, VictoryType, ViewState, ViewStateTimer,
    MAX_CAPTURE_SCORE,
};
use mod_physics::object3d::Frustum;
use winit::window::Window;

use crate::{
    asset::{
        MotionPool, StageBoundingVolumn, StageBoundingVolumnHierarchy, FIELD_DECO_00_URI,
        IMG_FONT_LOSE_URI, IMG_FONT_WIN_URI, NOTOSANS_BOLD, SCHALE_ICON_URI, TIMER_ICON_URI,
        WEAPON_ICON_URI,
    },
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage,
        clear_render_target_with_skybox, collect_bake_resources,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_bullet,
        draw_character, draw_character_eye_mouth, draw_character_halo, draw_energy_bullet,
        draw_stage, set_weapon_position, try_change_action_state, try_reset_movement_state,
        update_action_state_timer, update_bullet_resource, update_character_resource,
        update_entity_hierarchy, update_movement_state_timer, update_stage_resource,
        update_third_person_camera, update_third_person_camera_hierarchy,
        update_view_state_by_controller_input_flags, update_view_state_timer, AttributeKind,
        BakeList, BoneCollection, BulletRenderPipelineTransparency, CameraDataLayout,
        CameraResource, CameraUniform, CharacterBakePipeline, CharacterRenderPipeline, Child,
        DamageFontDataLayout, DamageFontRenderPipeline, DamageFontResource, DamageFontUniform,
        DamageParticle, EnergyBulletRenderPipeline, EyeMouthBakePipeline, EyeMouthRenderPipeline,
        GlobalLight, GlobalLightDataLayout, HaloRenderPipeline, LightSetResource,
        LightTransformDataLayout, MaterialKind, Mesh, MeshRenderer, OpaqueMap, Parent, Projection,
        ShadowMap, Sibling, SkinnedMeshRenderer, SkinningAnimation, Skybox, SkyboxDataLayout,
        SkyboxRenderPipeline, StageBakePipeline, StageRenderPipeline, ThirdPersonCamera,
        ToParentTrans, TransparentMap, TreeRenderPipeline, WeightedBlendedOITRenderPipeline,
        WeightedBlendedOITResource, WorldTransform,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, InGameResultScene, BASE_WIDTH, TEAM_COLOR, UI_BG_COLOR},
};

/// 게임 장면의 지속 시간입니다.
const MAX_SCENE_DURATION: f32 = 6.0;
/// 게임이 천천히 멈추는 시간
const SMOOTH_STOP_DURATION: f32 = 3.0;

/// 게임 결과 장면에 진입하는 장면입니다.
pub struct InGameResultEnterScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 사용자 식별자입니다.
    user_id: UserId,
    /// 로그인 토큰입니다.
    token: LoginToken,
    /// 시야 조작 민감도입니다.
    control_sensitivity: f32,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 승리 팀
    winner: Team,
    /// 승리 종류
    victory_type: VictoryType,
    /// 게임 진행 시간
    play_time: f32,
    /// 스테이지 종류
    stage_kind: StageKind,
    /// 게임 진행 데이터입니다.
    play_data: Vec<FinishPhasePlayer>,
    /// 현재 게임 진행 상황입니다.
    capture_point: CapturePoint,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    ///엔터티를 관리하는 월드 객체입니다.
    world: Option<World>,
    /// 스카이박스입니다.
    skybox: Option<Skybox>,
    /// 게임 결과 장면의 메인 카메라 엔터티입니다.
    main_camera: Entity,
    /// 플레이어 엔터티 집합입니다.
    players: HashMap<UserId, Entity>,
    /// 연결이 끊어진 플레이어 엔터티 집합입니다.
    disconnected_players: Vec<Entity>,
    /// 오브젝트 엔터티 집합입니다.
    bullets: HashMap<ObjectId, Entity>,
    /// 지형 엔터티 집합입니다.
    stages: StageBoundingVolumnHierarchy,
    /// 프러스텀 컬링을 수행한 지형 엔터티 집합입니다.
    culling_stages: Vec<Entity>,

    /// 데미지 파티클 엔터티입니다.
    damage_particles: VecDeque<Entity>,

    /// 전역 조명 데이터입니다.
    global_light: Option<GlobalLight>,
    /// 조명 집합 쉐이더 리소스입니다.
    light_set_resource: Option<LightSetResource>,
    /// 알파 블렌딩 쉐이더 리소스입니다.
    alpha_blend_resource: Option<WeightedBlendedOITResource>,

    /// 게임 인터페이스 레이아웃 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,

    /// 조명 렌더링 리소스 집합입니다.
    bake_list: BakeList,
    /// 불투명 메쉬 렌더링 리소스 집합입니다.
    opaque_map: OpaqueMap,
    /// 투명 메쉬 렌더링 리소스 집합입니다.
    transparent_map: TransparentMap,

    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
}

//--------------------------------------------------------------------------------------------
// 초기화 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
    /// 새로운 `InGameResultEnterScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        control_sensitivity: f32,
        flip_horizontal: bool,
        flip_vertical: bool,
        winner: Team,
        victory_type: VictoryType,
        play_time: f32,
        stage_kind: StageKind,
        play_data: Vec<FinishPhasePlayer>,
        capture_point: CapturePoint,
        world: World,
        skybox: Skybox,
        main_camera: Entity,
        players: HashMap<UserId, Entity>,
        disconnected_players: Vec<Entity>,
        bullets: HashMap<ObjectId, Entity>,
        damage_particles: VecDeque<Entity>,
        stages: StageBoundingVolumnHierarchy,
        global_light: Option<GlobalLight>,
        light_set_resource: LightSetResource,
        alpha_blend_resource: WeightedBlendedOITResource,
        ui_textures: HashMap<String, egui::load::SizedTexture>,
        motion_pool: MotionPool,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            control_sensitivity,
            flip_horizontal,
            flip_vertical,
            winner,
            victory_type,
            play_time,
            stage_kind,
            play_data,
            capture_point,
            elapsed_time_sec: 0.0,
            world: Some(world),
            skybox: Some(skybox),
            main_camera,
            players,
            disconnected_players,
            bullets,
            stages,
            culling_stages: Vec::default(),
            damage_particles,
            ui_textures,
            global_light,
            light_set_resource: Some(light_set_resource),
            alpha_blend_resource: Some(alpha_blend_resource),
            bake_list: Vec::default(),
            opaque_map: HashMap::default(),
            transparent_map: HashMap::default(),
            motion_pool,
        }
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
impl InGameResultEnterScene {
    /// 플레이어 엔터티를 반환합니다.
    fn get_player_entity(&self) -> Entity {
        self.players
            .get(&self.user_id)
            .cloned()
            .expect("the player entity must exist!")
    }

    /// 캐릭터의 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_and_timer(&mut self, elapsed_time_sec: f32) {
        type Query<'a> = (
            &'a CharacterKind,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
        );

        // 캐릭터 종류, 행동 상태, 행동 상태 타이머를 가져옵니다.
        let world = self.world.as_mut().unwrap();
        let mut view = world.view_mut::<Query>();
        for entity in self.players.values().cloned() {
            let (&character_kind, action_state, action_state_timer) = view
                .get_mut(entity)
                .expect("invalid entity or invalid entity component");

            try_change_action_state(
                character_kind,
                action_state,
                action_state_timer,
                ActionState::Idle,
            );
            update_action_state_timer(
                character_kind,
                action_state,
                action_state_timer,
                elapsed_time_sec,
            );
        }
    }

    /// 캐릭터의 `MovementStateTimer`를 갱신합니다.
    fn update_movement_state_and_timer(&mut self, elapsed_time_sec: f32) {
        type Query<'a> = (
            &'a CharacterKind,
            &'a ActionState,
            &'a mut MovementState,
            &'a mut MovementStateTimer,
        );

        // 캐릭터 종류, 행동 상태, 움직임 상태, 움직임 상태 타이머를 가져옵니다.
        let world = self.world.as_mut().unwrap();
        let mut view = world.view_mut::<Query>();
        for entity in self.players.values().cloned() {
            let (&character_kind, &action_state, movement_state, movement_state_timer) = view
                .get_mut(entity)
                .expect("invalid entity or invalid entity component");

            try_reset_movement_state(movement_state, movement_state_timer);
            update_movement_state_timer(
                character_kind,
                action_state,
                movement_state,
                movement_state_timer,
                elapsed_time_sec,
            );
        }
    }

    /// 플레이어 카메라 상태를 갱신합니다.
    fn update_view_state(&mut self) {
        type Query<'a> = (&'a CharacterKind, &'a mut ViewState, &'a mut ViewStateTimer);
        let entity = self.get_player_entity();

        // 캐릭터 종류, 카메라 상태, 카메라 상태 타이머 요소를 가져옵니다.
        let world = self.world.as_mut().unwrap();
        let (&character_kind, view_state, view_state_timer) = world
            .query_one_mut::<Query>(entity)
            .expect("invalid entity or invalid entity component");

        // 현재 입력 상태에 따라 카메라 상태를 갱신합니다.
        update_view_state_by_controller_input_flags(
            character_kind,
            view_state,
            view_state_timer,
            GameInputBits::empty(),
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
// 엔터티 계층 구조 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
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
impl InGameResultEnterScene {
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
        // FIXME: 현재는 모든 엔터티를 전부 렌더링함
        self.players.values().cloned().collect()
    }

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 총알 엔터티를 수집합니다.
    fn culling_bullets(&self) -> Vec<Entity> {
        // FIXME: 현재는 모든 엔터티를 전부 렌더링함
        self.bullets.values().cloned().collect()
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
            .query_one::<(&ThirdPersonCamera, &WorldTransform)>(self.main_camera)
            .expect("invalid entity");
        let (third_person_camera, transform) = query.get().expect("invalid entity component");

        // 카메라의 뷰 프러스텀의 모서리 위치를 계산합니다.
        let frustum_corners = compute_frustum_corners_no_inverse(
            transform,
            third_person_camera.fov_y,
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
        // entities.extend_from_slice(&self.stages.area);

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
}

//--------------------------------------------------------------------------------------------
// 렌더링과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
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
}

//--------------------------------------------------------------------------------------------
// 사용자 인터페이스와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
    /// 결과 인터페이스를 출력합니다.
    fn draw_ui_result_font(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;

        // 게임 장면 경과 시간이 시작 문구 지속 시간보다 큰 경우 함수 실행을 생략
        if self.elapsed_time_sec < SMOOTH_STOP_DURATION {
            return;
        }

        let delta = ((self.elapsed_time_sec - SMOOTH_STOP_DURATION) / DURATION).min(1.0);
        let t = delta * delta * (3.0 - 2.0 * delta);

        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let &(team, _) = world
            .query_one_mut::<&(Team, usize)>(entity)
            .expect("invalid entity or invalid entity component");

        // 게임 시작 폰트 속성
        // - 기준 가로 크기: 768
        // - 기준 세로 크기: 384
        let hw = (704.0 * (1.0 - t) + 768.0 * t) * 0.5;
        let hh = (352.0 * (1.0 - t) + 384.0 * t) * 0.5;
        let tint = egui::Color32::from_white_alpha((255.0 * t) as u8);
        let img_font_start = self
            .ui_textures
            .get(match self.winner == team {
                true => IMG_FONT_WIN_URI,
                false => IMG_FONT_LOSE_URI,
            })
            .cloned()
            .expect("the ImgFont must exist!");
        let font_rect = egui::Rect::from_min_max(
            egui::pos2((640.0 - hw) * scale, (360.0 - hh) * scale),
            egui::pos2((640.0 + hw) * scale, (360.0 + hh) * scale),
        );

        egui::Area::new(egui::Id::new("Start_Font_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(img_font_start)
                .tint(tint)
                .paint_at(ui, font_rect);
        });
    }

    /// 체력 인터페이스 배경을 그립니다.
    fn draw_ui_health_point_bg(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = -310.0;
        const END_X: f32 = 0.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 체력 인터페이스 레이아웃 이미지
        // - 기준 가로 크기: 280
        // - 기준 세로 크기: 94
        // - 기준 시작 위치: (30, 596)
        // - 기준 종료 위치: (310, 690)
        //
        let field_deco_00 = self
            .ui_textures
            .get(FIELD_DECO_00_URI)
            .cloned()
            .expect("the Field_Deco_00 must exist!");

        let front_rect = egui::Rect::from_min_max(
            egui::pos2((x + 30.0) * scale, 596.0 * scale),
            egui::pos2((x + 63.0) * scale, 690.0 * scale),
        );
        let front_uv = egui::Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(0.59375, 1.0));
        let middle_rect = egui::Rect::from_min_max(
            egui::pos2((x + 63.0) * scale, 596.0 * scale),
            egui::pos2((x + 277.0) * scale, 690.0 * scale),
        );
        let middle_uv =
            egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(0.40625, 1.0));
        let back_rect = egui::Rect::from_min_max(
            egui::pos2((x + 277.0) * scale, 596.0 * scale),
            egui::pos2((x + 310.0) * scale, 690.0 * scale),
        );
        let back_uv = egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.0, 1.0));

        // 체력 인터페이스 데코레이션
        // - 기준 가로 크기: 210
        // - 기준 세로 크기: 2
        // - 기준 시작 위치: (75, 678)
        // - 기준 종료 위치: (285, 680)
        let deco_pos = egui::Rect::from_min_max(
            egui::pos2((x + 75.0) * scale, 678.0 * scale),
            egui::pos2((x + 285.0) * scale, 680.0 * scale),
        );
        let deco_uv = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(0.40625, 1.0));

        egui::Area::new(egui::Id::new("Health_BG_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(field_deco_00)
                .uv(front_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, front_rect);
            egui::Image::new(field_deco_00)
                .uv(middle_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, middle_rect);
            egui::Image::new(field_deco_00)
                .uv(back_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, back_rect);

            egui::Image::new(field_deco_00)
                .uv(deco_uv)
                .paint_at(ui, deco_pos);
        });
    }

    /// 체력 게이지 인터페이스를 그립니다.
    fn draw_ui_health_point_gauge(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = -310.0;
        const END_X: f32 = 0.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 플레이어의 현재 체력을 가져옵니다.
        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let &health_point = world
            .query_one_mut::<&HealthPoint>(entity)
            .expect("invalid entity or invalid entity component");
        let percent = health_point.percent();

        // 체력 텍스트를 생성합니다.
        let text = format!("{}", health_point.current.min(9999));
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(22.0 * scale, family.clone());
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let health_text_rect = egui::Rect::from_min_max(
            egui::pos2((x + 55.0) * scale, 647.5 * scale),
            egui::pos2((x + 183.0) * scale, 672.0 * scale),
        );
        let health_point = egui::Label::new(text).sense(egui::Sense::empty());

        egui::Area::new(egui::Id::new("Health_Gauge_Layout")).show(egui_ctx, |ui| {
            // 기준 가로 크기: 35.5
            // 기준 세로 크기: 35.5
            // 기준 간격 가로 크기: 2.4
            // 기준 시작 위치: (55, 612)
            // 기준 종료 위치: (280, 647.5)
            // 기준 범위: 225
            let pivot_x = (x + 55.0) * scale;
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

        egui::Area::new(egui::Id::new("Health_Number_Layout")).show(egui_ctx, |ui| {
            ui.put(health_text_rect, health_point);
        });
    }

    /// 팀 점수 게이지 인터페이스를 그립니다.
    fn draw_ui_score_gauge(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_Y: f32 = -134.0;
        const END_Y: f32 = 0.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let y = BEG_Y * (1.0 - t) + END_Y * t;

        // schale 아이콘
        let schale_icon = self
            .ui_textures
            .get(SCHALE_ICON_URI)
            .cloned()
            .expect("the Schale_Icon must exist!");
        let icon_area = egui::Rect::from_min_max(
            egui::pos2(610.0 * scale, (y + 24.0) * scale),
            egui::pos2(670.0 * scale, (y + 84.0) * scale),
        );

        // 전체 배경
        // - 가로 기준 길이: 520
        // - 세로 기준 길이: 12
        //
        let rect = egui::Rect::from_min_max(
            egui::pos2(380.0 * scale, (y + 48.0) * scale),
            egui::pos2(900.0 * scale, (y + 60.0) * scale),
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
            egui::pos2(640.0 * scale, (y + 54.0) * scale),
            40.0 * scale,
            egui::Stroke::new(3.0 * scale, egui::Color32::WHITE),
        );
        let team_bg_shadow = egui::epaint::CircleShape::filled(
            egui::pos2(640.0 * scale, (y + 54.0) * scale),
            33.0 * scale,
            egui::Color32::from_black_alpha(192),
        );

        // 플레이어 팀 배경
        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let (team, _) = world
            .query_one_mut::<&(Team, usize)>(entity)
            .cloned()
            .expect("invalid entity or invalid entity component");
        let team_bg = egui::epaint::CircleShape::filled(
            egui::pos2(640.0 * scale, (y + 54.0) * scale),
            32.0 * scale,
            TEAM_COLOR[team as usize],
        );

        // 블루 팀 게이지 배경
        // - 가로 기준 길이: 200
        // - 세로 기준 길이: 8
        //
        let rect = egui::Rect::from_min_max(
            egui::pos2(382.0 * scale, (y + 50.0) * scale),
            egui::pos2(582.0 * scale, (y + 58.0) * scale),
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
            egui::pos2(698.0 * scale, (y + 50.0) * scale),
            egui::pos2(898.0 * scale, (y + 58.0) * scale),
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
            egui::pos2(582.0 * scale - width, (y + 50.0) * scale),
            egui::pos2(582.0 * scale, (y + 58.0) * scale),
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
            egui::pos2(698.0 * scale, (y + 50.0) * scale),
            egui::pos2(698.0 * scale + width, (y + 58.0) * scale),
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
                            egui::pos2(380.0 * scale, (y + 48.0) * scale),
                            egui::pos2(380.0 * scale + width, (y + 60.0) * scale),
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
                            egui::pos2(900.0 * scale - width, (y + 48.0) * scale),
                            egui::pos2(900.0 * scale, (y + 60.0) * scale),
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

    /// 남은 시간 인터페이스를 그립니다.
    fn draw_ui_remaining_timer(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = 1424.0;
        const END_X: f32 = 1280.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 남은 시간 인터페이스 레이아웃
        // - 기준 가로 크기: 144
        // - 기준 세로 크기: 36
        // - 기준 시작 위치: (1144, 12)
        // - 기준 종료 위치: (1264, 42)
        //
        let field_deco_00 = self
            .ui_textures
            .get(FIELD_DECO_00_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        let timer_icon = self
            .ui_textures
            .get(TIMER_ICON_URI)
            .cloned()
            .expect("the UI_Timer_Icon must exist!");

        let front_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1120.0)) * scale, 12.0 * scale),
            egui::pos2((x - (1280.0 - 1133.0)) * scale, 48.0 * scale),
        );
        let front_uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.40625, 1.0));
        let middle_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1133.0)) * scale, 12.0 * scale),
            egui::pos2((x - (1280.0 - 1253.0)) * scale, 48.0 * scale),
        );
        let middle_uv =
            egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.59375, 1.0));
        let back_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1253.0)) * scale, 12.0 * scale),
            egui::pos2((x - (1280.0 - 1264.0)) * scale, 48.0 * scale),
        );
        let back_uv = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(1.0, 1.0));

        // 타이머 아이콘 인터페이스
        let timer_icon_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1140.0)) * scale, 16.0 * scale),
            egui::pos2((x - (1280.0 - 1168.0)) * scale, 44.0 * scale),
        );

        // 남은 시간 폰트
        let font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(18.0 * scale, font_family);
        let remaining_time_text = egui::RichText::new("--:--")
            .font(font_id)
            .color(egui::Color32::WHITE);
        let text_area_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1164.0)) * scale, 14.0 * scale),
            egui::pos2((x - (1280.0 - 1252.0)) * scale, 46.0 * scale),
        );

        egui::Area::new(egui::Id::new("Timer_BG_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(field_deco_00)
                .uv(front_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, front_rect);
            egui::Image::new(field_deco_00)
                .uv(middle_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, middle_rect);
            egui::Image::new(field_deco_00)
                .uv(back_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, back_rect);

            egui::Image::new(timer_icon).paint_at(ui, timer_icon_rect);

            ui.put(text_area_rect, egui::Label::new(remaining_time_text));
        });
    }

    // 무기 정보 인터페이스를 그립니다.
    fn draw_ui_weapon_info(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = 1520.0;
        const END_X: f32 = 1280.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 총알 인터페이스 레이아웃
        // - 기준 가로 크기: 210
        // - 기준 세로 크기: 110
        // - 기준 시작 위치: (1040, 580)
        // - 기준 종료 위치: (1250, 690)
        //
        let field_deco_00 = self
            .ui_textures
            .get(FIELD_DECO_00_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        // 인터페이스 배경
        let front_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1030.0)) * scale, 580.0 * scale),
            egui::pos2((x - (1280.0 - 1063.0)) * scale, 690.0 * scale),
        );
        let front_uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.40625, 1.0));
        let middle_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1063.0)) * scale, 580.0 * scale),
            egui::pos2((x - (1280.0 - 1217.0)) * scale, 690.0 * scale),
        );
        let middle_uv =
            egui::Rect::from_min_max(egui::pos2(0.40625, 0.0), egui::pos2(0.59375, 1.0));
        let back_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1217.0)) * scale, 580.0 * scale),
            egui::pos2((x - (1280.0 - 1250.0)) * scale, 690.0 * scale),
        );
        let back_uv = egui::Rect::from_min_max(egui::pos2(0.59375, 0.0), egui::pos2(1.0, 1.0));

        egui::Area::new(egui::Id::new("Bullet_BG_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(field_deco_00)
                .uv(front_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, front_rect);
            egui::Image::new(field_deco_00)
                .uv(middle_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, middle_rect);
            egui::Image::new(field_deco_00)
                .uv(back_uv)
                .tint(UI_BG_COLOR)
                .paint_at(ui, back_rect);
        });
    }

    /// 님은 총알 갯수 인터페이스를 그립니다.
    fn draw_ui_bullet_count(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = 1520.0;
        const END_X: f32 = 1280.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 남은 총알을 개수를 가져옵니다.
        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let remaining_bullet = world
            .query_one_mut::<&RemainingBullet>(entity)
            .expect("invalid entity or invalid entity component");

        // 남은 총알 텍스트를 생성합니다.
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(22.0 * scale, family);
        let text = format!("{}/{}", remaining_bullet.current, remaining_bullet.maximum);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let max_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1040.0)) * scale, 650.0 * scale),
            egui::pos2((x - (1280.0 - 1140.0)) * scale, 670.0 * scale),
        );
        let widget = egui::Label::new(text).sense(egui::Sense::empty());

        egui::Area::new(egui::Id::new("Bullet_Text_Layout")).show(egui_ctx, |ui| {
            ui.put(max_rect, widget);
        });
    }

    /// 스킬 게이지 인터페이스를 그립니다.
    fn draw_ui_skill_guage(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = 1520.0;
        const END_X: f32 = 1280.0;
        const BG_COLOR: egui::Color32 = egui::Color32::from_black_alpha(192);
        const FILL_COLOR: egui::Color32 = egui::Color32::from_rgb(253, 218, 13);

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 현재 스킬 코스트를 가져옵니다.
        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let ex_skill_cost = world
            .query_one_mut::<&ExSkillCost>(entity)
            .expect("invalid entity or invalid entity component");
        let percent = ex_skill_cost.percent();

        egui::Area::new(egui::Id::new("Skill_Gauge_Layout")).show(egui_ctx, |ui| {
            // 기준 가로 크기: 18.3
            // 기준 세로 크기: 18.3
            // 기준 간격 가로 크기: 3
            // 기준 시작 위치: (1040, 555.7)
            // 기준 종료 위치: (1250, 574)
            let pivot_x = (x - (1280.0 - 1040.0)) * scale;
            let range_x = 210.0 * percent * scale;
            let maximum = 210.0 * scale;
            let corner_radius = 1.5 * scale;
            let interval = 3.0 * scale;
            let width = 18.3 * scale;
            let beg_y = 555.7 * scale;
            let end_y = 574.0 * scale;
            let mut beg_x = pivot_x;
            let mut end_x: f32;
            let mut rect: egui::Rect;
            let mut shape: egui::Shape;

            // 채워진 게이지 그리기
            while beg_x < pivot_x + range_x {
                end_x = beg_x + width;
                let x = if end_x > pivot_x + range_x {
                    // 게이지의 비어있는 영역 그리기
                    rect = egui::Rect::from_min_max(
                        egui::pos2(beg_x, beg_y),
                        egui::pos2(end_x, end_y),
                    );
                    shape = egui::Shape::rect_filled(rect, corner_radius, BG_COLOR);
                    ui.painter().add(shape);

                    pivot_x + range_x
                } else {
                    end_x
                };

                rect = egui::Rect::from_min_max(egui::pos2(beg_x, beg_y), egui::pos2(x, end_y));
                shape = egui::Shape::rect_filled(rect, corner_radius, FILL_COLOR);
                ui.painter().add(shape);

                beg_x = end_x + interval;
            }

            // 비어있는 게이지 그리기
            while beg_x < pivot_x + maximum {
                end_x = beg_x + width;
                rect = egui::Rect::from_min_max(egui::pos2(beg_x, beg_y), egui::pos2(end_x, end_y));
                shape = egui::Shape::rect_filled(rect, corner_radius, BG_COLOR);
                ui.painter().add(shape);

                beg_x = end_x + interval;
            }
        });
    }

    /// 무기 아이콘을 인터페이스를 그립니다.
    fn draw_ui_weapon_icon(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = 1520.0;
        const END_X: f32 = 1280.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        // 무기 아이콘을 가져옵니다.
        let weapon_icon = self
            .ui_textures
            .get(WEAPON_ICON_URI)
            .cloned()
            .expect("the Weapon_Icon must exist!");

        // 무기 아이콘
        // - 기준 가로 크기: 200
        // - 기준 시작 위치: (1040, 590)
        // - 기준 종료 위치: (1240, 200 / image_ratio)
        //
        let image_ratio = weapon_icon.size.x / weapon_icon.size.y;
        let width = 200.0;
        let height = width / image_ratio;
        let beg_x = (x - (1280.0 - 1040.0)) * scale;
        let beg_y = 590.0 * scale;
        let end_x = (x - (1280.0 - 1240.0)) * scale;
        let end_y = beg_y + height * scale;
        let icon_rect =
            egui::Rect::from_min_max(egui::pos2(beg_x, beg_y), egui::pos2(end_x, end_y));

        egui::Area::new(egui::Id::new("Weapon_Icon_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(weapon_icon).paint_at(ui, icon_rect);
        });
    }
}

//--------------------------------------------------------------------------------------------

impl GameScene for InGameResultEnterScene {
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

    fn on_cursor_moved(
        &mut self,
        _x: f32,
        _y: f32,
        mut dx: f32,
        mut dy: f32,
        _window: &Window,
        _app: &dyn AppHandle,
    ) -> bool {
        if self.world.is_none() {
            return false;
        }

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
        let delta = (self.elapsed_time_sec / SMOOTH_STOP_DURATION).min(1.0);
        third_person_camera.rotate(dx, dy, 1.0 * (1.0 - delta));
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
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_window_resized(&mut self, window: &Window, app: &dyn AppHandle) {
        self.create_alpha_blend_resource(window, app.render_device());
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        // 게임 장면 지속 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;
        if self.elapsed_time_sec >= MAX_SCENE_DURATION {
            // 다음 게임 장면으로 이동합니다.
            let mut world = self.world.take().unwrap();
            let skybox = self.skybox.take().unwrap();
            let players = self.players.to_owned();
            let disconnected_players = self.disconnected_players.to_owned();
            let stages = self.stages.to_owned();
            let play_data = self.play_data.to_owned();
            let global_light = self.global_light.take();
            let light_set_resource = self.light_set_resource.take().unwrap();
            let alpha_blend_resource = self.alpha_blend_resource.take().unwrap();
            let ui_textures = self.ui_textures.to_owned();
            let winner_players = InGameResultScene::get_winner_players(
                self.winner,
                &mut world,
                players,
                disconnected_players,
            );
            let next_scene = InGameResultScene::new(
                self.locale,
                self.user_id,
                self.token,
                self.winner,
                self.victory_type,
                self.play_time,
                self.stage_kind,
                world,
                skybox,
                winner_players,
                stages,
                play_data,
                global_light,
                light_set_resource,
                alpha_blend_resource,
                ui_textures,
                self.motion_pool.clone(),
            );
            let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
            return;
        }

        self.update_action_state_and_timer(elapsed_time_sec);
        self.update_movement_state_and_timer(elapsed_time_sec);

        self.update_view_state();
        self.update_view_state_timer(elapsed_time_sec);
        self.update_damage_particles(elapsed_time_sec);
    }

    fn on_prepare_draw(&mut self, window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        self.update_character();
        self.update_camera();

        self.culling_stages = self.culling_stages();
        self.update_bullet();

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

        // 총알 쉐이더 리소스를 갱신합니다.
        let entities = self.culling_bullets();
        for entity in entities {
            update_bullet_resource(
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

        // 지형 쉐이더 리소스를 갱신합니다.
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
        _app: &dyn AppHandle,
    ) {
        if self.world.is_none() {
            return;
        }

        // 카메라 쉐이더 리소스를 가져옵니다.
        let camera_resource = self
            .world
            .as_mut()
            .expect("the world must exist!")
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

        // 쉐이더 리소스를 가져옵니다.
        let light_set_resource = self.light_set_resource.as_ref().unwrap();
        let alpha_blend_resource = self.alpha_blend_resource.as_ref().unwrap();
        let skybox = self.skybox.as_ref().unwrap();

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

            for ((mesh, kind), resources) in shadow_map.iter() {
                let func = match kind {
                    MaterialKind::Character => bake_character,
                    MaterialKind::CharacterEyeMouth => bake_character_eye_mouth,
                    MaterialKind::Stage | MaterialKind::Tree => bake_stage,
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::Character => CharacterBakePipeline::get(),
                    MaterialKind::CharacterEyeMouth => EyeMouthBakePipeline::get(),
                    MaterialKind::Stage | MaterialKind::Tree => StageBakePipeline::get(),
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
                    MaterialKind::Tree => {
                        draw_stage(
                            &mesh,
                            TreeRenderPipeline::get().unwrap(),
                            &camera_resource,
                            light_set_resource,
                            resources,
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

            self.draw_damage_particle(
                DamageFontRenderPipeline::get().unwrap(),
                &camera_resource,
                &mut rpass,
            );

            clear_render_target_with_skybox(
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
                    MaterialKind::Bullet => draw_bullet,
                    MaterialKind::EnergyBullet => draw_energy_bullet,
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::Bullet => BulletRenderPipelineTransparency::get(),
                    MaterialKind::EnergyBullet => EnergyBulletRenderPipeline::get(),
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
        self.opaque_map.clear();
        self.transparent_map.clear();
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        if self.world.is_none() {
            return;
        }

        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let egui_ctx = app.egui_ctx();

        self.draw_ui_score_gauge(egui_ctx, scale);
        self.draw_ui_health_point_bg(egui_ctx, scale);
        self.draw_ui_health_point_gauge(egui_ctx, scale);
        self.draw_ui_remaining_timer(egui_ctx, scale);
        self.draw_ui_weapon_info(egui_ctx, scale);
        self.draw_ui_weapon_icon(egui_ctx, scale);
        self.draw_ui_bullet_count(egui_ctx, scale);
        self.draw_ui_skill_guage(egui_ctx, scale);
        self.draw_ui_result_font(egui_ctx, scale);
    }
}
