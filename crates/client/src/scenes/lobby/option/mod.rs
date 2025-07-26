//! 게임 설정 모달과 관련된 코드를 작성합니다.
//!

mod common;
mod control;
mod graphics;
mod save;
mod sound;

use std::fmt;

use mod_app::etc::WindowSize;

use crate::{
    config::{Locale, UserConfigIOError, NUM_LOCALE},
    scenes::BASE_WIDTH,
};

pub use self::{common::*, control::*, graphics::*, save::*, sound::*};

/// 타이틀 텍스트의 폰트 크기입니다.
const TITLE_FONT_SIZE: f32 = 32.0;
/// 메뉴 텍스트의 폰트 크기입니다.
const MENU_FONT_SIZE: f32 = 24.0;
/// 메인 텍스트의 폰트 크기입니다.
const MAIN_FONT_SIZE: f32 = 26.0;
/// 서브 텍스트의 폰트 크기입니다.
const SUB_FONT_SIZE: f32 = 22.0;
/// 버튼 텍스트의 폰트 크기입니다.
const BTN_FONT_SIZE: f32 = 24.0;

/// 버튼의 크기입니다.
const BTN_SIZE: egui::Vec2 = egui::vec2(180.0, 45.0);
/// 버튼의 모서리 각도입니다.
const BTN_CORNER: f32 = 5.0;

/// 모달 대화상자의 가로 길이입니다.
const MODAL_WIDTH: f32 = 960.0;
static_assertions::const_assert!(0.0 <= MODAL_WIDTH);
static_assertions::const_assert!(MODAL_WIDTH <= BASE_WIDTH);
/// 모달 대화상자의 세로 길이입니다.
const MODAL_HEIGHT: f32 = 540.0;
static_assertions::const_assert!(0.0 <= MODAL_HEIGHT);
static_assertions::const_assert!(MODAL_HEIGHT <= BASE_WIDTH);

/// 메뉴의 가로 길이입니다.
const MENU_WIDTH: f32 = MODAL_WIDTH * 0.3;
/// 메뉴의 세로 길이입니다.
const MENU_HEIGHT: f32 = 52.0;
static_assertions::const_assert!(0.0 <= MENU_HEIGHT);
static_assertions::const_assert!(MENU_HEIGHT <= CONTENT_HEIGHT);
/// 콘텐츠의 가로 길이입니다.
const CONTENT_WIDTH: f32 = MODAL_WIDTH * 0.7;
static_assertions::const_assert!(
    (MODAL_WIDTH - (MENU_WIDTH + CONTENT_WIDTH)).abs() <= f32::EPSILON
);
/// 콘텐츠의 세로 길이입니다.
const CONTENT_HEIGHT: f32 = 360.0;
static_assertions::const_assert!(0.0 <= CONTENT_HEIGHT);
static_assertions::const_assert!(CONTENT_HEIGHT <= MODAL_HEIGHT);
/// 하위 콘텐츠의 세로 길이입니다.
const SUB_HEIGHT: f32 = 42.0;
static_assertions::const_assert!(0.0 <= SUB_HEIGHT);
static_assertions::const_assert!(SUB_HEIGHT <= CONTENT_HEIGHT);

/// 애플리케이션 표시 언어에 따른 타이틀 텍스트입니다.
const TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["게임 옵션"];
/// 애플리케이션 표시 언어에 따른 오류 메시지 타이틀 텍스트입니다.
const ERR_TITLE_TEXTS: [&'static str; NUM_LOCALE] = ["설정 저장 실패"];

/// 애플리케이션 표시 언어에 따른 일반 설정 텍스트입니다.
const COMMON_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["일반 설정"];
/// 애플리케이션 표시 언어에 따른 그래픽 설정 텍스트입니다.
const GRAPHICS_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["그래픽 설정"];
/// 애플리케이션 표시 언어에 따른 조작키 설정 텍스트입니다.
const CONTROL_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["조작키 설정"];
/// 애플리케이션 표시 언어에 따른 사운드 설정 텍스트입니다.
const SOUND_OPT_TEXTS: [&'static str; NUM_LOCALE] = ["사운드 설정"];

/// 애플리케이션 표시 언어에 따른 `나가기` 버튼 텍스트입니다.
const EXIT_TEXTS: [&'static str; NUM_LOCALE] = ["나가기"];
/// 애플리케이션 표시 언어에 따른 `저장` 버튼 텍스트입니다.
const SAVE_TEXTS: [&'static str; NUM_LOCALE] = ["저장"];

/// 작업 결과 데이터입니다.
#[derive(Debug)]
pub enum TaskResult {
    Success,
    Failed(UserConfigIOError),
}

/// 변경된 설정 데이터입니다.
#[derive(Debug)]
pub enum ChangeOption {
    Common {
        locale: Locale,
    },
    Graphics {
        window_size: WindowSize,
        is_fullscreen: bool,
    },
    Control {},
    Sound {
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
    },
}

impl fmt::Debug for LobbyCommonOptionModalLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LobbyCommonOptionModalLayer))
    }
}

impl fmt::Debug for LobbyControlOptionModalLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LobbyControlOptionModalLayer))
    }
}

impl fmt::Debug for LobbyGraphicsOptionModalLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LobbyGraphicsOptionModalLayer))
    }
}

impl fmt::Debug for LobbySoundOptionModalLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LobbySoundOptionModalLayer))
    }
}

impl fmt::Debug for LobbyOptionSaveGuardLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LobbyOptionOnemoreLayer))
    }
}
