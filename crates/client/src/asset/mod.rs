mod hierarchy;
mod mesh;
mod motion;
mod texture;

use std::io;

use mod_network::components::{NUM_BULLETS, NUM_CHARACTERS, NUM_STAGES};

pub use self::{hierarchy::*, mesh::*, motion::*, texture::*};

/// 사용자 구성 파일의 상대 경로입니다.
pub const USER_CONFIG: &'static str = "user_config";

/// `NotoSans-Regular` 폰트 파일의 Uri입니다.
pub const NOTOSANS_REGULAR: &'static str = "NotoSans_Regular.ttf";
/// `NotoSans-Bold` 폰트 파일의 Uri입니다.
pub const NOTOSANS_BOLD: &'static str = "NotoSans_Bold.ttf";

/// 게임 로고 텍스처의 `Uri`입니다.
pub const GAME_LOGO_URI: &'static str = "ui/Game_Logo.png";
/// 게임 로고 텍스처의 데이터입니다.
pub const GAME_LOGO_DATA: &'static [u8; 26506] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/Game_Logo.png",
));

/// 게임 로그인 타이틀 0번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_0_URI: &'static str = "ui/BG_Login_Title_0.png";
/// 게임 로그인 타이틀 0번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_0_DATA: &'static [u8; 2744719] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_0.png"
));
/// 게임 로그인 타이틀 1번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_1_URI: &'static str = "ui/BG_Login_Title_1.png";
/// 게임 로그인 타이틀 1번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_1_DATA: &'static [u8; 3745175] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_1.png"
));
/// 게임 로그인 타이틀 2번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_2_URI: &'static str = "ui/BG_Login_Title_2.png";
/// 게임 로그인 타이틀 2번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_2_DATA: &'static [u8; 3090166] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_2.png"
));
/// 게임 로그인 타이틀 3번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_3_URI: &'static str = "ui/BG_Login_Title_3.png";
/// 게임 로그인 타이틀 3번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_3_DATA: &'static [u8; 1793237] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_3.png"
));
/// 게임 로그인 타이틀 4번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_4_URI: &'static str = "ui/BG_Login_Title_4.png";
/// 게임 로그인 타이틀 4번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_4_DATA: &'static [u8; 3338929] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_4.png"
));
/// 게임 로그인 타이틀 5번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_5_URI: &'static str = "ui/BG_Login_Title_5.png";
/// 게임 로그인 타이틀 5번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_5_DATA: &'static [u8; 3016216] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_5.png"
));

/// 메인 로비 화면 배경화면 텍스처의 `Uri`입니다.
pub const BG_MAIN_LOBBY_URI: &'static str = "BG_Main_Lobby.png";

/// 데미지 폰트 텍스처의 `Uri`입니다.
pub const DAMAGE_FONT_URI: &'static str = "D_Font_Normal";

/// 인게임 인터페이스 레이아웃 텍스처의 `Uri`입니다.
pub const UI_GAME_LAYOUT_URI: &'static str = "UI_Game_Layout";

/// 인게임 타이머 아이콘 텍스처의 `Uri`입니다.
pub const UI_TIMER_ICON_URI: &'static str = "UI_Timer_Icon";

/// 스카이박스 텍스처의 `Uri`입니다.
pub const SKYBOX_URI: &'static str = "Sky";

/// 캐릭터 모델의 작업 공간입니다.
pub const CHARACTER_WORKSPACES: [&'static str; NUM_CHARACTERS] = [
    "characters/aris_original",
    "characters/momoi_original",
    "characters/midori_original",
    "characters/yuuka_original",
];

/// 캐릭터 모델의 `Uri`입니다.
pub const CHARACTER_URIS: [&'static str; NUM_CHARACTERS] = [
    "aris_original",
    "momoi_original",
    "midori_original",
    "yuuka_original",
];

/// 무기 아이콘 `Uri`입니다.
pub const WEAPON_ICON_URI: &'static str = "Weapon_Icon";

/// 무기 아이콘의 `Uri`입니다.
pub const WEAPON_ICON_URIS: [&'static str; NUM_CHARACTERS] = [
    "Weapon_Icon_Aris",
    "Weapon_Icon_Momoi",
    "Weapon_Icon_Midori",
    "Weapon_Icon_Yuuka",
];

/// 무기 아이콘 마스크의 `Uri`입니다.
pub const WEAPON_ICON_MASK_URI: &'static str = "Weapon_Icon_Mask";

/// 무기 아이콘의 마스크의 `Uri`입니다.
pub const WEAPON_ICON_MASK_URIS: [&'static str; NUM_CHARACTERS] = [
    "Weapon_Icon_Aris_Mask",
    "Weapon_Icon_Momoi_Mask",
    "Weapon_Icon_Midori_Mask",
    "Weapon_Icon_Yuuka_Mask",
];

/// 총알 모델의 작업 공간입니다.
pub const BULLET_WORKSPACE: &'static str = "common";

/// 총알 모델의 `Uri`입니다.
pub const BULLET_URIS: [&'static str; NUM_BULLETS] = ["Bullet_01_Warhead", "Bullet_02_EnergyBoll"];

pub const STAGE_WORKSPACES: [&'static str; NUM_STAGES] = ["stage/city"];

/// 지형 데이터의 `Uri`입니다.
pub const STAGE_URI: &'static str = "map";

/// 점령 지역의 `Uri`입니다.
pub const CAPTURE_ZONE_URI: &'static str = "Capture_Zone";

/// ## Asset Load Error List
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("invalid data")]
    InvalidData,

    /// dds 포맷의 텍스처를 읽는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to read texture for the following reason:{0}")]
    TextureError(#[from] ddsfile::Error),

    /// 에셋 파일을 구문 분석하는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to parse asset for the following reason:{0})")]
    ParsingFailed(#[from] serde_json::Error),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read asset for the following reason:{0})")]
    IOError(#[from] io::Error),
}
