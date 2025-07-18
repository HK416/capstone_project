use std::{
    fmt,
    fs::OpenOptions,
    io::{self, ErrorKind, Read, Write},
    ops::Deref,
    path::Path,
};

use ahash::{HashMap, HashSet};
use lazy_static::lazy_static;
use mod_app::etc::WindowSize;
use mod_network::components::{InputKind, NUM_INPUT_KINDS};
use serde::{Deserialize, Serialize};
use spin::{Mutex, MutexGuard};
use winit::{
    event::MouseButton,
    keyboard::{KeyCode, KeyLocation},
};

lazy_static! {
    /// 전역 변수로 선언된 사용자 구성 설정의 인스턴스입니다.
    static ref USER_CONFIG: Mutex<UserConfig> = Mutex::new(UserConfig::default());
}

/// 애플리케이션 표시 언어 수
pub const NUM_LOCALE: usize = 1;

/// 애플리케이션 표시 언어 목록입니다.
#[repr(u8)]
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub enum Locale {
    // ENG,
    // JPN,
    #[default]
    KOR,
}

impl Locale {
    /// 주어진 정수로 애플리케이션 표시 언어를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Locale::KOR),
            _ => None,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Locale::KOR => "한국어",
            }
        )
    }
}

/// 사용자 설정 파일 읽기/쓰기 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum UserConfigIOError {
    #[error("could not find user configuration file!")]
    NotFound,

    #[error("failed to open user configuration file! (REASON:{0})")]
    IO(#[from] io::Error),

    #[error("invalid user configuration format!")]
    InvalidFormat(#[from] serde_json::Error),

    #[error("some user configuration settings are missing!")]
    MissionData,
}

/// 사용자 구성 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserConfig {
    /// 애플리케이션 표시 언어입니다.
    pub locale: Locale,
    /// 애플리케이션 창의 크기입니다.
    pub window_size: WindowSize,
    /// 애플리케이션 창의 전체 창 화면 여부입니다.
    pub is_fullscreen: bool,
    /// 좌우 움직임 반전 여부입니다.
    pub flip_horizontal: bool,
    /// 상하 움직임 반전 여부입니다.
    pub flip_vertical: bool,

    /// 게임 입력과 키보드 매핑 정보를 저장합니다.
    input_keyboard_map: HashMap<InputKind, (KeyCode, KeyLocation)>,
    /// 게임 입력과 마우스 매핑 정보를 저장합니다.
    input_mouse_map: HashMap<InputKind, MouseButton>,
    /// 키보드와 게임 입력 매핑 정보를 저장합니다.
    #[serde(skip)]
    keyboard_input_map: HashMap<(KeyCode, KeyLocation), InputKind>,
    /// 마우스와 게임 입력 매핑 정보를 저장합니다.
    #[serde(skip)]
    mouse_input_map: HashMap<MouseButton, InputKind>,

    /// 배경음 음량
    pub background_volume: u8,
    /// 이펙트 음량
    pub effect_volume: u8,
    /// 목소리 음량
    pub voice_volume: u8,
}

impl UserConfig {
    /// 사용자 구성을 가져옵니다.
    pub fn get() -> MutexGuard<'static, UserConfig> {
        USER_CONFIG.lock()
    }

    /// 파일에서 사용자 구성을 로드합니다.
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<MutexGuard<'static, UserConfig>, UserConfigIOError> {
        // 파일에서 엽니다.
        let result = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(path);
        let mut file = match result {
            Ok(file) => file,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    log::warn!("user configuration file not found!");
                    return Err(UserConfigIOError::NotFound);
                } else {
                    log::error!("failed to open user configuration file! (REASON:{})", &e);
                    return Err(UserConfigIOError::IO(e));
                }
            }
        };

        // 파일 내용을 읽습니다.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!("failed to read user configuration file! (REASON:{})", &e);
            UserConfigIOError::IO(e)
        })?;

        // 파일을 닫습니다.
        drop(file);

        // 사용자 구성 파일을 구문 분석합니다.
        let mut config: Self = serde_json::from_slice(&buf).map_err(|e| {
            log::warn!("failed to decode user configuration file! (REASON:{})", &e);
            UserConfigIOError::InvalidFormat(e)
        })?;

        // 로드한 사용자 구성 데이터가 유효하지 않은지 확인합니다.
        if !config.check_pc_input_config() {
            log::warn!("some user configuration settings are missing!");
            return Err(UserConfigIOError::MissionData);
        }

        // 로드한 사용자 구성 데이터를 저장합니다.
        let mut global = Self::get();
        global.locale = config.locale;
        global.window_size = config.window_size;
        global.is_fullscreen = config.is_fullscreen;
        global.input_keyboard_map = config.input_keyboard_map;
        global.input_mouse_map = config.input_mouse_map;
        global.keyboard_input_map = config.keyboard_input_map;
        global.mouse_input_map = config.mouse_input_map;

        Ok(global)
    }

    /// 파일에 사용자 구성을 저장합니다.
    ///
    /// # Warnings
    /// 이 함수는 해당 경로에 파일이 존재하는 경우 기존의 데이터를 제거하고 새로운 데이터로 덮어씁니다.
    ///
    pub fn store_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<MutexGuard<'static, UserConfig>, UserConfigIOError> {
        // 사용자 구성 데이터를 가져옵니다.
        let config = Self::get();

        // 데이터를 JSON 포맷으로 인코딩합니다.
        let data = serde_json::to_vec_pretty(config.deref()).map_err(|e| {
            log::warn!("failed to encode user configuration data. (REASON:{})", &e);
            UserConfigIOError::InvalidFormat(e)
        })?;

        // 파일을 엽니다.
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| {
                log::error!("failed to open user configuration file! (REASON:{})", &e);
                UserConfigIOError::IO(e)
            })?;

        file.write_all(&data).map_err(|e| {
            log::error!("failed to write user configuration data. (REASON:{})", &e);
            UserConfigIOError::IO(e)
        })?;

        Ok(config)
    }

    /// 사용자 구성의 입력 설정이 유효한지 확인합니다.  
    /// 유효하지 않은 경우 `false`를 반환합니다.
    ///
    /// # Note
    /// 전역 변수로 선언된 `UserConfig`가 이 함수를 호출할 경우 데이터가 오염될 수 있습니다.
    ///
    fn check_pc_input_config(&mut self) -> bool {
        // 기존의 데이터를 지웁니다.
        self.keyboard_input_map.clear();
        self.mouse_input_map.clear();

        // 모든 입력이 존재하는지 확인하기 위한 입력 집합입니다.
        let mut inputs = HashSet::default();
        for (&input, &keyboard) in self.input_keyboard_map.iter() {
            inputs.insert(input);
            let duplicate = self.keyboard_input_map.insert(keyboard, input).is_some();

            // 중복된 입력 키가 존재할 경우 유효하지 않은 구성 설정입니다.
            if duplicate {
                return false;
            }
        }

        for (&input, &button) in self.input_mouse_map.iter() {
            inputs.insert(input);
            let duplicate = self.mouse_input_map.insert(button, input).is_some();

            // 중복된 입력 키가 존재할 경우 유효하지 않은 구성 설정입니다.
            if duplicate {
                return false;
            }
        }

        // 게임 입력 집합의 요소가 게임 입력 수와 다른 경우 유효하지 않은 구성 설정입니다.
        inputs.len() == NUM_INPUT_KINDS
    }
}

