use std::sync::atomic;

use ahash::{HashMap, RandomState};
use hecs::{Entity, World};
use mod_app::{
    app::AppHandle,
    etc::{AppEvent, WindowSize},
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, StageKind, UserId, MAX_IN_GAME_PLAYERS},
    protocol::{InGameEnterNotifyPacket, InGameReadyNotifyPacket, Packet, PacketType, RawPacket},
};
use mod_render::{UiRenderer, SWAPCHAIN_FORMAT};
use winit::window::Window;

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, StageBoundingVolumnHierarchy,
        TextureDataPool, TexturePool, TextureViewPool, NOTOSANS_BOLD,
    },
    component::{
        AccumRenderTarget, AlphaBlendPipeline, BloomPipeline, BrightRenderTarget, DirectionLight,
        GaussianBlurPipeline, LightSetResource, PlayerArchetype, RevealRenderTarget, Skybox,
        GLOBAL_SHADOW_MAP_SIZE, LOCAL_SHADOW_MAP_SIZE,
    },
    config::{Locale, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, InGameEnterScene, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["다른 플레이어를 기다리는 중"];

/// `InGameReadyScene` 생성 빌더입니다.
pub struct InGameReadySceneBuilder {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 스테이지 종류
    stage_kind: StageKind,

    /// 플레이어 엔터티
    players: HashMap<UserId, (Entity, PlayerArchetype)>,
    /// 스테이지 엔터티
    stage: Option<StageBoundingVolumnHierarchy>,

    /// 스테이지 스카이박스
    skybox: Option<Skybox>,
    /// 스테이지 방향 조명
    direction_light: Option<DirectionLight>,

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

impl InGameReadySceneBuilder {
    /// 새로운 빌더를 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        stage_kind: StageKind,
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
            players: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::new()),
            stage: None,
            skybox: None,
            direction_light: None,
            mesh_pool,
            model_pool,
            motion_pool,
            texture_pool,
            texture_data_pool,
            texture_view_pool,
            sampler_pool,
        }
    }

    /// 플레이어를 추가합니다.
    pub fn insert_player(&mut self, uid: UserId, entity: Entity, archetype: PlayerArchetype) {
        self.players.insert(uid, (entity, archetype));
    }

    /// 스테이지 엔터티를 설정합니다.
    pub fn set_stage(&mut self, stage: StageBoundingVolumnHierarchy) {
        self.stage = Some(stage);
    }

    /// 스카이박스를 설정합니다.
    pub fn set_skybox(&mut self, skybox: Skybox) {
        self.skybox = Some(skybox);
    }

    /// 스테이지 방향 조명을 설정합니다.
    pub fn set_direction_light(&mut self, direction_light: DirectionLight) {
        self.direction_light = Some(direction_light);
    }

    pub fn build(self, world: World) -> InGameReadyScene {
        InGameReadyScene {
            locale: self.locale,
            uid: self.uid,
            token: self.token,
            stage_kind: self.stage_kind,
            world: Some(world),
            players: self.players,
            stage: self.stage,
            skybox: self.skybox,
            direction_light: self.direction_light,
            light_resource: None,
            accum_render_target: None,
            reveal_render_target: None,
            bright_render_target: None,
            alpha_blend_pipeline: None,
            gaussian_blur_pipeline: None,
            bloom_pipeline: None,
            mesh_pool: self.mesh_pool,
            model_pool: self.model_pool,
            motion_pool: self.motion_pool,
            texture_pool: self.texture_pool,
            texture_data_pool: self.texture_data_pool,
            texture_view_pool: self.texture_view_pool,
            sampler_pool: self.sampler_pool,
        }
    }
}

/// 다른 플레이어가 게임 월드의 구성 요소를 생성을 완료할 때 까지 대기하는 장면입니다.
pub struct InGameReadyScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 스테이지 종류
    stage_kind: StageKind,

    /// 게임 월드
    world: Option<World>,
    /// 플레이어 엔터티
    players: HashMap<UserId, (Entity, PlayerArchetype)>,
    /// 스테이지 엔터티
    stage: Option<StageBoundingVolumnHierarchy>,

    /// 스테이지 스카이박스
    skybox: Option<Skybox>,
    /// 스테이지 방향 조명
    direction_light: Option<DirectionLight>,
    /// 조명 쉐이더 리소스
    light_resource: Option<LightSetResource>,

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

impl InGameReadyScene {
    /// 조명 집합 쉐이더 리소스를 생성합니다.
    fn create_light_set_resource(&mut self, device: &wgpu::Device) {
        // 방향성 조명을 가져옵니다.
        let directional_light = self
            .direction_light
            .as_ref()
            .expect("the directional ligth must be exists!");

        // 조명 쉐이더 리소스를 생성합니다.
        let resource = LightSetResource::new(
            Some(&format!("{:?}", &self.stage_kind)),
            device,
            &directional_light.shadow_map_view,
            &directional_light.shadow_sampler,
            GLOBAL_SHADOW_MAP_SIZE,
            LOCAL_SHADOW_MAP_SIZE,
        );

        self.light_resource = Some(resource);
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
}

impl GameScene for InGameReadyScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let size = app.window_size();
        let device = app.render_device();
        self.create_light_set_resource(device);
        self.create_weighted_blend_oit_resource(size, device);
        self.create_bloom_resource(size, device);

        // 준비 완료 패킷을 전송합니다.
        let packet = InGameReadyNotifyPacket::new(self.uid, self.token);
        let net = app.net_manager();
        let socket = net.get(&SERVER_TCP_ADDR).unwrap();
        socket.push_packet(packet.as_raw());
    }

    fn on_window_resized(&mut self, _window: &Window, app: &dyn AppHandle) {
        if self.world.is_some() {
            let size = app.window_size();
            let device = app.render_device();
            self.create_weighted_blend_oit_resource(size, device);
            self.create_bloom_resource(size, device);
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
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::InGameEnterNotify => {
                let packet = InGameEnterNotifyPacket::from_raw(packet);

                // 다음 게임 장면으로 전환합니다.
                let world = match self.world.take() {
                    Some(world) => world,
                    None => return None,
                };
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
                let scene = InGameEnterScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    self.stage_kind,
                    packet.remaining_time_ms,
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
        };

        None
    }

    fn on_draw(
        &mut self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({:?})", &self)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
    }

    fn ui_callback(&mut self, window: &Window, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let viewport = app.viewport();
        let scale_factor = window.scale_factor() as f32;
        let scale = viewport.width / scale_factor / BASE_WIDTH;
        let clip_rect = egui::Rect::from_min_size(
            egui::pos2(viewport.x, viewport.y) / scale_factor,
            egui::vec2(viewport.width, viewport.height) / scale_factor,
        );

        // 로드 텍스트
        let text = LOAD_TEXTS[i];
        let family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let font_id = egui::FontId::new(28.0 * scale, family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);
        let label = egui::Label::new(text)
            .sense(egui::Sense::empty())
            .selectable(false);

        let ctx = app.egui_ctx();
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                ui.shrink_clip_rect(clip_rect);

                let width = clip_rect.width() * 0.3;
                let height = width * 0.25;
                let size = egui::vec2(width, height);
                let max = clip_rect.max;
                let min = max - size;
                let rect = egui::Rect::from_min_max(min, max);
                ui.put(rect, label);
            });
    }
}
