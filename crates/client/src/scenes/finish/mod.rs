mod enter;

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, ViewBorrow, World};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{
        ActionState, ActionStateTimer, CharacterKind, FinishPhasePlayer, LatLon, LoginToken,
        MovementState, MovementStateTimer, StageKind, Team, UserId, VictoryType,
        MAX_IN_GAME_PLAYERS,
    },
    protocol::{FinishStageResponsePacket, Packet},
};
use mod_physics::object3d::Frustum;
use mod_render::{UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT};
use winit::window::Window;

use crate::{
    asset::{MotionPool, StageBoundingVolumn, StageBoundingVolumnHierarchy, NOTOSANS_BOLD},
    component::{
        animate_character, bake_character, bake_character_eye_mouth, bake_stage,
        clear_render_target_with_skybox, collect_bake_resources,
        compute_frustum_corners_no_inverse, compute_light_view_proj_matrix, draw_character,
        draw_character_eye_mouth, draw_character_halo, draw_stage, update_action_state_timer,
        update_character_resource, update_entity_hierarchy, update_stage_resource,
        AccumRenderTarget, BakeList, BoneCollection, CameraDataLayout, CameraResource,
        CameraUniform, CharacterBakePipeline, CharacterRenderPipeline, Child, CompositePipeline,
        EyeMouthBakePipeline, EyeMouthRenderPipeline, GlobalLight, GlobalLightDataLayout,
        HaloRenderPipeline, LightSetResource, LightTransformDataLayout, MaterialKind, MeshRenderer,
        OpaqueMap, Projection, RevealRenderTarget, ShadowMap, Sibling, SkinnedMeshRenderer,
        SkinningAnimation, Skybox, SkyboxDataLayout, SkyboxRenderPipeline, StageBakePipeline,
        StageRenderPipeline, ToParentTrans, TransparentMap, TreeRenderPipeline, WorldTransform,
        RESET_POSITIONS, RESET_ROTATION,
    },
    config::{Locale, NUM_LOCALE},
    scenes::FatalErrorSceneLayer,
    SERVER_TCP_ADDR,
};

pub use self::enter::*;

use super::BASE_WIDTH;

/// 게임 장면의 최대 지속 시간입니다.
const MAX_SCENE_DURATION: f32 = 10.0;

/// 애플리케이션 표시 언어에 따른 나가기 버튼 텍스트
const EXIT_BTN_TEXTS: [&'static str; NUM_LOCALE] = ["나가기"];

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

    /// 나가기 버튼이 눌린 여부입니다.
    exit_btn_pressed: bool,

    ///엔터티를 관리하는 월드 객체입니다.
    world: World,
    /// 스카이박스입니다.
    skybox: Skybox,
    /// 게임 결과 장면의 메인 카메라 엔터티입니다.
    main_camera: Entity,
    /// 우승팀 플레이어 집합입니다.
    winner_players: Vec<Entity>,
    /// 지역 엔터티 집합입니다.
    stages: StageBoundingVolumnHierarchy,
    /// 프러스텀 컬링을 수행한 지형 엔터티 집합입니다.
    culling_stages: Vec<Entity>,

    /// 게임 진행 데이터입니다.
    play_data: Vec<FinishPhasePlayer>,

    /// 전역 조명 데이터입니다.
    global_light: Option<GlobalLight>,
    /// 조명 집합 쉐이더 리소스입니다.
    light_set_resource: LightSetResource,

    /// 반투명 오브젝트의 누적 값(Accumuldate)을 저장하는 렌더 타겟입니다.
    accum_render_target: Option<AccumRenderTarget>,
    /// 반투명 오브젝트의 노출 값(Revealage)을 저장하는 렌더 타겟입니다.
    reveal_render_target: Option<RevealRenderTarget>,
    /// 여러 렌더 타겟을 취합하는 파이프라인입니다.
    composite_pipeline: Option<CompositePipeline>,

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
        stages: StageBoundingVolumnHierarchy,
        play_data: Vec<FinishPhasePlayer>,
        global_light: Option<GlobalLight>,
        light_set_resource: LightSetResource,
        accum_render_target: AccumRenderTarget,
        reveal_render_target: RevealRenderTarget,
        composite_pipeline: CompositePipeline,
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
            exit_btn_pressed: false,
            world,
            skybox,
            main_camera: Entity::DANGLING,
            winner_players,
            stages,
            culling_stages: Vec::default(),
            play_data,
            global_light,
            light_set_resource,
            accum_render_target: Some(accum_render_target),
            reveal_render_target: Some(reveal_render_target),
            composite_pipeline: Some(composite_pipeline),
            ui_textures,
            bake_list: Vec::default(),
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
        let position = pivot + direction * 2.0 + glam::Vec3A::Y * 0.2;
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
            Projection::perspective(80f32.to_radians(), 16.0 / 9.0, 0.01, 100.0),
            camera_uniform,
            camera_resource,
            Frustum::from_mat4(glam::Mat4::IDENTITY),
        ));

        // 카메라 엔터티를 생성합니다.
        self.main_camera = self.world.spawn(builder.build());
    }

    /// 플레이어 위치를 재설정합니다.
    fn reset_player_position(&mut self) {
        type Query<'a> = (
            &'a (Team, usize),
            &'a mut ToParentTrans,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
        );

        // 플레이어 엔터티를 순회하면서
        // 승리 팀 플레이어 엔터티의 위치를 재설정합니다.
        for entity in self.winner_players.iter().cloned() {
            let (&(team, index), local_transform, action_state, action_state_timer) = self
                .world
                .query_one_mut::<Query>(entity)
                .expect("invalid entity or invalid entity component");

            // 액션 상태를 초기화합니다.
            *action_state = ActionState::VictoryStart;
            action_state_timer.reset();

            // 승리 팀 플레이어의 위치를 재설정합니다.
            local_transform.set_rotation_translation(
                RESET_ROTATION[self.stage_kind as usize][team as usize],
                RESET_POSITIONS[self.stage_kind as usize][index],
            );
        }
    }

    /// 여러 렌더 타겟을 취합하는 그래픽스 파이프라인을 생성합니다.
    fn create_composite_pipeline(&mut self, window: &Window, device: &wgpu::Device) {
        let (width, height) = window.inner_size().into();
        let accum_render_target = AccumRenderTarget::new(width, height, device);
        let reveal_render_target = RevealRenderTarget::new(width, height, device);
        let composite_pipeline = match self.composite_pipeline.take() {
            Some(pipeline) => pipeline.renew(device, &accum_render_target, &reveal_render_target),
            None => CompositePipeline::new(
                device,
                &accum_render_target,
                &reveal_render_target,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
            ),
        };

        self.composite_pipeline = Some(composite_pipeline);
        self.accum_render_target = Some(accum_render_target);
        self.reveal_render_target = Some(reveal_render_target);
    }
}

