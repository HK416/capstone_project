mod hierarchy;
mod motion;
mod stage;

use std::io;

use mod_network::components::NUM_STAGES;

pub use self::{hierarchy::*, motion::*, stage::*};

/// 사용자 구성 파일의 상대 경로입니다.
pub const USER_CONFIG: &'static str = "user_config";

/// 폰트 에셋의 작업 디렉토리 상대 경로입니다.
pub const FONT_WORKSPACE: &'static str = "font/";
/// `NotoSans-Regular` 폰트 파일의 상대 경로입니다.
pub const NOTOSANS_REGULAR: &'static str =
    constcat::concat!(FONT_WORKSPACE, "NotoSans_Regular.ttf");
/// `NotoSans-Bold` 폰트 파일의 상대 경로입니다.
pub const NOTOSANS_BOLD: &'static str = constcat::concat!(FONT_WORKSPACE, "NotoSans_Bold.ttf");

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
pub const BG_MAIN_LOBBY_URI: &'static str = "ui/BG_Main_Lobby.png";

/// 데미지 폰트 텍스처의 `Uri`입니다.
pub const DAMAGE_FONT_URI: &'static str = "font/D_Font_Normal.dds";

/// 스카이박스 텍스처의 `Uri`입니다.
pub const SKYBOX_URI: &'static str = "stage/Sky.dds";

/// 지형 데이터의 작업공간 상대 위치입니다.
pub const STAGE_WORKSPACES: [&'static str; NUM_STAGES] = ["stage/city/"];
/// 지형 데이터의 `Uri`입니다.
pub const STAGE_URIS: [&'static str; NUM_STAGES] = ["stage/city/map.json"];

/// ## Asset Load Error List
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
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
