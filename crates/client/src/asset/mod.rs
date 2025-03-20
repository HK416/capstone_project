mod hierarchy;
mod motion;
mod stage;

use std::io;

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
pub const GAME_LOGO_URI: &'static str = "assets/ui/Game_Logo.png";
/// 게임 로고 텍스처의 데이터입니다.
pub const GAME_LOGO_DATA: &'static [u8; 26506] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/Game_Logo.png",
));

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
