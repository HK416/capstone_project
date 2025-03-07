use std::{
    fs::{self, File},
    io::{self, ErrorKind, Read},
    ops::Deref,
    path::Path,
};

use ahash::{HashMap, HashSet};
use lazy_static::lazy_static;
use mod_app::etc::WindowSize;
use mod_network::components::{GameInput, LoginToken, User, NUM_GAME_INPUTS};
use parking_lot::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use winit::{
    event::MouseButton,
    keyboard::{KeyCode, KeyLocation},
};

lazy_static! {
    /// 전역 변수로 선언된 사용자 구성 설정의 인스턴스입니다.
    static ref USER_CONFIG: Mutex<UserConfig> = Mutex::new(UserConfig::default());
}

/// 애플리케이션 표시 언어 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Locale {
    KOR,
}

impl Default for Locale {
    fn default() -> Self {
        Self::KOR
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserConfigError {
    #[error("could not find user configuration file")]
    NotFound,

    #[error("failed to open user configuration file! (REASON:{0})")]
    IO(#[from] io::Error),

    #[error("invalid user configuration data file")]
    InvalidData,
}

/// 사용자 구성 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserConfig {
    /// 현재 사용자의 정보입니다.  
    /// 사용자의 식별자가 `UserId::NULL`인 경우 클라이언트에서 로그인 하지 않았음을 의미합니다.
    #[serde(skip)]
    pub info: User,
    /// 현재 사용자의 로그인 토큰입니다.  
    /// 로그인 토큰이 `LoginToken::NULL`인 경우 클라이언트에서 로그인 하지 않았음을 의미합니다.
    #[serde(skip)]
    pub token: LoginToken,

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
    input_keyboard_map: HashMap<GameInput, (KeyCode, KeyLocation)>,
    /// 게임 입력과 마우스 매핑 정보를 저장합니다.
    input_mouse_map: HashMap<GameInput, MouseButton>,
    /// 키보드와 게임 입력 매핑 정보를 저장합니다.
    #[serde(skip)]
    keyboard_input_map: HashMap<(KeyCode, KeyLocation), GameInput>,
    /// 마우스와 게임 입력 매핑 정보를 저장합니다.
    #[serde(skip)]
    mouse_input_map: HashMap<MouseButton, GameInput>,
}

impl UserConfig {
    /// 사용자 구성을 가져옵니다.
    pub fn get() -> MutexGuard<'static, UserConfig> {
        USER_CONFIG.lock()
    }

    /// 파일에서 사용자 구성을 로드합니다.
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<MutexGuard<'static, UserConfig>, UserConfigError> {
        // 파일에서 데이터를 읽습니다.
        let mut file = File::open(path).map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                log::warn!("could not find user configuration file.");
                UserConfigError::NotFound
            } else {
                log::warn!("failed to open user configuration file! (REASON:{})", &e);
                UserConfigError::IO(e)
            }
        })?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::warn!("failed to read user configuration file! (REASON:{})", &e);
            UserConfigError::IO(e)
        })?;

        // 사용자 구성 파일을 구문 분석합니다.
        let mut config: Self = serde_json::from_slice(&buf).map_err(|e| {
            log::warn!("failed to parse user configuration file! (REASON:{})", &e);
            UserConfigError::InvalidData
        })?;

        // 로드한 사용자 구성 데이터가 유효하지 않은지 확인합니다.
        if !config.check_pc_input_config() {
            log::warn!("invalid user configuration data file");
            return Err(UserConfigError::InvalidData);
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
    ) -> Result<MutexGuard<'static, UserConfig>, UserConfigError> {
        // 사용자 구성 데이터를 가져옵니다.
        let config = Self::get();
        let data = serde_json::to_vec_pretty(config.deref()).map_err(|e| {
            log::warn!(
                "failed to serialize user configuration data. (REASON:{})",
                &e
            );
            UserConfigError::InvalidData
        })?;

        // 파일을 씁니다.
        fs::write(path, data).map_err(|e| {
            log::warn!("failed to write user configuration data. (REASON:{})", &e);
            UserConfigError::IO(e)
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
        inputs.len() == NUM_GAME_INPUTS
    }
}

impl UserConfig {
    /// 키보드 입력에 대한 게임 입력을 가져옵니다.  
    /// 해당 키보드 입력에 대한 게임 입력이 없는 경우 `None`을 반환합니다.
    pub fn get_keyboard_input(&self, keyboard: &(KeyCode, KeyLocation)) -> Option<GameInput> {
        self.keyboard_input_map.get(keyboard).cloned()
    }

    /// 마우스 입력에 대한 게임 입력을 가져옵니다.  
    /// 해당 마우스 입력에 대한 게임 입력이 없는 경우 `None`을 반환합니다.
    pub fn get_mouse_input(&self, button: &MouseButton) -> Option<GameInput> {
        self.mouse_input_map.get(button).cloned()
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            info: User::EMPTY,
            token: LoginToken::NULL,
            locale: Locale::KOR,
            window_size: WindowSize::MAX,
            is_fullscreen: true,
            flip_horizontal: false,
            flip_vertical: false,
            input_keyboard_map: HashMap::from_iter([
                (GameInput::Left, (KeyCode::KeyA, KeyLocation::Standard)),
                (GameInput::Right, (KeyCode::KeyD, KeyLocation::Standard)),
                (GameInput::Forward, (KeyCode::KeyW, KeyLocation::Standard)),
                (GameInput::Backward, (KeyCode::KeyS, KeyLocation::Standard)),
                (GameInput::Skill, (KeyCode::KeyE, KeyLocation::Standard)),
                (GameInput::ExSkill, (KeyCode::KeyQ, KeyLocation::Standard)),
                (GameInput::Jump, (KeyCode::Space, KeyLocation::Standard)),
                (GameInput::Reload, (KeyCode::KeyR, KeyLocation::Standard)),
                (GameInput::Status, (KeyCode::Tab, KeyLocation::Standard)),
                (
                    GameInput::Emotion,
                    (KeyCode::ShiftLeft, KeyLocation::Standard),
                ),
            ]),
            input_mouse_map: HashMap::from_iter([
                (GameInput::Aiming, MouseButton::Right),
                (GameInput::Attack, MouseButton::Left),
            ]),
            keyboard_input_map: HashMap::from_iter([
                ((KeyCode::KeyA, KeyLocation::Standard), GameInput::Left),
                ((KeyCode::KeyD, KeyLocation::Standard), GameInput::Right),
                ((KeyCode::KeyW, KeyLocation::Standard), GameInput::Forward),
                ((KeyCode::KeyS, KeyLocation::Standard), GameInput::Backward),
                ((KeyCode::KeyE, KeyLocation::Standard), GameInput::Skill),
                ((KeyCode::KeyQ, KeyLocation::Standard), GameInput::ExSkill),
                ((KeyCode::Space, KeyLocation::Standard), GameInput::Jump),
                ((KeyCode::KeyR, KeyLocation::Standard), GameInput::Reload),
                ((KeyCode::Tab, KeyLocation::Standard), GameInput::Status),
                (
                    (KeyCode::ShiftLeft, KeyLocation::Standard),
                    GameInput::Emotion,
                ),
            ]),
            mouse_input_map: HashMap::from_iter([
                (MouseButton::Right, GameInput::Aiming),
                (MouseButton::Left, GameInput::Attack),
            ]),
        }
    }
}
