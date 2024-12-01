use mod_app::etc::WindowSize;
use serde::{Deserialize, Serialize};
use winit::{
    event::MouseButton,
    keyboard::{KeyCode, KeyLocation},
};

/// ## Application User Configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserConfig {
    pub locale: Locale,
    pub resolution: WindowSize,
    pub fullscreen: bool,
    pub keyboard: KeyboardConfig,
    pub mouse: MouseConfig,
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
    pub aimming: MouseButton,
    pub left_right_reversal: bool,
    pub up_down_reversal: bool,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            attack: MouseButton::Left,
            aimming: MouseButton::Right,
            left_right_reversal: false,
            up_down_reversal: false,
        }
    }
}