impl UserConfig {
    /// 키보드 입력에 대한 게임 입력을 가져옵니다.  
    /// 해당 키보드 입력에 대한 게임 입력이 없는 경우 `None`을 반환합니다.
    pub fn get_keyboard_input(&self, keyboard: &(KeyCode, KeyLocation)) -> Option<InputKind> {
        self.keyboard_input_map.get(keyboard).cloned()
    }

    /// 마우스 입력에 대한 게임 입력을 가져옵니다.  
    /// 해당 마우스 입력에 대한 게임 입력이 없는 경우 `None`을 반환합니다.
    pub fn get_mouse_input(&self, button: &MouseButton) -> Option<InputKind> {
        self.mouse_input_map.get(button).cloned()
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            locale: Locale::KOR,
            window_size: WindowSize::MAX,
            is_fullscreen: true,
            flip_horizontal: false,
            flip_vertical: false,
            input_keyboard_map: HashMap::from_iter([
                (InputKind::Left, (KeyCode::KeyA, KeyLocation::Standard)),
                (InputKind::Right, (KeyCode::KeyD, KeyLocation::Standard)),
                (InputKind::Forward, (KeyCode::KeyW, KeyLocation::Standard)),
                (InputKind::Backward, (KeyCode::KeyS, KeyLocation::Standard)),
                (InputKind::Skill, (KeyCode::ShiftLeft, KeyLocation::Left)),
                (InputKind::Jump, (KeyCode::Space, KeyLocation::Standard)),
                (InputKind::Reload, (KeyCode::KeyR, KeyLocation::Standard)),
                (InputKind::Status, (KeyCode::Tab, KeyLocation::Standard)),
                (InputKind::Emotion, (KeyCode::AltLeft, KeyLocation::Left)),
            ]),
            input_mouse_map: HashMap::from_iter([
                (InputKind::Aiming, MouseButton::Right),
                (InputKind::Attack, MouseButton::Left),
            ]),
            keyboard_input_map: HashMap::from_iter([
                ((KeyCode::KeyA, KeyLocation::Standard), InputKind::Left),
                ((KeyCode::KeyD, KeyLocation::Standard), InputKind::Right),
                ((KeyCode::KeyW, KeyLocation::Standard), InputKind::Forward),
                ((KeyCode::KeyS, KeyLocation::Standard), InputKind::Backward),
                ((KeyCode::ShiftLeft, KeyLocation::Left), InputKind::Skill),
                ((KeyCode::Space, KeyLocation::Standard), InputKind::Jump),
                ((KeyCode::KeyR, KeyLocation::Standard), InputKind::Reload),
                ((KeyCode::Tab, KeyLocation::Standard), InputKind::Status),
                ((KeyCode::AltLeft, KeyLocation::Left), InputKind::Emotion),
            ]),
            mouse_input_map: HashMap::from_iter([
                (MouseButton::Right, InputKind::Aiming),
                (MouseButton::Left, InputKind::Attack),
            ]),
            background_volume: 102,
            effect_volume: 255,
            voice_volume: 204,
        }
    }
}
