use ahash::HashMap;
use lazy_static::lazy_static;
use mod_app::etc::WindowSize;
use mod_network::components::GameInputFlags;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use winit::{
    event::MouseButton,
    keyboard::{KeyCode, KeyLocation},
};

// WARNINGS
// 락 획득 순서를 지켜야 합니다.
// 1. Flag_Controller_Map
// 2. Controller_Flag_Map
//
lazy_static! {
    static ref FLAG_KEYBOARD_MAP: Mutex<HashMap<GameInputFlags, (KeyCode, KeyLocation)>> = {
        Mutex::new(HashMap::from_iter([
            (GameInputFlags::Left, (KeyCode::KeyA, KeyLocation::Standard)),
            (
                GameInputFlags::Right,
                (KeyCode::KeyD, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Forward,
                (KeyCode::KeyW, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Backward,
                (KeyCode::KeyS, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Skill,
                (KeyCode::KeyE, KeyLocation::Standard),
            ),
            (
                GameInputFlags::ExSkill,
                (KeyCode::KeyQ, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Reload,
                (KeyCode::KeyR, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Jump,
                (KeyCode::Space, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Status,
                (KeyCode::Tab, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Emotion1,
                (KeyCode::Digit1, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Emotion2,
                (KeyCode::Digit2, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Emotion3,
                (KeyCode::Digit3, KeyLocation::Standard),
            ),
            (
                GameInputFlags::Emotion4,
                (KeyCode::Digit4, KeyLocation::Standard),
            ),
        ]))
    };
    static ref KEYBOARD_FLAG_MAP: Mutex<HashMap<(KeyCode, KeyLocation), GameInputFlags>> = {
        Mutex::new(HashMap::from_iter([
            ((KeyCode::KeyA, KeyLocation::Standard), GameInputFlags::Left),
            (
                (KeyCode::KeyD, KeyLocation::Standard),
                GameInputFlags::Right,
            ),
            (
                (KeyCode::KeyW, KeyLocation::Standard),
                GameInputFlags::Forward,
            ),
            (
                (KeyCode::KeyS, KeyLocation::Standard),
                GameInputFlags::Backward,
            ),
            (
                (KeyCode::KeyE, KeyLocation::Standard),
                GameInputFlags::Skill,
            ),
            (
                (KeyCode::KeyQ, KeyLocation::Standard),
                GameInputFlags::ExSkill,
            ),
            (
                (KeyCode::KeyR, KeyLocation::Standard),
                GameInputFlags::Reload,
            ),
            (
                (KeyCode::Space, KeyLocation::Standard),
                GameInputFlags::Jump,
            ),
            (
                (KeyCode::Tab, KeyLocation::Standard),
                GameInputFlags::Status,
            ),
            (
                (KeyCode::Digit1, KeyLocation::Standard),
                GameInputFlags::Emotion1,
            ),
            (
                (KeyCode::Digit2, KeyLocation::Standard),
                GameInputFlags::Emotion2,
            ),
            (
                (KeyCode::Digit3, KeyLocation::Standard),
                GameInputFlags::Emotion3,
            ),
            (
                (KeyCode::Digit4, KeyLocation::Standard),
                GameInputFlags::Emotion4,
            ),
        ]))
    };
    static ref FLAG_MOUSE_MAP: Mutex<HashMap<GameInputFlags, MouseButton>> = {
        Mutex::new(HashMap::from_iter([
            (GameInputFlags::Attack, MouseButton::Left),
            (GameInputFlags::Aiming, MouseButton::Right),
        ]))
    };
    static ref MOUSE_FLAG_MAP: Mutex<HashMap<MouseButton, GameInputFlags>> = {
        Mutex::new(HashMap::from_iter([
            (MouseButton::Left, GameInputFlags::Attack),
            (MouseButton::Right, GameInputFlags::Aiming),
        ]))
    };
}

/// 주어진 `Keycode`와 `KeyLocation`에 해당하는 `GameInputFlags`를 반환합니다.
pub fn get_input_flag_from_keyboard(keycode: KeyCode, location: KeyLocation) -> GameInputFlags {
    // Warnings: 락의 획득 순서를 반드시 지켜야한다.
    let _guard = FLAG_KEYBOARD_MAP.lock();
    let keyboard_flag_map = KEYBOARD_FLAG_MAP.lock();

    keyboard_flag_map
        .get(&(keycode, location))
        .cloned()
        .unwrap_or_default()
}

/// 주어진 `Keycode`와 `KeyLocation`에 해당하는 `GameInputFlags`를 설정합니다.
pub fn set_keyboard_input_flags(keycode: KeyCode, location: KeyLocation, flags: GameInputFlags) {
    // Warnings: 락의 획득 순서를 반드시 지켜야한다.
    let mut flag_keyboard_map = FLAG_KEYBOARD_MAP.lock();
    let mut keyboard_flag_map = KEYBOARD_FLAG_MAP.lock();

    let result = flag_keyboard_map.insert(flags, (keycode, location));
    if let Some(old) = result {
        keyboard_flag_map.remove(&old);
    }
    keyboard_flag_map.insert((keycode, location), flags);
}

/// 주어진 `MouseButton`에 해당하는 `GameInputFlags`를 반환합니다.
pub fn get_input_flag_from_mouse(button: MouseButton) -> GameInputFlags {
    // Warnings: 락의 획득 순서를 반드시 지켜야한다.
    let _guard = FLAG_MOUSE_MAP.lock();
    let keyboard_flag_map = MOUSE_FLAG_MAP.lock();

    keyboard_flag_map.get(&button).cloned().unwrap_or_default()
}

/// 주어진 `MouseButton`에 해당하는 `GameInputFlags`를 설정합니다.
pub fn set_mouse_input_flags(button: MouseButton, flags: GameInputFlags) {
    // Warnings: 락의 획득 순서를 반드시 지켜야한다.
    let mut flag_mouse_map = FLAG_MOUSE_MAP.lock();
    let mut mouse_flag_map = MOUSE_FLAG_MAP.lock();

    let result = flag_mouse_map.insert(flags, button);
    if let Some(old) = result {
        mouse_flag_map.remove(&old);
    }
    mouse_flag_map.insert(button, flags);
}

#[derive(Debug, thiserror::Error)]
#[error("failed to parse user configuration for the following reason:{0}")]
pub struct InvalidConfig(pub serde_json::Error);

/// ## Application User Configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserConfig {
    pub locale: Option<Locale>,
    pub window_size: WindowSize,
    pub fullscreen: bool,
    pub keyboard: KeyboardConfig,
    pub mouse: MouseConfig,
}

impl UserConfig {
    /// 새로운 사용자 구성을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            locale: None,
            window_size: WindowSize::MAX,
            fullscreen: true,
            keyboard: KeyboardConfig::default(),
            mouse: MouseConfig::default(),
        }
    }
}

/// ## Application Locale
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Locale {
    Korean,
}

/// ## Keyboard Configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct KeyboardConfig {
    pub left: (KeyCode, KeyLocation),
    pub right: (KeyCode, KeyLocation),
    pub forward: (KeyCode, KeyLocation),
    pub backward: (KeyCode, KeyLocation),
    pub skill: (KeyCode, KeyLocation),
    pub ex_skill: (KeyCode, KeyLocation),
    pub reloading: (KeyCode, KeyLocation),
    pub jumping: (KeyCode, KeyLocation),
    pub status: (KeyCode, KeyLocation),
    pub emotion_1: (KeyCode, KeyLocation),
    pub emotion_2: (KeyCode, KeyLocation),
    pub emotion_3: (KeyCode, KeyLocation),
    pub emotion_4: (KeyCode, KeyLocation),
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            left: (KeyCode::KeyA, KeyLocation::Standard),
            right: (KeyCode::KeyD, KeyLocation::Standard),
            forward: (KeyCode::KeyW, KeyLocation::Standard),
            backward: (KeyCode::KeyS, KeyLocation::Standard),
            skill: (KeyCode::KeyE, KeyLocation::Standard),
            ex_skill: (KeyCode::KeyQ, KeyLocation::Standard),
            reloading: (KeyCode::KeyR, KeyLocation::Standard),
            jumping: (KeyCode::Space, KeyLocation::Standard),
            status: (KeyCode::Tab, KeyLocation::Standard),
            emotion_1: (KeyCode::Digit1, KeyLocation::Standard),
            emotion_2: (KeyCode::Digit2, KeyLocation::Standard),
            emotion_3: (KeyCode::Digit3, KeyLocation::Standard),
            emotion_4: (KeyCode::Digit4, KeyLocation::Standard),
        }
    }
}

/// ## Mouse Configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MouseConfig {
    pub attack: MouseButton,
    pub aiming: MouseButton,
    pub left_right_reversal: bool,
    pub up_down_reversal: bool,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            attack: MouseButton::Left,
            aiming: MouseButton::Right,
            left_right_reversal: false,
            up_down_reversal: false,
        }
    }
}