//--------------------------------------------------------------------------------------------
// 플레이어 조작과 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
    /// 캐릭터의 `ActionStateTimer`를 갱신합니다.
    fn update_action_state_and_timer(&mut self, elapsed_time_sec: f32) {
        type Query<'a> = (
            &'a CharacterKind,
            &'a mut ActionState,
            &'a mut ActionStateTimer,
        );

        // 캐릭터 종류, 행동 상태, 행동 상태 타이머를 가져옵니다.
        let mut view = self.world.view_mut::<Query>();
        for entity in self.winner_players.iter().cloned() {
            let (&character_kind, action_state, action_state_timer) = view
                .get_mut(entity)
                .expect("invalid entity or invalid entity component");

            update_action_state_timer(
                character_kind,
                action_state,
                action_state_timer,
                elapsed_time_sec,
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

    // /// 캐릭터의 무기를 갱신합니다.
    // fn update_character_weapon(&mut self) {
    //     type Query<'a> = (&'a CharacterKind, &'a ActionState, &'a SkinningAnimation);
    //     let element_view = self.world.view::<Query>();
    //     let child_view = self.world.view::<&Child>();
    //     let sibling_view = self.world.view::<&Sibling>();
    //     let mut transform_view = self.world.view::<(&ToParentTrans, &mut WorldTransform)>();

    //     for entity in self.winner_players.iter().cloned() {
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

    /// 캐릭터를 갱신합니다.
    fn update_character(&mut self) {
        self.animate_character();

        // 캐릭터의 계층 구조를 갱신합니다.
        for entity in self.winner_players.iter().cloned() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }
    }

    /// 지형 엔터티의 계층 구조를 갱신합니다.
    fn update_stage(&mut self) {
        for entity in self.culling_stages.iter().cloned() {
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

    /// 프러스텀 컬링(Frustum Culling)을 통해 렌더링을 수행할 지형 엔터티를 수집합니다.
    ///
    /// # Note
    /// 이 함수는 카메라의 월드 변환 행렬을 갱신한 후 호출되어야 합니다.
    ///
    fn culling_stages(&self) -> Vec<Entity> {
        // 카메라의 위치와 뷰 프러스텀을 가져옵니다.
        let mut query = self
            .world
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
        let mut query = self
            .world
            .query_one::<&WorldTransform>(self.main_camera)
            .expect("invalid entity");
        let transform = query.get().expect("invalid entity component");

        // 카메라의 뷰 프러스텀의 모서리 위치를 계산합니다.
        let frustum_corners = compute_frustum_corners_no_inverse(
            transform,
            80f32.to_radians(),
            width / height,
            0.01,
            15.0,
        );

        // 전역 조명의 변환 행렬을 계산합니다.
        let g_light = self.global_light.as_ref().unwrap();
        let light_proj_view =
            compute_light_view_proj_matrix(&frustum_corners, g_light.direction_w, 5.0);

        // 전역 조명 데이터 유니폼 버퍼를 갱신합니다.
        self.light_set_resource.global_light_uniform.update(
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
        let shadow_resource = self.light_set_resource.get_global();
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
        let mut entities: Vec<Entity> = self.winner_players.iter().cloned().collect();
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
// 사용자 인터페이스와 관련된 코드를 작성합니다.
//--------------------------------------------------------------------------------------------
impl InGameResultScene {
    /// 나가기 버튼을 그립니다.
    fn draw_exit_button(&mut self, egui_ctx: &egui::Context, scale: f32) {
        // 버튼 속성
        // - 기본 가로 크기: 240
        // - 기본 세로 크기: 80
        // - 기본 시작 위치: (976, 576)
        // - 기본 끝 위치: (1216, 656)
        let btn_rect = egui::Rect::from_min_max(
            egui::pos2(976.0 * scale, 576.0 * scale),
            egui::pos2(1216.0 * scale, 656.0 * scale),
        );
        let i = self.locale as usize;
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(18.0 * scale, family);
        let text = egui::RichText::new(EXIT_BTN_TEXTS[i])
            .font(font_id)
            .color(egui::Color32::BLACK);
        let button = egui::Button::new(text)
            .fill(egui::Color32::LIGHT_GRAY)
            .corner_radius(2.0);

        egui::Area::new(egui::Id::new("Exit_Btn_Layout")).show(egui_ctx, |ui| {
            ui.add_enabled_ui(!self.exit_btn_pressed, |ui| {
                if ui.put(btn_rect, button).clicked() {
                    self.exit_btn_pressed = true;
                }
            })
        });
    }
}

//--------------------------------------------------------------------------------------------
impl GameScene for InGameResultScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        self.create_main_camera(app.render_device());
        self.reset_player_position();
    }

    fn on_enter_foreground(&mut self, _window: &Window, app: &dyn AppHandle) {
        let event = AppEvent::CursorEnable;
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
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
        self.create_composite_pipeline(window, app.render_device());
    }

    fn on_update(&mut self, elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        self.update_action_state_and_timer(elapsed_time_sec);

        if self.exit_btn_pressed {
            // 패킷을 전송합니다.
            let packet = FinishStageResponsePacket::new(self.user_id, self.token);
            let net_manager = app.net_manager();
            let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
            socket.push_packet(packet.as_raw());

            // 이전 게임 장면으로 전환합니다.
            let scene_flow = GameSceneFlow::Pop;
            let event = AppEvent::AddGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
    }

    fn on_prepare_draw(&mut self, window: &Window, app: &dyn AppHandle) {
        self.update_camera();
        self.update_character();

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

        let child_view = &self.world.view::<&Child>();
        let sibling_view = &self.world.view::<&Sibling>();
        let transform_view = &self.world.view::<&WorldTransform>();
        let mesh_filter_view = &mut self.world.view::<MeshRenderer>();
        let skinned_mesh_filter_view = &mut self.world.view::<SkinnedMeshRenderer>();

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
        // 카메라 쉐이더 리소스를 가져옵니다.
        let camera_resource = self
            .world
            .query_one_mut::<&CameraResource>(self.main_camera)
            .cloned()
            .expect("invalid entity or invalid entity component");

        // 쉐이더 리소스를 가져옵니다.
        let accum_render_target = self.accum_render_target.as_ref().unwrap();
        let reveal_render_target = self.reveal_render_target.as_ref().unwrap();
        let composite_pipeline = self.composite_pipeline.as_ref().unwrap();

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

            for ((mesh, kind), material_resources) in self.opaque_map.iter() {
                let func = match kind {
                    MaterialKind::Character => {
                        draw_character(
                            &mesh,
                            CharacterRenderPipeline::get().unwrap(),
                            &camera_resource,
                            &self.light_set_resource,
                            &material_resources,
                            &mut rpass,
                        );
                        continue;
                    }
                    MaterialKind::CharacterEyeMouth => {
                        draw_character_eye_mouth(
                            &mesh,
                            EyeMouthRenderPipeline::get().unwrap(),
                            &camera_resource,
                            &self.light_set_resource,
                            &material_resources,
                            &mut rpass,
                        );
                        continue;
                    }
                    MaterialKind::CharacterHalo => draw_character_halo,
                    MaterialKind::Stage => {
                        draw_stage(
                            &mesh,
                            StageRenderPipeline::get().unwrap(),
                            &camera_resource,
                            &self.light_set_resource,
                            &material_resources,
                            &mut rpass,
                        );
                        continue;
                    }
                    MaterialKind::Tree => {
                        draw_stage(
                            &mesh,
                            TreeRenderPipeline::get().unwrap(),
                            &camera_resource,
                            &self.light_set_resource,
                            material_resources,
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
                    MaterialKind::Stage => StageRenderPipeline::get(),
                    _ => continue,
                }
                .unwrap();

                func(
                    &mesh,
                    pipeline,
                    &camera_resource,
                    &material_resources,
                    &mut rpass,
                );
            }

            clear_render_target_with_skybox(
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
                        view: accum_render_target.view(),
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
                        view: reveal_render_target.view(),
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

            composite_pipeline.process(&mut rpass);
        }
        encoder.pop_debug_group();
    }

    fn on_finish_draw(&mut self, _window: &Window, _app: &dyn AppHandle) {
        self.opaque_map.clear();
        self.transparent_map.clear();
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        let egui_ctx = app.egui_ctx();
        self.draw_exit_button(egui_ctx, scale);
    }
}
