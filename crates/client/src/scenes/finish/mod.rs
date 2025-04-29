mod enter;

use std::sync::Arc;

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, FinishPhasePlayer, LatLon, LoginToken,
    MovementState, MovementStateTimer, StageKind, Team, UserId, VictoryType, MAX_IN_GAME_PLAYERS,
};
use mod_physics::object3d::Frustum;
use winit::window::Window;

use crate::{
    asset::MotionPool,
    component::{
        animate_character, set_weapon_position, update_entity_hierarchy, AttributeKind,
        BoneCollection, CameraDataLayout, CameraResource, CameraUniform, CharacterRenderPipeline,
        Child, EyeMouthRenderPipeline, HaloRenderPipeline, MaterialKind, MaterialResource, Mesh,
        MeshFilter, MeshRenderer, OpaqueMap, Projection, ShadowMap, ShadowResource, Sibling,
        SkinnedMeshRenderer, SkinningAnimation, Skybox, SkyboxDataLayout, SkyboxRenderPipeline,
        StageRenderPipeline, ToParentTrans, TransformDataLayout, TransparentMap,
        WeightedBlendedOITRenderPipeline, WeightedBlendedOITResource, WorldTransform,
        NUM_CUBE_VERTICES, RESET_POSITIONS, RESET_ROTATION,
    },
    config::{Locale, NUM_LOCALE},
    scenes::FatalErrorSceneLayer,
};

pub use self::enter::*;

/// 게임 장면의 최대 지속 시간입니다.
const MAX_SCENE_DURATION: f32 = 10.0;

/// 인게임 장면의 결과를 보여주는 장면입니다.
pub struct InGameResultScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 사용자 식별자입니다.
    user_id: UserId,
    /// 로그인 토큰입니다.
    token: LoginToken,

    /// 승리 팀
    winner: Team,
    /// 승리 종류
    victory_type: VictoryType,
    /// 게임 플레이 시간
    play_time: f32,
    /// 스테이지 종류
    stage_kind: StageKind,

    ///엔터티를 관리하는 월드 객체입니다.
    world: World,
    /// 스카이박스입니다.
    skybox: Skybox,
    /// 게임 결과 장면의 메인 카메라 엔터티입니다.
    main_camera: Entity,
    /// 우승팀 플레이어 집합입니다.
    winner_players: Vec<Entity>,
    /// 스테이지 엔터티 집합입니다.
    stages: Vec<Entity>,

    /// 게임 진행 데이터입니다.
    play_data: Vec<FinishPhasePlayer>,

    /// 그림자 쉐이더 리소스입니다.
    shadow_resource: ShadowResource,
    /// 알파 블렌딩 쉐이더 리소스입니다.
    alpha_blend_resource: WeightedBlendedOITResource,

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
// InGameDominationModeScene에서 사용되는 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
    /// 승리한 팀의 플레이어 엔터티를 가져옵니다.
    pub fn get_winner_players(
        winner: Team,
        world: &mut World,
        players: HashMap<UserId, Entity>,
        disconnected_players: Vec<Entity>,
    ) -> Vec<Entity> {
        let mut entities = disconnected_players;
        entities.extend(players.values());

        type Query<'a> = &'a (Team, usize);
        let mut winner_players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for entity in entities {
            let &(team, _) = world
                .query_one_mut::<Query>(entity)
                .expect("invalid entity or invalid entity component");

            if winner == team {
                winner_players.push(entity);
            }
        }

        winner_players
    }
}

