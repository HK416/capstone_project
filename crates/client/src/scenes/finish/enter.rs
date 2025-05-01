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
        MotionPool, FIELD_DECO_00_URI, IMG_FONT_LOSE_URI, IMG_FONT_WIN_URI, NOTOSANS_BOLD,
        NOTOSANS_REGULAR, SCHALE_ICON_URI, TIMER_ICON_URI, WEAPON_ICON_MASK_URI, WEAPON_ICON_URI,
    },
    component::{
        animate_character, set_weapon_position, try_change_action_state, try_reset_movement_state,
        update_action_state_timer, update_entity_hierarchy, update_movement_state_timer,
        update_third_person_camera, update_third_person_camera_hierarchy,
        update_view_state_by_controller_input_flags, update_view_state_timer, AttributeKind,
        BoneCollection, BulletRenderPipelineTransparency, CameraDataLayout, CameraResource,
        CameraUniform, CaptureZoneRenderPipeline, CharacterRenderPipeline, Child,
        DamageFontDataLayout, DamageFontRenderPipeline, DamageFontResource, DamageFontUniform,
        DamageParticle, EnergyBulletRenderPipeline, EyeMouthRenderPipeline, HaloRenderPipeline,
        MaterialKind, MaterialResource, MaterialUniform, Mesh, MeshFilter, MeshRenderer, OpaqueMap,
        Parent, Projection, ShadowMap, ShadowResource, Sibling, SkinnedMeshRenderer,
        SkinningAnimation, Skybox, SkyboxDataLayout, SkyboxRenderPipeline, StageRenderPipeline,
        ThirdPersonCamera, ToParentTrans, TransformDataLayout, TransparentMap,
        WeightedBlendedOITRenderPipeline, WeightedBlendedOITResource, WorldTransform,
        NUM_CUBE_VERTICES,
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
    stages: Vec<Entity>,

    /// 데미지 파티클 엔터티입니다.
    damage_particles: VecDeque<Entity>,

    /// 그림자 쉐이더 리소스입니다.
    shadow_resource: Option<ShadowResource>,
    /// 알파 블렌딩 쉐이더 리소스입니다.
    alpha_blend_resource: Option<WeightedBlendedOITResource>,

    /// 게임 인터페이스 레이아웃 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,

    /// 그림자 렌더링 리소스 집합입니다.
    shadow_map: ShadowMap,
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
        stages: Vec<Entity>,
        shadow_resource: ShadowResource,
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
            damage_particles,
            ui_textures,
            shadow_resource: Some(shadow_resource),
            alpha_blend_resource: Some(alpha_blend_resource),
            shadow_map: HashMap::default(),
            opaque_map: HashMap::default(),
            transparent_map: HashMap::default(),
            motion_pool,
        }
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
        mesh_filter_view: &mut ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &mut ViewBorrow<'_, SkinnedMeshRenderer>,
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

        let result = mesh_filter_view.get_mut(entity);
        if let Some((mesh, mesh_resource, uniform, _, materials)) = result {
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

        let result = skinned_mesh_filter_view.get_mut(entity);
        if let Some((mesh, mesh_resource, collection, uniform, _, materials)) = result {
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
        transparent_map: &mut TransparentMap,
        child_view: &ViewBorrow<'_, &Child>,
        sibling_view: &ViewBorrow<'_, &Sibling>,
        transform_view: &ViewBorrow<'_, &WorldTransform>,
        mesh_filter_view: &mut ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &mut ViewBorrow<'_, SkinnedMeshRenderer>,
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
            self.update_bullet_resource(
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

        let delta = (self.elapsed_time_sec / SMOOTH_STOP_DURATION).min(1.0);
        let result = mesh_filter_view.get_mut(entity);
        if let Some((mesh, mesh_resource, uniform, uniforms, materials)) = result {
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
            let iter = uniforms.iter().zip(materials.iter());
            for (index, (uniform, material)) in iter.enumerate() {
                let key = (mesh.clone(), material.kind());
                let value = (
                    index,
                    MeshFilter::Mesh(mesh_resource.clone()),
                    material.clone(),
                );

                match uniform {
                    MaterialUniform::Bullet { data, buffer } => {
                        let mut data_layout = data.clone();
                        data_layout.main_color[3] = data.main_color[3] * (1.0 - delta);
                        buffer.update(device, encoder, staging_buffers, data_layout);
                    }
                    MaterialUniform::EnergyBullet { data, buffer } => {
                        let mut data_layout = data.clone();
                        data_layout.main_color[3] = data.main_color[3] * (1.0 - delta);
                        buffer.update(device, encoder, staging_buffers, data_layout);
                    }
                    _ => {}
                };

                match material.kind() {
                    MaterialKind::Bullet => {
                        if let Some(resources) = transparent_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            transparent_map.insert(key, vec![value]);
                        }
                    }
                    MaterialKind::EnergyBullet => {
                        if let Some(resources) = transparent_map.get_mut(&key) {
                            resources.push(value);
                        } else {
                            transparent_map.insert(key, vec![value]);
                        }
                    }
                    _ => {}
                };
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

        let result = skinned_mesh_filter_view.get_mut(entity);
        if let Some((mesh, mesh_resource, collection, uniform, uniforms, materials)) = result {
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
            let delta = (self.elapsed_time_sec / SMOOTH_STOP_DURATION).min(1.0);
            let iter = uniforms.iter_mut().zip(materials.iter());
            for (index, (uniform, material)) in iter.enumerate() {
                let key = (mesh.clone(), material.kind());
                let value = (
                    index,
                    MeshFilter::SkinnedMesh(mesh_resource.clone()),
                    material.clone(),
                );
                if let Some(resources) = transparent_map.get_mut(&key) {
                    resources.push(value);
                } else {
                    transparent_map.insert(key, vec![value]);
                }

                match uniform {
                    MaterialUniform::Bullet { data, buffer } => {
                        let mut data_layout = data.clone();
                        data_layout.main_color[3] = data.main_color[3] * (1.0 - delta);
                        buffer.update(device, encoder, staging_buffers, data_layout);
                    }
                    MaterialUniform::EnergyBullet { data, buffer } => {
                        let mut data_layout = data.clone();
                        data_layout.main_color[3] = data.main_color[3] * (1.0 - delta);
                        buffer.update(device, encoder, staging_buffers, data_layout);
                    }
                    _ => {}
                };
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
        mesh_filter_view: &mut ViewBorrow<'_, MeshRenderer>,
        skinned_mesh_filter_view: &mut ViewBorrow<'_, SkinnedMeshRenderer>,
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

        let result = mesh_filter_view.get_mut(entity);
        if let Some((mesh, mesh_resource, uniform, uniforms, materials)) = result {
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

            let delta = (self.elapsed_time_sec / SMOOTH_STOP_DURATION).min(1.0);
            let iter = uniforms.iter_mut().zip(materials.iter());
            for (index, (uniform, material)) in iter.enumerate() {
                match uniform {
                    MaterialUniform::CaptureZone { data, buffer } => {
                        let mut data_layout = data.clone();
                        data_layout.color0[3] = data.color0[3] * (1.0 - delta);
                        data_layout.color1[3] = data.color1[3] * (1.0 - delta);
                        buffer.update(device, encoder, staging_buffers, data_layout);
                    }
                    _ => {}
                };

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

        let result = skinned_mesh_filter_view.get_mut(entity);
        if let Some((mesh, mesh_resource, collection, uniform, _, materials)) = result {
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
}

//--------------------------------------------------------------------------------------------
// 렌더링과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
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
// 시스템 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
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
// 사용자 인터페이스와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultEnterScene {
    /// 체력 인터페이스 배경 레이아웃이미지입니다.
    fn draw_health_point_bg_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
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

    /// 체력 게이지 인터페이스 레이아웃입니다.
    fn draw_health_point_gauge_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
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
        let font_id = egui::FontId::new(22.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let health_text_rect = egui::Rect::from_min_max(
            egui::pos2((x + 55.0) * scale, 647.5 * scale),
            egui::pos2((x + 183.0) * scale, 672.0 * scale),
        );
        let health_point = egui::Label::new(text).sense(egui::Sense::empty());

        egui::Area::new(egui::Id::new("Health_Gauge_Layout")).show(egui_ctx, |ui| {
            // 기준 가로 크기: 39.6
            // 기준 세로 크기: 52
            // 기준 간격 가로 크기: 3
            // 기준 시작 위치: (55, 612)
            // 기준 종료 위치: (280, 647.5)
            // 기준 범위: 225
            let pivot_x = (x + 55.0) * scale;
            let range_x = (x + 225.0) * percent * scale;
            let maximum = (x + 225.0) * scale;
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

    /// 팀 점수 게이지 인터페이스 레이아웃입니다.
    fn draw_score_gauge_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
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

    /// 남은 시간 인터페이스 레이아웃입니다.
    fn draw_remaining_timer_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
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
        let minute = (self.play_time / 60.0).floor();
        let seconds = (self.play_time % 60.0).floor();
        let text = format!("{:0>2}:{:0>2}", minute, seconds);
        let remaining_time_text = egui::RichText::new(text)
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

    // 남은 총알 인터페이스 레이아웃입니다.
    fn draw_remaining_bullet_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
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
        // - 기준 가로 크기: 220
        // - 기준 세로 크기: 110
        // - 기준 시작 위치: (1040, 580)
        // - 기준 종료 위치: (1250, 690)
        //
        let field_deco_00 = self
            .ui_textures
            .get(FIELD_DECO_00_URI)
            .cloned()
            .expect("the UI_Game_Layout must exist!");

        let weapon_mask_icon = self
            .ui_textures
            .get(WEAPON_ICON_MASK_URI)
            .cloned()
            .expect("the Weapon Icon must exist!");

        // 남은 총알 텍스트
        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let remaining_bullet = world
            .query_one_mut::<&RemainingBullet>(entity)
            .expect("invalid entity or invalid entity component");
        let text = format!("{}/{}", remaining_bullet.current, remaining_bullet.maximum);
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(18.0 * scale, family);
        let remaining_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let text_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1040.0)) * scale, 650.0 * scale),
            egui::pos2((x - (1280.0 - 1240.0)) * scale, 670.0 * scale),
        );

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

        // 무기 아이콘
        // 가로 길이: 200
        let ratio = weapon_mask_icon.size.x / weapon_mask_icon.size.y;
        let icon_width = 200.0;
        let icon_height = icon_width / ratio;
        let weapon_icon_rect = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1040.0)) * scale, 590.0 * scale),
            egui::pos2(
                (x - (1280.0 - 1240.0)) * scale,
                (590.0 + icon_height) * scale,
            ),
        );

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

            egui::Image::new(weapon_mask_icon)
                .tint(egui::Color32::DARK_GRAY)
                .paint_at(ui, weapon_icon_rect);

            ui.put(text_rect, egui::Label::new(remaining_text));
        });
    }

    /// Ex 스킬 게이지 인터페이스 레이아웃입니다.
    fn draw_ex_skill_guage_layout(&mut self, egui_ctx: &egui::Context, scale: f32) {
        const DURATION: f32 = 0.8;
        const BEG_X: f32 = 1520.0;
        const END_X: f32 = 1280.0;

        if self.elapsed_time_sec > DURATION {
            return;
        }

        let delta = (self.elapsed_time_sec / DURATION).min(1.0);
        let t = 1.0 - delta * delta * (3.0 - 2.0 * delta);
        let x = BEG_X * (1.0 - t) + END_X * t;

        let weapon_icon = self
            .ui_textures
            .get(WEAPON_ICON_URI)
            .cloned()
            .expect("the Weapon Icon must exist!");

        // 현재 Ex스킬 코스트를 가져옵니다.
        let entity = self.get_player_entity();
        let world = self.world.as_mut().unwrap();
        let ex_skill_cost = world
            .query_one_mut::<&ExSkillCost>(entity)
            .expect("invalid entity or invalid entity component");
        let percent = ex_skill_cost.percent();

        // 무기 아이콘
        // 가로 길이: 200
        let ratio = weapon_icon.size.x / weapon_icon.size.y;
        let icon_width = 200.0;
        let icon_height = icon_width / ratio;
        let icon_area = egui::Rect::from_min_max(
            egui::pos2((x - (1280.0 - 1040.0)) * scale, 590.0 * scale),
            egui::pos2(
                ((x - (1280.0 - 1040.0)) + icon_width * percent) * scale,
                (590.0 + icon_height) * scale,
            ),
        );
        let icon_uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(percent, 1.0));

        egui::Area::new(egui::Id::new("ExSkill_Layout")).show(egui_ctx, |ui| {
            egui::Image::new(weapon_icon)
                .uv(icon_uv)
                .paint_at(ui, icon_area);
        });
    }

    /// 결과 폰트를 출력합니다.
    fn draw_result_font(&mut self, egui_ctx: &egui::Context, scale: f32) {
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
}

