use std::{
    error::Error,
    fs::OpenOptions,
    io::{Cursor, Read},
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use ahash::HashSet;
use ddsfile::Dds;
use image::{ImageFormat, ImageReader};
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{BulletKind, CharacterKind, LoginToken, StageLayoutData, UserId},
    protocol::{InitStagePacket, RawPacket},
};
use mod_parallelism::collections::Queue;
use rayon::ThreadPool;
use winit::window::Window;

use crate::{
    asset::{
        MeshPool, ModelPool, MotionPool, SamplerPool, TextureDataPool, TexturePool,
        TextureViewPool, BULLET_URIS, BULLET_WORKSPACE, CAPTURE_ZONE_URI, CHARACTER_ICON_URIS,
        CHARACTER_URIS, CHARACTER_WORKSPACES, DAMAGE_FONT_URI, NOTOSANS_BOLD, SCHALE_ICON_URI,
        SKYBOX_URI, STAGE_URI, STAGE_WORKSPACES, TIMER_ICON_URI, UI_GAME_LAYOUT_URI,
        WEAPON_ICON_MASK_URI, WEAPON_ICON_MASK_URIS, WEAPON_ICON_URI, WEAPON_ICON_URIS,
    },
    component::{load_stage_layout_from_file, Attributes, MaterialDataPool, Mesh, Vertices},
    config::{Locale, NUM_LOCALE},
    scenes::{FatalErrorSceneLayer, BASE_WIDTH},
};

use super::InGameBuildScene;

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];
/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["오류"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["게임 리소스를 로드하는데 실패했습니다!"];

/// 게임 월드에 필요한 에셋을 로드하는 장면입니다.
pub struct InGameLoadScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 클라이언트 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 플레이어 캐릭터 종류
    character_kind: CharacterKind,
    /// 초기화 패킷
    packet: Option<InitStagePacket>,

    /// 그래픽스 명령어 작업
    commands: Arc<Queue<(Vec<wgpu::Buffer>, wgpu::CommandBuffer)>>,
    /// 작업 결과를 저장합니다.
    task_results: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,
    /// 남은 작업의 수
    num_remaining_tasks: usize,

    /// 로드된 에셋 데이터입니다.
    stage_layout_data: Arc<OnceLock<StageLayoutData>>,

    /// 재질 데이터 풀 객체입니다.
    material_data_pool: MaterialDataPool,
    /// 메쉬 풀 객체입니다.
    mesh_pool: MeshPool,
    /// 모델 풀 객체입니다.
    model_pool: ModelPool,
    /// 애니메이션 데이터 풀 객체입니다.
    motion_pool: MotionPool,
    /// 텍스처 데이터 풀 객체입니다.
    texture_data_pool: TextureDataPool,
    /// 텍스처 풀 객체입니다.
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체입니다.
    texture_view_pool: TextureViewPool,
    /// 텍스처 샘플러 풀 객체입니다.
    sampler_pool: SamplerPool,
}