//--------------------------------------------------------------------------------------------
// 초기화 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
    /// 새로운 `InGameResultScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        winner: Team,
        victory_type: VictoryType,
        play_time: f32,
        stage_kind: StageKind,
        world: World,
        skybox: Skybox,
        winner_players: Vec<Entity>,
        stages: Vec<Entity>,
        play_data: Vec<FinishPhasePlayer>,
        shadow_resource: ShadowResource,
        alpha_blend_resource: WeightedBlendedOITResource,
        ui_textures: HashMap<String, egui::load::SizedTexture>,
        motion_pool: MotionPool,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            winner,
            victory_type,
            play_time,
            stage_kind,
            world,
            skybox,
            main_camera: Entity::DANGLING,
            winner_players,
            stages,
            play_data,
            ui_textures,
            shadow_resource,
            alpha_blend_resource,
            shadow_map: HashMap::default(),
            opaque_map: HashMap::default(),
            transparent_map: HashMap::default(),
            motion_pool,
        }
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, device: &wgpu::Device) {
        // 카메라의 위치와 방향을 설정합니다.
        let pivot = glam::Vec3A::Y * 0.6;
        let direction = RESET_ROTATION[self.stage_kind as usize][self.winner as usize];
        let direction = direction.mul_vec3a(glam::Vec3A::Z).normalize();
        let position = pivot + direction * 2.0 + glam::Vec3A::Y * 0.05;
        let look = (pivot - position).normalize();
        let right = glam::Vec3A::Y.cross(look);
        let up = look.cross(right);
        let cam_trans = glam::Mat4::from_mat3_translation(
            glam::mat3(right.into(), up.into(), look.into()),
            position.into(),
        );

        // 카메라 쉐이더 리소스를 생성합니다.
        let camera_uniform = CameraUniform::uninit(Some("Finish"), device);
        let camera_resource = CameraResource::new(Some("Finish"), device, &camera_uniform);

        // 로컬 변환 행렬, 월드 변환 행렬, 투영 변환 행렬 컴포넌트를 추가합니다.
        let mut builder = EntityBuilder::new();
        builder.add_bundle((
            ToParentTrans(cam_trans),
            WorldTransform::default(),
            Projection::perspective(50f32.to_radians(), 16.0 / 9.0, 0.01, 100.0),
            camera_uniform,
            camera_resource,
            Frustum::from_mat4(glam::Mat4::IDENTITY),
        ));

        // 카메라 엔터티를 생성합니다.
        self.main_camera = self.world.spawn(builder.build());
    }

    /// 플레이어 위치를 재설정합니다.
    fn reset_player_position(&mut self) {
        type Query<'a> = (&'a (Team, usize), &'a mut ToParentTrans);

        // 플레이어 엔터티를 순회하면서
        // 승리 팀 플레이어 엔터티의 위치를 재설정합니다.
        for entity in self.winner_players.iter().cloned() {
            let (&(team, index), local_transform) = self
                .world
                .query_one_mut::<Query>(entity)
                .expect("invalid entity or invalid entity component");

            // 승리 팀 플레이어의 위치를 재설정합니다.
            local_transform.set_rotation_translation(
                RESET_ROTATION[self.stage_kind as usize][team as usize],
                RESET_POSITIONS[self.stage_kind as usize][index],
            );
        }
    }
}

//--------------------------------------------------------------------------------------------
// 엔터티 계층 구조 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
    /// 카메라를 갱신합니다.
    fn update_camera(&mut self) {
        update_entity_hierarchy(&mut self.world, self.main_camera, glam::Mat4::IDENTITY);
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

        for entity in self.winner_players.iter().cloned() {
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

        for entity in self.winner_players.iter().cloned() {
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

    /// 캐릭터를 갱신합니다.
    fn update_character(&mut self) {
        self.animate_character();

        // 캐릭터의 계층 구조를 갱신합니다.
        for entity in self.winner_players.iter().cloned() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }

        self.update_character_weapon();
    }

    /// 지형 엔터티의 계층 구조를 갱신합니다.
    fn update_stage(&mut self) {
        for entity in self.stages.iter().cloned() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }
}

//--------------------------------------------------------------------------------------------
// 쉐이더 리소스 갱신과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
    fn update_camera_and_skybox_resource(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        type Query<'a> = (
            &'a CameraUniform,
            &'a WorldTransform,
            &'a Projection,
            &'a mut Frustum,
        );

        // 카메라 엔터티의 요소를 가져옵니다.
        let (uniform, trans, proj, frustum) = self
            .world
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
        self.skybox.uniform.update(
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
        self.winner_players.clone()
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
}

//--------------------------------------------------------------------------------------------
// 렌더링과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
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
}

//--------------------------------------------------------------------------------------------
// 시스템 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
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
}

//--------------------------------------------------------------------------------------------
impl GameScene for InGameResultScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {
        self.enable_cursor(window);
        self.create_main_camera(app.render_device());
        self.reset_player_position();
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

    fn on_prepare_draw(&mut self, _window: &Window, app: &dyn AppHandle) {
        self.update_camera();
        self.update_character();

        let device = app.render_device();
        let queue = app.render_queue();
        let mut staging_buffers = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 카메라 쉐이더 리소스를 갱신합니다.
        self.update_camera_and_skybox_resource(device, &mut encoder, &mut staging_buffers);

        let mut shadow_map = HashMap::default();
        let mut opaque_map = HashMap::default();
        let mut transparent_map = HashMap::default();

        let child_view = &self.world.view::<&Child>();
        let sibling_view = &self.world.view::<&Sibling>();
        let transform_view = &self.world.view::<&WorldTransform>();
        let mesh_filter_view = &mut self.world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &mut self.world.view::<SkinnedMeshRenderer>();

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
        // 카메라 쉐이더 리소스를 가져옵니다.
        let camera_resource = self
            .world
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

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

            Self::clear_render_target_with_skybox(
                &self.skybox,
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
                        view: &self.alpha_blend_resource.accum_render_target,
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
                        view: &self.alpha_blend_resource.reveal_render_target,
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
            rpass.set_bind_group(0, &self.alpha_blend_resource.bind_group, &[]);
            rpass.draw(0..4, 0..1);
        }
        encoder.pop_debug_group();
    }

    fn on_finish_draw(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.shadow_map.clear();
        self.opaque_map.clear();
        self.transparent_map.clear();
    }
}