//--------------------------------------------------------------------------------------------
impl GameScene for InGameResultEnterScene {
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
            let shadow_resource = self.shadow_resource.take().unwrap();
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
                shadow_resource,
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
        let mesh_filter_view = &mut world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &mut world.view::<SkinnedMeshRenderer>();

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
                &mut transparent_map,
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
        let camera_resource = self
            .world
            .as_mut()
            .expect("the world must exist!")
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

        // Weighted Blended OIT 쉐이더 리소스를 가져옵니다.
        let alpha_blend_resource = self
            .alpha_blend_resource
            .as_ref()
            .expect("the alpha blend shader resource must exist!");
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
                    MaterialKind::Character => Self::draw_character,
                    MaterialKind::CharacterEyeMouth => Self::draw_character_eye_mouth,
                    MaterialKind::CharacterHalo => Self::draw_character_halo,
                    MaterialKind::Stage => Self::draw_stage,
                    _ => continue,
                };
                let pipeline = match kind {
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
                    MaterialKind::Bullet => Self::draw_bullet,
                    MaterialKind::EnergyBullet => Self::draw_energy_bullet,
                    MaterialKind::CaptureZone => Self::draw_capture_zone,
                    _ => continue,
                };
                let pipeline = match kind {
                    MaterialKind::Bullet => BulletRenderPipelineTransparency::get(),
                    MaterialKind::EnergyBullet => EnergyBulletRenderPipeline::get(),
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
        if self.world.is_none() {
            return;
        }

        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        let egui_ctx = app.egui_ctx();
        self.draw_score_gauge_layout(egui_ctx, scale);
        self.draw_health_point_bg_layout(egui_ctx, scale);
        self.draw_health_point_gauge_layout(egui_ctx, scale);
        self.draw_remaining_timer_layout(egui_ctx, scale);
        self.draw_remaining_bullet_layout(egui_ctx, scale);
        self.draw_ex_skill_guage_layout(egui_ctx, scale);
        self.draw_result_font(egui_ctx, scale);
    }
}