impl InGameLoadScene {
    /// 새로운 `InGameLoadScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        packet: InitStagePacket,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            character_kind: packet
                .players
                .iter()
                .find(|item| item.account.uid == user_id)
                .map(|item| item.character_kind)
                .unwrap_or_default(),
            packet: Some(packet),
            commands: Arc::new(Queue::new()),
            task_results: Arc::new(Queue::new()),
            num_remaining_tasks: 0,
            stage_layout_data: Arc::new(OnceLock::new()),
            material_data_pool: MaterialDataPool::new(),
            mesh_pool: MeshPool::new(),
            model_pool: ModelPool::new(),
            motion_pool: MotionPool::new(),
            texture_data_pool: TextureDataPool::new(),
            texture_pool: TexturePool::new(),
            texture_view_pool: TextureViewPool::new(),
            sampler_pool: SamplerPool::new(),
        }
    }

    /// 사용되는 캐릭터 모델을 로드합니다.
    fn load_character_models(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let character_kinds: HashSet<_> = init_stage_packet
            .players
            .iter()
            .map(|player| player.character_kind)
            .collect();

        for kind in character_kinds {
            let commands = self.commands.clone();
            let task_results = self.task_results.clone();
            let material_data_pool = self.material_data_pool.clone();
            let mesh_pool = self.mesh_pool.clone();
            let model_pool = self.model_pool.clone();
            let texture_data_pool = self.texture_data_pool.clone();
            let texture_pool = self.texture_pool.clone();
            let texture_view_pool = self.texture_view_pool.clone();
            let sampler_pool = self.sampler_pool.clone();
            let device = device.clone();

            let mut workspace = workspace.clone();
            workspace.push(CHARACTER_WORKSPACES[kind as usize]);

            thread_pool.spawn(move || {
                // 스레드의 커맨드 버퍼를 생성합니다.
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                // 캐릭터 모델 데이터를 로드합니다.
                let result = model_pool
                    .get_or_init(
                        &mesh_pool,
                        &material_data_pool,
                        &texture_data_pool,
                        &texture_pool,
                        &texture_view_pool,
                        &sampler_pool,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        &workspace,
                        CHARACTER_URIS[kind as usize],
                    )
                    .map(|_| ())
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>);

                // 커맨드 버퍼를 전송합니다.
                commands.push((staging_buffers, encoder.finish()));
                // 결과를 전송합니다.
                task_results.push(result);
            });
            self.num_remaining_tasks += 1;
        }
    }

    /// 캐릭터 애니메이션 데이터를 로드합니다.
    fn load_character_motions(&mut self, workspace: &PathBuf, thread_pool: &ThreadPool) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let character_kinds: HashSet<_> = init_stage_packet
            .players
            .iter()
            .map(|player| player.character_kind)
            .collect();

        for kind in character_kinds {
            let task_results = self.task_results.clone();
            let motion_pool = self.motion_pool.clone();

            let mut workspace = workspace.clone();
            workspace.push(CHARACTER_WORKSPACES[kind as usize]);

            thread_pool.spawn(move || {
                // 캐릭터 애니메이션 데이터를 로드합니다.
                let result = motion_pool
                    .get_or_init(workspace, CHARACTER_URIS[kind as usize])
                    .map(|_| ())
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>);

                // 결과를 전송합니다.
                task_results.push(result);
            });
            self.num_remaining_tasks += 1;
        }
    }

    /// 사용되는 총알 모델을 로드합니다.
    fn load_bullet_models(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let bullet_kinds: HashSet<BulletKind> = init_stage_packet
            .players
            .iter()
            .map(|player| player.character_kind.into())
            .collect();

        for kind in bullet_kinds {
            let commands = self.commands.clone();
            let task_results = self.task_results.clone();
            let material_data_pool = self.material_data_pool.clone();
            let model_pool = self.model_pool.clone();
            let mesh_pool = self.mesh_pool.clone();
            let texture_data_pool = self.texture_data_pool.clone();
            let texture_pool = self.texture_pool.clone();
            let texture_view_pool = self.texture_view_pool.clone();
            let sampler_pool = self.sampler_pool.clone();
            let device = device.clone();

            let mut workspace = workspace.clone();
            workspace.push(BULLET_WORKSPACE);

            thread_pool.spawn(move || {
                // 스레드의 커맨드 버퍼를 생성합니다.
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                // 총알 모델을 로드합니다.
                let result = model_pool
                    .get_or_init(
                        &mesh_pool,
                        &material_data_pool,
                        &texture_data_pool,
                        &texture_pool,
                        &texture_view_pool,
                        &sampler_pool,
                        &device,
                        &mut encoder,
                        &mut staging_buffers,
                        workspace,
                        BULLET_URIS[kind as usize],
                    )
                    .map(|_| ())
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>);

                // 커맨드 버퍼를 전송합니다.
                commands.push((staging_buffers, encoder.finish()));
                // 결과를 전송합니다.
                task_results.push(result);
            });
            self.num_remaining_tasks += 1;
        }
    }

    /// 지역 모델들을 로드합니다.
    fn load_stage_models(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let init_stage_packet = self
            .packet
            .as_ref()
            .expect("the InitStagePacket must exist!");
        let i = init_stage_packet.stage_kind() as usize;

        let mut workspace = workspace.clone();
        workspace.push(STAGE_WORKSPACES[i]);

        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let stage_layout_data = self.stage_layout_data.clone();
        let material_data_pool = self.material_data_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let device = device.clone();

        thread_pool.spawn(move || {
            // 지형 데이터를 로드합니다.
            let result = load_stage_layout_from_file(&workspace, STAGE_URI);
            let layout = match result {
                Ok(layout) => layout,
                Err(e) => {
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 스레드의 커맨드 버퍼를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            for uri in layout.models.iter() {
                // 지형 데이터를 구성하는 모델을 로드합니다.
                let result = model_pool.get_or_init(
                    &mesh_pool,
                    &material_data_pool,
                    &texture_data_pool,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                    &device,
                    &mut encoder,
                    &mut staging_buffers,
                    &workspace,
                    uri,
                );

                // 결과를 전송합니다.
                if let Err(e) = result {
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            }

            // 스테이지 모델 데이터를 저장합니다.
            stage_layout_data
                .set(layout)
                .expect("the stage layout data already exist!");

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));
            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    fn load_capture_zone_model(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let mut workspace = workspace.clone();
        workspace.push(STAGE_WORKSPACES[0]);

        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let material_data_pool = self.material_data_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let device = device.clone();

        thread_pool.spawn(move || {
            // 스레드의 커맨드 버퍼를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // 지형 데이터를 구성하는 모델을 로드합니다.
            let result = model_pool.get_or_init(
                &mesh_pool,
                &material_data_pool,
                &texture_data_pool,
                &texture_pool,
                &texture_view_pool,
                &sampler_pool,
                &device,
                &mut encoder,
                &mut staging_buffers,
                &workspace,
                CAPTURE_ZONE_URI,
            );

            // 결과를 전송합니다.
            if let Err(e) = result {
                task_results.push(Err(Box::new(e)));
                return;
            }

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));
            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// `UI_Game_Layout` 텍스처를 생성합니다.
    fn create_ui_layout_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();

        let mut path = workspace.clone();
        path.push(format!("ui/{}.png", UI_GAME_LAYOUT_URI));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let mut reader = ImageReader::new(Cursor::new(buf));
            reader.set_format(ImageFormat::Png);

            let image = match reader.decode() {
                Ok(image) => image,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                UI_GAME_LAYOUT_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                image.width(),
                image.height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
                1,
                image.to_rgba8().to_vec(),
            );

            // 텍스터를 등록합니다.
            texture_pool.insert(UI_GAME_LAYOUT_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// `Timer_Icon` 텍스처를 생성합니다.
    fn create_timer_icon_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();

        let mut path = workspace.clone();
        path.push(format!("ui/{}.dds", TIMER_ICON_URI));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let result = Dds::read(Cursor::new(buf));
            let dds = match result {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                TIMER_ICON_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                dds.get_width(),
                dds.get_height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
                1,
                dds.data,
            );

            // 텍스터를 등록합니다.
            texture_pool.insert(TIMER_ICON_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// `Schale_Icon` 텍스처를 생성합니다.
    fn create_schale_icon_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();

        let mut path = workspace.clone();
        path.push(format!("ui/{}.dds", SCHALE_ICON_URI));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let result = Dds::read(Cursor::new(buf));
            let dds = match result {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                SCHALE_ICON_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                dds.get_width(),
                dds.get_height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
                1,
                dds.data,
            );

            // 텍스터를 등록합니다.
            texture_pool.insert(SCHALE_ICON_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 캐릭터 아이콘 텍스처를 생성합니다.
    fn create_character_icon_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let characters: HashSet<_> = self
            .packet
            .as_ref()
            .expect("the init stage packet must exist!")
            .players
            .iter()
            .map(|item| item.character_kind)
            .collect();

        for kind in characters {
            let commands = self.commands.clone();
            let task_results = self.task_results.clone();
            let texture_pool = self.texture_pool.clone();
            let device = device.clone();
            let i = kind as usize;

            let mut path = workspace.clone();
            path.push(format!("ui/{}.dds", CHARACTER_ICON_URIS[i]));

            thread_pool.spawn(move || {
                log::debug!("open texture asset (PATH:{})", path.display());
                let result = OpenOptions::new().read(true).write(false).open(&path);
                let mut file = match result {
                    Ok(file) => file,
                    Err(e) => {
                        log::error!(
                            "failed to texture asset (PATH:{}, REASON:{})",
                            path.display(),
                            &e
                        );
                        task_results.push(Err(Box::new(e)));
                        return;
                    }
                };

                log::debug!("read texture asset (PATH:{})", path.display());
                let mut buf = Vec::new();
                if let Err(e) = file.read_to_end(&mut buf) {
                    log::error!(
                        "failed to read texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }

                log::debug!("close texture asset (PATH:{})", path.display());
                drop(file);

                log::debug!("decode texture asset (PATH:{})", path.display());
                let result = Dds::read(Cursor::new(buf));
                let dds = match result {
                    Ok(dds) => dds,
                    Err(e) => {
                        log::error!(
                            "failed to decode texture asset (PATH:{}, REASON:{})",
                            path.display(),
                            &e
                        );
                        task_results.push(Err(Box::new(e)));
                        return;
                    }
                };

                // 텍스터를 생성합니다.
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                let texture = TexturePool::create_texture(
                    CHARACTER_ICON_URIS[i],
                    &device,
                    &mut encoder,
                    &mut staging_buffers,
                    dds.get_width(),
                    dds.get_height(),
                    1,
                    wgpu::TextureDimension::D2,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    1,
                    1,
                    dds.data,
                );

                // 텍스터를 등록합니다.
                texture_pool.insert(CHARACTER_ICON_URIS[i], texture);

                // 커맨드 버퍼를 전송합니다.
                commands.push((staging_buffers, encoder.finish()));

                // 결과를 전송합니다.
                task_results.push(Ok(()));
            });
            self.num_remaining_tasks += 1;
        }
    }

    /// 플레이어 캐릭터의 무기 아이콘 텍스처를 생성합니다.
    fn create_player_weapon_icon_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();
        let i = self.character_kind as usize;

        let mut path = workspace.clone();
        path.push(format!("ui/{}.dds", WEAPON_ICON_URIS[i]));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let result = Dds::read(Cursor::new(buf));
            let dds = match result {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                WEAPON_ICON_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                dds.get_width(),
                dds.get_height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
                1,
                dds.data,
            );

            // 텍스터를 등록합니다.
            texture_pool.insert(WEAPON_ICON_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 플레이어 캐릭터의 무기 아이콘 마스크의 텍스처를 생성합니다.
    fn create_player_weapon_icon_mask_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();
        let i = self.character_kind as usize;

        let mut path = workspace.clone();
        path.push(format!("ui/{}.dds", WEAPON_ICON_MASK_URIS[i]));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let result = Dds::read(Cursor::new(buf));
            let dds = match result {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                WEAPON_ICON_MASK_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                dds.get_width(),
                dds.get_height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                1,
                1,
                dds.data,
            );

            // 텍스터를 등록합니다.
            texture_pool.insert(WEAPON_ICON_MASK_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 스카이박스 텍스처를 생성합니다.
    fn create_skybox_texture(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();

        let mut path = workspace.clone();
        path.push(format!("stage/{}.dds", SKYBOX_URI));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let dds = match Dds::read(Cursor::new(buf)) {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스처를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                SKYBOX_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                dds.get_width(),
                dds.get_height(),
                6,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Bc7RgbaUnorm,
                dds.get_num_mipmap_levels(),
                1,
                dds.data,
            );

            // 텍스처를 등록합니다.
            texture_pool.insert(SKYBOX_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 데미지 폰트 텍스처를 생성합니다.
    fn create_damage_font(
        &mut self,
        workspace: &PathBuf,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let texture_pool = self.texture_pool.clone();
        let device = device.clone();

        let mut path = workspace.clone();
        path.push(format!("font/{}.dds", DAMAGE_FONT_URI));

        thread_pool.spawn(move || {
            log::debug!("open texture asset (PATH:{})", path.display());
            let result = OpenOptions::new().read(true).write(false).open(&path);
            let mut file = match result {
                Ok(file) => file,
                Err(e) => {
                    log::error!(
                        "failed to texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            log::debug!("read texture asset (PATH:{})", path.display());
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf) {
                log::error!(
                    "failed to read texture asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                task_results.push(Err(Box::new(e)));
                return;
            }

            log::debug!("close texture asset (PATH:{})", path.display());
            drop(file);

            log::debug!("decode texture asset (PATH:{})", path.display());
            let dds = match Dds::read(Cursor::new(buf)) {
                Ok(dds) => dds,
                Err(e) => {
                    log::error!(
                        "failed to decode texture asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 텍스터를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder: wgpu::CommandEncoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            let texture = TexturePool::create_texture(
                DAMAGE_FONT_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                dds.get_width(),
                dds.get_height(),
                1,
                wgpu::TextureDimension::D2,
                wgpu::TextureFormat::Bc7RgbaUnorm,
                1,
                1,
                dds.data,
            );

            // 텍스터를 등록합니다.
            texture_pool.insert(DAMAGE_FONT_URI, texture);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }

    /// 데미지 파티클 메쉬를 생성합니다.
    fn create_damage_particle_mesh(
        &mut self,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
    ) {
        let commands = self.commands.clone();
        let task_results = self.task_results.clone();
        let mesh_pool = self.mesh_pool.clone();
        let device = device.clone();

        thread_pool.spawn(move || {
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // 데미지 파티클 메쉬를 생성합니다.
            let positions = Vertices(vec![
                [-0.025, -0.05, 0.0],
                [-0.025, 0.05, 0.0],
                [0.025, -0.05, 0.0],
                [0.025, 0.05, 0.0],
                [0.025, -0.05, 0.0],
                [-0.025, 0.05, 0.0],
            ]);
            let texcoords = Attributes::Texcoord0(vec![
                [0.0, 1.0],
                [0.0, 0.0],
                [1.0, 1.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 0.0],
            ]);

            let mut mesh = Mesh::new(
                DAMAGE_FONT_URI,
                &device,
                &mut encoder,
                &mut staging_buffers,
                positions,
            );
            mesh.with_attribute(&device, &mut encoder, &mut staging_buffers, texcoords);

            // 풀 객체에 메쉬를 등록합니다.
            mesh_pool.insert(DAMAGE_FONT_URI, Arc::new(mesh), None);

            // 커맨드 버퍼를 전송합니다.
            commands.push((staging_buffers, encoder.finish()));

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }
}

impl GameScene for InGameLoadScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle) {
        let workspace = app.asset_manager().get_root_dir().to_path_buf();
        let thread_pool = app.io_threads();
        let device = app.render_device();

        self.load_character_motions(&workspace, thread_pool);
        self.load_character_models(&workspace, thread_pool, device);
        self.load_bullet_models(&workspace, thread_pool, device);
        self.load_stage_models(&workspace, thread_pool, device);
        self.load_capture_zone_model(&workspace, thread_pool, device);
        self.create_ui_layout_texture(&workspace, thread_pool, device);
        self.create_timer_icon_texture(&workspace, thread_pool, device);
        self.create_schale_icon_texture(&workspace, thread_pool, device);
        self.create_character_icon_texture(&workspace, thread_pool, device);
        self.create_player_weapon_icon_texture(&workspace, thread_pool, device);
        self.create_player_weapon_icon_mask_texture(&workspace, thread_pool, device);
        self.create_skybox_texture(&workspace, thread_pool, device);
        self.create_damage_font(&workspace, thread_pool, device);
        self.create_damage_particle_mesh(thread_pool, device);
    }

    fn on_exit(&mut self, _window: Option<&Window>, app: &dyn AppHandle) {
        // 커맨드 버퍼를 수집합니다.
        let mut commands = Vec::new();
        let mut staging_buffers: Vec<wgpu::Buffer> = Vec::new();
        while let Some((mut buffers, command)) = self.commands.pop() {
            commands.push(command);
            staging_buffers.append(&mut buffers);
        }

        app.render_queue().submit(commands);
        drop(staging_buffers);
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

    fn on_received_packet(
        &mut self,
        _packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        None
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            match result {
                Ok(()) => {
                    self.num_remaining_tasks -= 1;
                    log::debug!(
                        "task success (number of tasks remaining:{}, queue:{})",
                        self.num_remaining_tasks,
                        self.task_results.len()
                    );
                }
                Err(_) => {
                    // 다음 게임 장면으로 전환합니다.
                    let i = self.locale as usize;
                    let next_scene = FatalErrorSceneLayer::new(
                        self.locale,
                        ERR_TITLE_TEXTS[i],
                        ERR_MSG_TEXTS[i],
                    );
                    let scene_flow = GameSceneFlow::Push(Box::new(next_scene));
                    let event = AppEvent::SetGameSceneFlow(scene_flow);
                    let event_loop_proxy = app.event_loop_proxy();
                    event_loop_proxy.send_event(event).unwrap();
                }
            };
        }

        // 모든 작업이 끝난 경우 다음 장면으로 전환합니다.
        if self.num_remaining_tasks == 0 {
            let next_scene = Box::new(InGameBuildScene::new(
                self.locale,
                self.user_id,
                self.token,
                self.packet.take(),
                self.stage_layout_data.clone(),
                self.mesh_pool.clone(),
                self.model_pool.clone(),
                self.motion_pool.clone(),
                self.texture_data_pool.clone(),
                self.texture_pool.clone(),
                self.texture_view_pool.clone(),
                self.sampler_pool.clone(),
            ));
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }
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
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());

        // 로드 텍스트
        let text = LOAD_TEXTS[i];
        let font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let load_text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        egui::Area::new(egui::Id::new("Load_Layout"))
            .anchor(egui::Align2::RIGHT_BOTTOM, (-16.0 * scale, -16.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(load_text)
                });
            });
    }
}
