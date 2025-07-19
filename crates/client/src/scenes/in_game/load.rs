use std::{
    path::Path,
    sync::{Arc, OnceLock},
    time::Instant,
};

use ahash::HashSet;
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{BulletKind, CharacterKind, LoginToken, StageAttributes, StageKind, Team, UserId},
    protocol::{InGameDataInitPacket, RawPacket},
};
use mod_parallelism::collections::Queue;
use mod_render::UiRenderer;
use rand::seq::SliceRandom;
use rayon::ThreadPool;
use rodio::{Sink, Source};
use winit::window::Window;

use crate::{
    asset::{
        AssetError, DecodedSound, MeshPool, ModelPool, MotionPool, SamplerPool, SoundDataPool,
        TextureDataPool, TexturePool, TextureViewPool, BG_SKY_URI, BG_SKY_WORKSPACE,
        BG_SOUND_THEME_23, BG_SOUND_WORKSPACE, BULLET_URIS, BULLET_WORKSPACE,
        CHARACTER_IMG_SMALL_URI, CHARACTER_IMG_URI, CHARACTER_URIS, CHARACTER_WORKSPACES,
        CV_BATTLE_DAMAGE, CV_BATTLE_DEFENSE, CV_BATTLE_MOVE, CV_BATTLE_RETIRE, CV_BATTLE_SHOUT,
        CV_COMMONSKILL, CV_EXSKILL_LEVEL, CV_SOUND_WORKSPACES, CV_TACTIC_IN, EMBLEM_BG_URI,
        HUD_LAYOUT_URI_02, HUD_LAYOUT_URI_03, ICON_WORKSPACE, IMG_FONT_DRAW,
        IMG_FONT_LOSE_SMALL_URI, IMG_FONT_LOSE_URI, IMG_FONT_MISSION_URI, IMG_FONT_MISS_URI,
        IMG_FONT_NUMBER_URI, IMG_FONT_START_URI, IMG_FONT_WIN_SMALL_URI, IMG_FONT_WIN_URI,
        IMG_FONT_WORKSPACE, NOTOSANS_BOLD, PROFILE_ICON_URI, SCHALE_ICON_URI, SFX_COMMON,
        SFX_COMMON_RELOAD, SFX_SKILL, SFX_WORKSPACE, STAGE_URI, STAGE_WORKSPACES, UI_BUTTON_BACK,
        UI_BUTTON_TOUCH, UI_LOADING, UI_NOTICE, UI_PAUSE, UI_TURN_DOWN, UI_TURN_UP,
        WEAPON_ICON_URI,
    },
    component::MaterialDataPool,
    config::{Locale, UserConfig, NUM_LOCALE},
    scenes::{
        FatalErrorSceneLayer, InGameBuildScene, BASE_WIDTH, ERR_CLOSED_MSG_TEXTS, ERR_IO_MSG_TEXTS,
        ERR_NETWORK_TITLE_TEXTS,
    },
};

/// 애플리케이션 표시 언어에 따른 로드 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];
/// 애플리케이션 표시 언어에 따른 오류 타이틀 텍스트
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["오류"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 텍스트
const ERR_MSG_TEXTS: [&'static str; NUM_LOCALE] = ["게임 리소스를 로드하는데 실패했습니다!"];

/// 작업 결과
#[derive(Debug)]
enum TaskResult {
    /// 텍스처
    Textures {
        staging_buffers: Vec<wgpu::Buffer>,
        command: wgpu::CommandBuffer,
    },
    Sound,
    /// 캐릭터 애니메이션
    CharacterMotions,
    /// 스테이지 데이터
    Stage {
        decoded: DecodedSound,
        attributes: StageAttributes,
        staging_buffers: Vec<wgpu::Buffer>,
        command: wgpu::CommandBuffer,
    },
    /// 모델 데이터
    Model {
        staging_buffers: Vec<wgpu::Buffer>,
        command: wgpu::CommandBuffer,
    },
    /// 오류
    Failed(AssetError),
}

/// 게임 월드에 필요한 에셋을 로드하는 장면입니다.
pub struct InGameLoadScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 클라이언트 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,
    /// 시야 조작 민감도입니다.
    control_sensitivity: f32,
    /// 시야 조작의 상하 반전 여부입니다.
    flip_horizontal: bool,
    /// 시야 조작의 좌우 반전 여부입니다.
    flip_vertical: bool,

    /// 초기화 패킷
    packet: Option<InGameDataInitPacket>,

    /// 플레이어 캐릭터 종류
    player_character: CharacterKind,
    /// 플레이어가 속한 팀
    player_team: Team,

    /// 작업 결과를 저장합니다.
    task_results: Arc<Queue<TaskResult>>,
    /// 남은 작업의 수
    num_remaining_tasks: usize,

    /// 로드된 에셋 데이터입니다.
    stage_layout_data: Arc<OnceLock<Arc<StageAttributes>>>,
    /// 스테이징 버퍼 집합
    staging_buffers: Vec<wgpu::Buffer>,

    /// 재질 데이터 풀 객체입니다.
    material_data_pool: MaterialDataPool,
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
    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl InGameLoadScene {
    /// 새로운 `InGameLoadScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        packet: InGameDataInitPacket,
        previous_texture_pool: &TexturePool,
        previous_sound_data_pool: &SoundDataPool,
    ) -> Self {
        // 새로운 텍스처 풀을 생성하고 이전 텍스처 풀에서 필요한 데이터를 취합니다.
        let texture_pool = TexturePool::new();
        const TEXTURE_URIS: [&'static str; 5] = [
            HUD_LAYOUT_URI_02,
            HUD_LAYOUT_URI_03,
            EMBLEM_BG_URI,
            PROFILE_ICON_URI,
            CHARACTER_IMG_URI,
        ];
        for uri in TEXTURE_URIS {
            let texture = previous_texture_pool
                .get(uri)
                .expect(&format!("{} texture must be preloaded!", uri));
            texture_pool.insert(uri, texture);
        }

        let sound_data_pool = SoundDataPool::new();
        const SOUND_URIS: [&'static str; 7] = [
            UI_LOADING,
            UI_NOTICE,
            UI_BUTTON_BACK,
            UI_BUTTON_TOUCH,
            UI_PAUSE,
            UI_TURN_UP,
            UI_TURN_DOWN,
        ];
        for uri in SOUND_URIS {
            let decoded = previous_sound_data_pool
                .get(uri)
                .expect(&format!("{} sound must be preloaded!", uri));
            sound_data_pool.insert(uri, decoded);
        }

        let config = UserConfig::get();
        Self {
            locale,
            uid,
            token,
            background_volume,
            effect_volume,
            voice_volume,
            control_sensitivity: config.control_sensitivity as f32 / 255.0,
            flip_horizontal: config.flip_horizontal,
            flip_vertical: config.flip_vertical,
            player_character: CharacterKind::default(),
            player_team: Team::default(),
            packet: Some(packet),
            task_results: Arc::new(Queue::new()),
            num_remaining_tasks: 0,
            stage_layout_data: Arc::new(OnceLock::new()),
            staging_buffers: Vec::new(),
            material_data_pool: MaterialDataPool::new(),
            mesh_pool: MeshPool::new(),
            model_pool: ModelPool::new(),
            motion_pool: MotionPool::new(),
            texture_pool,
            texture_data_pool: TextureDataPool::new(),
            texture_view_pool: TextureViewPool::new(),
            sampler_pool: SamplerPool::new(),
            sound_data_pool,
        }
    }

    /// Ui 레이아웃 텍스처를 생성합니다.
    fn create_textures(
        &mut self,
        root_dir: &Path,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
    ) {
        let textures = vec![
            ("ui", CHARACTER_IMG_SMALL_URI),
            (BG_SKY_WORKSPACE, BG_SKY_URI),
            (ICON_WORKSPACE, WEAPON_ICON_URI),
            (ICON_WORKSPACE, SCHALE_ICON_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_DRAW),
            (IMG_FONT_WORKSPACE, IMG_FONT_LOSE_SMALL_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_LOSE_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_MISS_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_MISSION_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_NUMBER_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_START_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_WIN_SMALL_URI),
            (IMG_FONT_WORKSPACE, IMG_FONT_WIN_URI),
        ];

        let root_dir = root_dir.to_path_buf();
        let task_results = self.task_results.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        thread_pool.spawn(move || {
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            for (path, uri) in textures {
                let mut workspace = root_dir.clone();
                workspace.push(format!("{}", path));

                // 텍스처를 로드합니다.
                let result = texture_data_pool.get_or_init(
                    &workspace,
                    uri,
                    &device,
                    &mut encoder,
                    &mut staging_buffers,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                );

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 결과를 전송합니다.
            task_results.push(TaskResult::Textures {
                staging_buffers,
                command: encoder.finish(),
            });
        });
        self.num_remaining_tasks += 1;
    }

    /// 사운드 데이터를 로드합니다.
    fn load_sounds(
        &mut self,
        root_dir: &Path,
        thread_pool: &ThreadPool,
        character_kinds: HashSet<CharacterKind>,
    ) {
        // 인게임에서 플레이어 캐릭터 목소리를 로드합니다.
        let i = self.player_character as usize;
        let path = root_dir.to_path_buf();
        let task_results = self.task_results.clone();
        let sound_data_pool = self.sound_data_pool.clone();
        thread_pool.spawn(move || {
            // 인게임 종료시 재생되는 배경음을 로드합니다.
            let mut workspace = path.clone();
            workspace.push(BG_SOUND_WORKSPACE);
            {
                // 사운드 데이터를 로드합니다.
                let result = sound_data_pool.get_or_init(workspace, BG_SOUND_THEME_23);

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            let mut workspace = path.clone();
            workspace.push(CV_SOUND_WORKSPACES[i]);

            // 인게임 진입시 발생하는 목소리를 로드합니다.
            for uri in CV_TACTIC_IN[i] {
                // 사운드 데이터를 로드합니다.
                let result = sound_data_pool.get_or_init(&workspace, uri);

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 인게임 이동시 발생하는 목소리를 로드합니다.
            for uri in CV_BATTLE_MOVE[i] {
                // 사운드 데이터를 로드합니다.
                let result = sound_data_pool.get_or_init(&workspace, uri);

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 인게임 일반공격시 발생하는 목소리를 로드합니다.
            for uri in CV_BATTLE_SHOUT[i] {
                // 사운드 데이터를 로드합니다.
                let result = sound_data_pool.get_or_init(&workspace, uri);

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 인게임 스킬 발동 조건 충족시 발생하는 목소리를 로드합니다.
            {
                // 사운드 데이터를 로드합니다.
                let result = sound_data_pool.get_or_init(&workspace, CV_COMMONSKILL[i]);

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 결과를 전송합니다.
            task_results.push(TaskResult::Sound);
        });
        self.num_remaining_tasks += 1;

        // 인게임 캐릭터 목소리를 로드합니다.
        let path = root_dir.to_path_buf();
        let task_results = self.task_results.clone();
        let sound_data_pool = self.sound_data_pool.clone();
        let characters = character_kinds.clone();
        thread_pool.spawn(move || {
            // 인게임 캐릭터 피격시 발생하는 캐릭터 목소리를 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uris = CV_BATTLE_DAMAGE[i];
                let workspace = CV_SOUND_WORKSPACES[i];
                for uri in uris {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 인게임 캐릭터 스킬 시전시 발생하는 캐릭터 목소리를 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uris = CV_EXSKILL_LEVEL[i];
                let workspace = CV_SOUND_WORKSPACES[i];
                for uri in uris {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 인게임 캐릭터 방어시 발생하는 캐릭터 목소리를 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uri = CV_BATTLE_DEFENSE[i];
                let workspace = CV_SOUND_WORKSPACES[i];
                {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 인게임 캐릭터 행동 불능시 발생하는 캐릭터 목소리를 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uri = CV_BATTLE_RETIRE[i];
                let workspace = CV_SOUND_WORKSPACES[i];
                {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 인게임 캐릭터 스킬 사용시 발생하는 효과음을 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uri = SFX_SKILL[i];
                let workspace = SFX_WORKSPACE;
                {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 인게임 캐릭터 공격시 발생하는 효과음을 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uri = SFX_COMMON[i];
                let workspace = SFX_WORKSPACE;
                {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 인게임 캐릭터 재장전시 발생하는 효과음을 로드합니다.
            for kind in characters.iter().cloned() {
                let i = kind as usize;
                let uri = SFX_COMMON_RELOAD[i];
                let workspace = SFX_WORKSPACE;
                {
                    let mut path = path.clone();
                    path.push(workspace);

                    // 사운드 데이터를 로드합니다.
                    let result = sound_data_pool.get_or_init(path, uri);

                    // 오류를 전송합니다.
                    if let Err(e) = result {
                        task_results.push(TaskResult::Failed(e));
                        return;
                    }
                }
            }

            // 결과를 전송합니다.
            task_results.push(TaskResult::Sound);
        });
        self.num_remaining_tasks += 1;
    }

    /// 캐릭터 애니메이션 데이터를 로드합니다.
    fn load_character_motions(
        &mut self,
        root_dir: &Path,
        thread_pool: &ThreadPool,
        character_kinds: HashSet<CharacterKind>,
    ) {
        let task_results = self.task_results.clone();
        let motion_pool = self.motion_pool.clone();
        let root_dir = root_dir.to_path_buf();
        thread_pool.spawn(move || {
            for character_kind in character_kinds {
                let i = character_kind as usize;
                let uri = CHARACTER_URIS[i];
                let mut workspace = root_dir.clone();
                workspace.push(CHARACTER_WORKSPACES[i]);

                // 캐릭터 애니메이션 데이터를 로드합니다.
                let result = motion_pool.get_or_init(&workspace, uri);

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 결과를 전송합니다.
            task_results.push(TaskResult::CharacterMotions);
        });
        self.num_remaining_tasks += 1;
    }

    /// 캐릭터 모델을 로드합니다.
    fn load_character_models(
        &mut self,
        root_dir: &Path,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
        character_kinds: HashSet<CharacterKind>,
    ) {
        let task_results = self.task_results.clone();
        let material_data_pool = self.material_data_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let root_dir = root_dir.to_path_buf();
        thread_pool.spawn(move || {
            // 임시 버퍼를 생성합니다.
            let mut staging_buffers = Vec::with_capacity(character_kinds.len());
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            for character_kind in character_kinds {
                let i = character_kind as usize;
                let uri = CHARACTER_URIS[i];
                let mut workspace = root_dir.clone();
                workspace.push(CHARACTER_WORKSPACES[i]);

                // 캐릭터 모델을 로드합니다.
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

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 결과를 전송합니다.
            task_results.push(TaskResult::Model {
                staging_buffers,
                command: encoder.finish(),
            });
        });
        self.num_remaining_tasks += 1;
    }

    /// 총알 모델을 로드합니다.
    fn load_bullet_models(
        &mut self,
        root_dir: &Path,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
        bullet_kinds: HashSet<BulletKind>,
    ) {
        let task_results = self.task_results.clone();
        let material_data_pool = self.material_data_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let root_dir = root_dir.to_path_buf();
        thread_pool.spawn(move || {
            // 임시 버퍼를 생성합니다.
            let mut staging_buffers = Vec::with_capacity(bullet_kinds.len());
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            for bullet_kind in bullet_kinds {
                let i = bullet_kind as usize;
                let uri = BULLET_URIS[i];
                let mut workspace = root_dir.clone();
                workspace.push(BULLET_WORKSPACE);

                // 캐릭터 모델을 로드합니다.
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

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 결과를 전송합니다.
            task_results.push(TaskResult::Model {
                staging_buffers,
                command: encoder.finish(),
            });
        });
        self.num_remaining_tasks += 1;
    }

    /// 스테이지에 배치된 모델을 로드합니다.
    fn load_stage_models(
        &mut self,
        root_dir: &Path,
        thread_pool: &ThreadPool,
        device: Arc<wgpu::Device>,
        stage_kind: StageKind,
    ) {
        let task_results = self.task_results.clone();
        let material_data_pool = self.material_data_pool.clone();
        let mesh_pool = self.mesh_pool.clone();
        let model_pool = self.model_pool.clone();
        let texture_data_pool = self.texture_data_pool.clone();
        let texture_pool = self.texture_pool.clone();
        let texture_view_pool = self.texture_view_pool.clone();
        let sampler_pool = self.sampler_pool.clone();
        let sound_data_pool = self.sound_data_pool.clone();
        let root_dir = root_dir.to_path_buf();
        thread_pool.spawn(move || {
            let i = stage_kind as usize;
            let mut workspace = root_dir.clone();
            workspace.push(STAGE_WORKSPACES[i]);

            // 지형 데이터를 로드합니다.
            let mut path = workspace.clone();
            path.push(format!("{}.json", STAGE_URI));

            let result = StageAttributes::load_from_file(path);
            let mut attributes = match result {
                Ok(attributes) => attributes,
                Err(e) => {
                    // 오류를 전송합니다.
                    task_results.push(TaskResult::Failed(e.into()));
                    return;
                }
            };

            // 임시 버퍼를 생성합니다.
            let mut staging_buffers = Vec::new();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

            // 지형 데이터를 구성하는 모델을 로드합니다.
            for uri in attributes.model_list.iter() {
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

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 스테이지의 쉐도우 맵을 로드합니다.
            if let Some(light) = attributes.global_light.as_ref() {
                let uri = &light.static_shadow_map;
                let result = texture_data_pool.get_or_init(
                    &workspace,
                    uri,
                    &device,
                    &mut encoder,
                    &mut staging_buffers,
                    &texture_pool,
                    &texture_view_pool,
                    &sampler_pool,
                );

                // 오류를 전송합니다.
                if let Err(e) = result {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            }

            // 스테이지 배경 음악을 로드합니다.
            let mut sound_list: Vec<_> = attributes.sound_list.drain(..).collect();
            sound_list.shuffle(&mut rand::rng());
            let sound_uri = match sound_list.pop() {
                Some(sound_uri) => sound_uri,
                None => {
                    task_results.push(TaskResult::Failed(AssetError::InvalidData));
                    return;
                }
            };
            let mut workspace = root_dir.clone();
            workspace.push(BG_SOUND_WORKSPACE);
            let result = sound_data_pool.get_or_init(workspace, sound_uri);
            let decoded = match result {
                Ok(decoded) => decoded,
                Err(e) => {
                    task_results.push(TaskResult::Failed(e));
                    return;
                }
            };

            // 결과를 전송합니다.
            task_results.push(TaskResult::Stage {
                decoded,
                attributes,
                staging_buffers,
                command: encoder.finish(),
            });
        });
        self.num_remaining_tasks += 1;
    }
}

impl GameScene for InGameLoadScene {
    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        let packet = self.packet.as_ref().expect("the packet must exist!");

        // 플레이어 캐릭터 종류와 속한 팀 데이터를 찾습니다.
        (self.player_character, self.player_team) = packet
            .players
            .iter()
            .find(|data| data.uid == self.uid)
            .map(|data| (data.character_kind, data.team()))
            .expect("player not found!");

        // 인게임에서 사용되는 캐릭터를 수집합니다.
        let character_kinds: HashSet<CharacterKind> = packet
            .players
            .iter()
            .map(|data| data.character_kind)
            .collect();

        // 인게임에서 사용되는 총알을 수집합니다.
        let bullet_kinds: HashSet<BulletKind> = packet
            .players
            .iter()
            .map(|data| data.character_kind.into())
            .collect();

        // 지형 종류를 가져옵니다.
        let stage_kind = packet.stage_kind;

        let mut root_dir = app.current_dir().to_path_buf();
        root_dir.push("assets");

        let device = app.render_device();
        let io_thread_pool = app.io_threads();
        self.create_textures(&root_dir, io_thread_pool, device.clone());
        self.load_sounds(&root_dir, io_thread_pool, character_kinds.clone());
        self.load_character_motions(&root_dir, io_thread_pool, character_kinds.clone());
        self.load_character_models(&root_dir, io_thread_pool, device.clone(), character_kinds);
        self.load_bullet_models(&root_dir, io_thread_pool, device.clone(), bullet_kinds);
        self.load_stage_models(&root_dir, io_thread_pool, device.clone(), stage_kind);
    }

    fn on_exit(
        &mut self,
        _window: Option<&Window>,
        app: &dyn AppHandle,
        _ui_renderer: &mut UiRenderer,
    ) {
        let device = app.render_device();
        device.poll(wgpu::PollType::Wait).unwrap();
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

    fn on_received_packet(
        &mut self,
        _: Instant,
        _packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        None
    }

    fn on_update(&mut self, _elapsed_time_sec: f32, _window: &Window, app: &dyn AppHandle) {
        // 작업 결과를 확인합니다.
        let queue = app.render_queue();
        if let Some(result) = self.task_results.pop() {
            self.num_remaining_tasks -= 1;

            match result {
                TaskResult::CharacterMotions | TaskResult::Sound => { /* empty */ }
                TaskResult::Model {
                    mut staging_buffers,
                    command,
                } => {
                    self.staging_buffers.append(&mut staging_buffers);
                    queue.submit(Some(command));
                }
                TaskResult::Stage {
                    decoded,
                    attributes,
                    mut staging_buffers,
                    command,
                } => {
                    let source = decoded.as_source().repeat_infinite();
                    let sink = Sink::connect_new(app.audio_mixer());
                    sink.set_volume(self.background_volume as f32 / 255.0);
                    sink.append(source);
                    app.sink_list().push(sink);

                    self.stage_layout_data
                        .set(Arc::new(attributes))
                        .expect("data already exist!");
                    self.staging_buffers.append(&mut staging_buffers);
                    queue.submit(Some(command));
                }
                TaskResult::Textures {
                    mut staging_buffers,
                    command,
                } => {
                    self.staging_buffers.append(&mut staging_buffers);
                    queue.submit(Some(command));
                }
                TaskResult::Failed(_e) => {
                    // 다음 게임 장면으로 전환합니다.
                    let i = self.locale as usize;
                    let next_scene = FatalErrorSceneLayer::new(
                        self.locale,
                        self.background_volume,
                        self.effect_volume,
                        self.voice_volume,
                        ERR_TITLE_TEXTS[i],
                        ERR_MSG_TEXTS[i],
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
            };
        }

        // 모든 작업이 끝난 경우 다음 장면으로 전환합니다.
        if self.num_remaining_tasks == 0 {
            // 다음 게임 장면으로 전환합니다.
            if let Some(packet) = self.packet.take() {
                let next_scene = Box::new(InGameBuildScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    self.control_sensitivity,
                    self.flip_horizontal,
                    self.flip_vertical,
                    self.player_character,
                    self.player_team,
                    packet,
                    self.stage_layout_data.clone(),
                    self.mesh_pool.clone(),
                    self.model_pool.clone(),
                    self.motion_pool.clone(),
                    self.texture_pool.clone(),
                    self.texture_data_pool.clone(),
                    self.texture_view_pool.clone(),
                    self.sampler_pool.clone(),
                    self.sound_data_pool.clone(),
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();

                // 효과음을 재생합니다.
                let decoded = self
                    .sound_data_pool
                    .get(UI_LOADING)
                    .expect("UI_Loading sound must be preloaded!");
                let source = decoded.as_source();
                let sink = Sink::connect_new(app.audio_mixer());
                sink.set_volume(self.effect_volume as f32 / 255.0);
                sink.append(source);
                sink.play();
                sink.detach();
            }
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
                    depth_slice: None,
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
        let font_id = egui::FontId::new(32.0 * scale, family);
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
